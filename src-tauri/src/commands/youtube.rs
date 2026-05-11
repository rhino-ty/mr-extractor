// Design Ref: §4.2 — queue-page Phase 3 본구현
// Plan FR-07 / FR-14 / FR-15 / FR-16 / FR-17 — yt-dlp metadata + 다운로드 + Channel 진행률
// fix V — yt-dlp [download] 라인 파싱은 String::find (regex 의존 0)
// fix III — Phase 3에서 cancel_queue_item 본구현 시 PID hook 사용

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout as tokio_timeout;

use crate::commands::common::{self, ErrorContext};
use crate::commands::queue::QueueHandle;

// ─── Data Model (Design §3.1) ────────────────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeMetadata {
    pub item_id: String,
    pub title: String,
    pub duration_sec: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub item_id: String,
    pub step: String,
    pub percent: u32,
}

// ─── Constants ───────────────────────────────────────────────────────────────

const FETCH_METADATA_TIMEOUT: Duration = Duration::from_secs(10);
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

// ─── Sidecar 경로 ────────────────────────────────────────────────────────────

fn ytdlp_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = common::sidecar_dir(app)?;
    let candidates = [
        dir.join("yt-dlp-x86_64-pc-windows-msvc.exe"),
        dir.join("yt-dlp.exe"),
        dir.join("yt-dlp"),
    ];
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| "yt-dlp 실행 파일을 찾을 수 없어요.".to_string())
}

// ─── fetch_youtube_metadata ──────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_youtube_metadata(
    app: AppHandle,
    item_id: String,
    url: String,
) -> Result<YoutubeMetadata, String> {
    common::dev_log(
        &app,
        &format!("queue:fetch_youtube_metadata({}): start", item_id),
    );

    let ytdlp = ytdlp_path(&app)?;
    let output = tokio_timeout(
        FETCH_METADATA_TIMEOUT,
        TokioCommand::new(&ytdlp)
            .args([
                "--skip-download",
                "--no-playlist",
                "--no-warnings",
                "--print",
                "%(title)s",
                "--print",
                "%(duration)s",
                &url,
            ])
            .output(),
    )
    .await
    .map_err(|_| common::translate_error("timeout", ErrorContext::FetchMetadata))?
    .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::FetchMetadata))?;

    if !output.status.success() {
        // 비공개/지역차단/코덱 에러 → friendly 메시지 (SC-14)
        return Err(common::translate_error(
            &String::from_utf8_lossy(&output.stderr),
            ErrorContext::YoutubeDownload,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let title = lines.next().unwrap_or("").trim().to_string();
    let duration_sec = lines
        .next()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);

    common::dev_log(
        &app,
        &format!(
            "queue:fetch_youtube_metadata({}): title={} duration={}s",
            item_id, title, duration_sec
        ),
    );
    Ok(YoutubeMetadata {
        item_id,
        title,
        duration_sec,
    })
}

// ─── download_youtube ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn download_youtube(
    app: AppHandle,
    item_id: String,
    url: String,
    on_progress: Channel<DownloadProgress>,
    handle: State<'_, QueueHandle>,
) -> Result<String, String> {
    common::dev_log(
        &app,
        &format!("queue:download_youtube({}): start url={}", item_id, url),
    );

    // FR-10/16: 출력 디렉토리 강제 — queue_tmp_dir/{id}.%(ext)s
    let tmp_dir = common::queue_tmp_dir(&app)?;
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::YoutubeDownload))?;
    let output_pattern = tmp_dir.join(format!("{}.%(ext)s", item_id));

    let ytdlp = ytdlp_path(&app)?;
    let mut child = TokioCommand::new(&ytdlp)
        .args([
            "--output",
            &output_pattern.to_string_lossy(),
            "--no-playlist", // FR-15: 플레이리스트 첫 영상만
            "--no-mtime",
            "--no-warnings",
            "--newline", // 진행률 라인 단위 buffer flush (fix V — line-based parsing)
            &url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::YoutubeDownload))?;

    // PID hook (FR-18 — cancel_queue_item이 사용)
    if let Some(pid) = child.id() {
        handle.register(item_id.clone(), pid);
    }

    // stdout 라인 단위 파싱 — `[download]   45.2% of 12.3MiB ...`
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "yt-dlp stdout 캡처 실패".to_string())?;

    let item_id_for_parse = item_id.clone();
    let on_progress_clone = on_progress.clone();
    let parse_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        let mut last_emit = Instant::now() - EMIT_INTERVAL;
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(start) = line.find("[download]") {
                let after = &line[start + "[download]".len()..];
                if let Some(pct_end) = after.find('%') {
                    let pct_str = after[..pct_end].trim();
                    if let Ok(percent_f) = pct_str.parse::<f32>() {
                        if last_emit.elapsed() >= EMIT_INTERVAL {
                            let _ = on_progress_clone.send(DownloadProgress {
                                item_id: item_id_for_parse.clone(),
                                step: "다운로드 중...".into(),
                                percent: percent_f as u32,
                            });
                            last_emit = Instant::now();
                        }
                    }
                }
            }
        }
    });

    // stderr 캡처 (실패 시 친절 에러 매핑용)
    let stderr_handle = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        if let Some(pipe) = stderr_handle {
            let mut reader = BufReader::new(pipe).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                buf.push_str(&line);
                buf.push('\n');
            }
        }
        buf
    });

    let status = child
        .wait()
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::YoutubeDownload))?;
    handle.take(&item_id);
    parse_task.abort();
    let stderr_buf = stderr_task.await.unwrap_or_default();

    if !status.success() {
        // 다운로드 실패 → tmp cleanup (best-effort) + friendly error
        cleanup_partial_files(&tmp_dir, &item_id).await;
        return Err(common::translate_error(
            &stderr_buf,
            ErrorContext::YoutubeDownload,
        ));
    }

    // 100% 마지막 emit
    let _ = on_progress.send(DownloadProgress {
        item_id: item_id.clone(),
        step: "다운로드 완료".into(),
        percent: 100,
    });

    // 다운로드 파일 찾기 — yt-dlp가 ext 결정 (.webm / .mp4 / .m4a 등)
    // .part/.ytdl 같은 임시 파일 제외
    let downloaded = find_downloaded_file(&tmp_dir, &item_id).await?;
    common::dev_log(
        &app,
        &format!(
            "queue:download_youtube({}): done -> {}",
            item_id,
            downloaded.display()
        ),
    );
    Ok(downloaded.to_string_lossy().to_string())
}

async fn find_downloaded_file(tmp_dir: &PathBuf, item_id: &str) -> Result<PathBuf, String> {
    let mut entries = tokio::fs::read_dir(tmp_dir)
        .await
        .map_err(|e| e.to_string())?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with(item_id) && !name.ends_with(".part") && !name.ends_with(".ytdl") {
                return Ok(entry.path());
            }
        }
    }
    Err("다운로드 파일을 찾을 수 없어요.".to_string())
}

async fn cleanup_partial_files(tmp_dir: &PathBuf, item_id: &str) {
    let Ok(mut entries) = tokio::fs::read_dir(tmp_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with(item_id) {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }
    }
}
