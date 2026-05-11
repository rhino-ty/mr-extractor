// Design Ref: §4.2 — queue-page Phase 2 본구현
// Plan FR-06 / FR-17 — ffprobe 단일 호출 metadata + ffmpeg 추출 + Channel 진행률 + 30min timeout
// fix #1 — duration_sec를 frontend가 캐시 후 인자 전달 (ffprobe 중복 호출 회피)
// fix III — Phase 2에 PID 등록 hook (cancel 본구현은 Phase 3)

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
pub struct VideoMetadata {
    pub item_id: String,
    /// ffprobe duration. Ok로 반환되는 경우 항상 > 0 (fix C — 0은 Err).
    pub duration_sec: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractProgress {
    pub item_id: String,
    pub percent: u32,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// fetch_video_metadata timeout (Design §4.2 fix #10): 헤더 읽기는 ms 단위, 10s는 안전 마진.
const FETCH_METADATA_TIMEOUT: Duration = Duration::from_secs(10);

/// extract_audio timeout (Plan §2.1 Phase 2 + Risk 2): 30분.
const EXTRACT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Channel emit 빈도 (NFR Performance): 2초마다 진행률 갱신.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

// ─── Sidecar 바이너리 경로 ───────────────────────────────────────────────────

fn ffprobe_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = common::sidecar_dir(app)?;
    let candidates = [
        dir.join("ffprobe-x86_64-pc-windows-msvc.exe"),
        dir.join("ffprobe.exe"),
        dir.join("ffprobe"),
    ];
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| "ffprobe 실행 파일을 찾을 수 없어요.".to_string())
}

fn ffmpeg_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = common::sidecar_dir(app)?;
    let candidates = [
        dir.join("ffmpeg-x86_64-pc-windows-msvc.exe"),
        dir.join("ffmpeg.exe"),
        dir.join("ffmpeg"),
    ];
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| "ffmpeg 실행 파일을 찾을 수 없어요.".to_string())
}

// ─── fetch_video_metadata ────────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_video_metadata(
    app: AppHandle,
    item_id: String,
    path: String,
) -> Result<VideoMetadata, String> {
    common::dev_log(&app, &format!("queue:fetch_video_metadata({}): start", item_id));

    let ffprobe = ffprobe_path(&app)?;
    let output = tokio_timeout(
        FETCH_METADATA_TIMEOUT,
        TokioCommand::new(&ffprobe)
            .args(["-v", "quiet", "-print_format", "json", "-show_format", &path])
            .output(),
    )
    .await
    .map_err(|_| common::translate_error("timeout", ErrorContext::FetchMetadata))?
    .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(common::translate_error(
            &String::from_utf8_lossy(&output.stderr),
            ErrorContext::FetchMetadata,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("ffprobe JSON 파싱 실패: {}", e))?;
    let duration_sec = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|d| d as u32)
        .unwrap_or(0);

    if duration_sec == 0 {
        // SC-15 — corrupt video
        return Err("이 파일을 읽을 수 없어요. 손상된 영상일 수 있어요.".into());
    }

    common::dev_log(&app, &format!("queue:fetch_video_metadata({}): duration={}s", item_id, duration_sec));
    Ok(VideoMetadata { item_id, duration_sec })
}

// ─── extract_audio ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn extract_audio(
    app: AppHandle,
    item_id: String,
    path: String,
    duration_sec: u32,
    on_progress: Channel<ExtractProgress>,
    handle: State<'_, QueueHandle>,
) -> Result<String, String> {
    common::dev_log(&app, &format!("queue:extract_audio({}): start path={}", item_id, path));

    // queue-tmp 디렉토리 생성 (FR-10)
    let tmp_dir = common::queue_tmp_dir(&app)?;
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::VideoExtract))?;
    let tmp_path = tmp_dir.join(format!("{}.wav", item_id));

    let total_sec = duration_sec.max(1);

    // ffmpeg subprocess spawn — pcm_s16le 44100Hz stereo (CLAUDE.md output spec)
    let ffmpeg = ffmpeg_path(&app)?;
    let mut child = TokioCommand::new(&ffmpeg)
        .args([
            "-i", &path,
            "-vn",
            "-acodec", "pcm_s16le",
            "-ar", "44100",
            "-ac", "2",
            "-y",
            &tmp_path.to_string_lossy(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::VideoExtract))?;

    // PID 등록 (Phase 3 cancel 인프라 준비)
    if let Some(pid) = child.id() {
        handle.register(item_id.clone(), pid);
    }

    // stderr 파싱 → time=HH:MM:SS.MS → percent → Channel emit
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "ffmpeg stderr 캡처 실패".to_string())?;

    let item_id_for_parse = item_id.clone();
    let on_progress_clone = on_progress.clone();
    let parse_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut last_emit = Instant::now() - EMIT_INTERVAL;
        // ffmpeg stderr는 줄바꿈 대신 \r로 진행률 갱신할 때가 많음 → 라인 + 부분 버퍼 모두 처리
        loop {
            // BufReader::lines는 \n 단위. \r 단위 진행률 갱신은 라인이 안 끝나서 묶일 수 있음.
            // ffmpeg `-progress pipe:2`로 강제 line-based 진행률을 받는 방법도 있으나,
            // CLAUDE.md sidecar 인자 변경 방지를 위해 stderr 파싱 유지. \r 분리 후 마지막 segment에서
            // time= 추출.
            let Ok(line_opt) = reader.next_line().await else { break };
            let Some(raw_line) = line_opt else { break };
            for segment in raw_line.split('\r') {
                if let Some(t_idx) = segment.find("time=") {
                    let after = &segment[t_idx + 5..];
                    let token = after.split_whitespace().next().unwrap_or("");
                    if let Some(secs) = parse_ffmpeg_time(token) {
                        let pct = ((secs as f64 / total_sec as f64) * 100.0)
                            .clamp(0.0, 99.0) as u32;
                        if last_emit.elapsed() >= EMIT_INTERVAL {
                            let _ = on_progress_clone.send(ExtractProgress {
                                item_id: item_id_for_parse.clone(),
                                percent: pct,
                            });
                            last_emit = Instant::now();
                        }
                    }
                }
            }
        }
    });

    // 30분 timeout으로 child wait
    let wait_result = tokio_timeout(EXTRACT_TIMEOUT, child.wait()).await;
    handle.take(&item_id);

    // parse_task은 stderr EOF 시 자동 종료. abort()로 안전망.
    parse_task.abort();

    let status = match wait_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(common::translate_error(&e.to_string(), ErrorContext::VideoExtract));
        }
        Err(_) => {
            // timeout — child는 kill_on_drop으로 정리됨
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(common::translate_error("timeout", ErrorContext::VideoExtract));
        }
    };

    if !status.success() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(common::translate_error("ffmpeg fail", ErrorContext::VideoExtract));
    }

    // 100% 마지막 emit
    let _ = on_progress.send(ExtractProgress {
        item_id: item_id.clone(),
        percent: 100,
    });

    common::dev_log(&app, &format!("queue:extract_audio({}): done -> {}", item_id, tmp_path.display()));
    Ok(tmp_path.to_string_lossy().to_string())
}

/// ffmpeg `time=HH:MM:SS.MS` 또는 `time=SS.MS` → 초.
fn parse_ffmpeg_time(token: &str) -> Option<u32> {
    let parts: Vec<&str> = token.split(':').collect();
    let total: f64 = match parts.len() {
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].parse().ok()?;
            h * 3600.0 + m * 60.0 + s
        }
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].parse().ok()?;
            m * 60.0 + s
        }
        1 => parts[0].parse().ok()?,
        _ => return None,
    };
    Some(total as u32)
}
