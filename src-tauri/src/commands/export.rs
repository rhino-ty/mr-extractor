// Design Ref: COMMANDS.md export.rs — 믹스 내보내기 + 피치 시프트.
// CLAUDE.md Output Rules: {title}_instrumental.wav / {title}_instrumental_320k.mp3,
// 기본 출력 ~/Desktop/MR Extractor/.
//
// 섹션 (queue-page Option C 패턴):
//   § 1. Payloads       — StemConfig / ExportProgress
//   § 2. export_mix     — Tauri command 본체 (ffmpeg amix + Channel 진행률)
//   § 3. Filter Graph   — volume→amix→pitch 필터 조립
//   § 4. Pitch          — rubberband 우선, 빌드 미포함 시 asetrate+atempo 폴백
//   § 5. Paths          — 출력 파일명 sanitize + 중복 회피
//
// 피치 전략 (COMMANDS.md): 미리듣기 = Tone.js(프론트), 내보내기 = ffmpeg.
// 번들 ffmpeg에 librubberband가 없을 수 있어 `ffmpeg -filters`로 감지 후 폴백.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

use crate::commands::common::{self, ErrorContext};

// ═══════════════════════════════════════════════════════════════════════════════
// § 1. Payloads
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StemConfig {
    pub path: String,
    /// 0.0 ~ 1.0 (프론트 슬라이더 0~100 / 100)
    pub volume: f32,
    pub muted: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub step: String,
    pub percent: u32,
}

// ─── Constants ───────────────────────────────────────────────────────────────

const EMIT_INTERVAL: Duration = Duration::from_millis(500);

// ═══════════════════════════════════════════════════════════════════════════════
// § 2. export_mix — 본체
// ═══════════════════════════════════════════════════════════════════════════════

/// 현재 믹서 상태(볼륨/뮤트)와 키 조절을 반영해 스템을 하나로 믹스다운.
/// 반환: 생성된 출력 파일 절대경로.
#[tauri::command]
pub async fn export_mix(
    app: AppHandle,
    title: String,
    stems: Vec<StemConfig>,
    format: String,
    semitones: i32,
    duration_sec: u32,
    on_progress: Channel<ExportProgress>,
) -> Result<String, String> {
    common::dev_log(
        &app,
        &format!(
            "export:export_mix: title={} format={} semitones={} stems={}",
            title,
            format,
            semitones,
            stems.len()
        ),
    );

    // 소리가 나는 스템만 입력으로 사용
    let audible: Vec<&StemConfig> = stems
        .iter()
        .filter(|s| !s.muted && s.volume > 0.0)
        .collect();
    if audible.is_empty() {
        return Err("내보낼 소리가 없어요. 뮤트를 해제하거나 볼륨을 올려주세요.".to_string());
    }
    for s in &audible {
        if !Path::new(&s.path).exists() {
            return Err("스템 파일을 찾을 수 없어요. 다시 분리해 주세요.".to_string());
        }
    }

    let ffmpeg = ffmpeg_path(&app)?;

    // 출력 경로: ~/Desktop/MR Extractor/{title}_{suffix}.{ext}
    let out_path = resolve_output_path(&app, &title, &stems, &format).await?;

    let _ = on_progress.send(ExportProgress {
        step: "내보내는 중...".into(),
        percent: 0,
    });

    // § 3. 필터 그래프 조립
    let use_rubberband = semitones != 0 && ffmpeg_has_rubberband(&ffmpeg).await;
    let filter = build_filter_graph(audible.len(), &audible, semitones, use_rubberband);
    common::dev_log(&app, &format!("export:filter_complex = {}", filter));

    let mut args: Vec<String> = Vec::new();
    for s in &audible {
        args.push("-i".into());
        args.push(s.path.clone());
    }
    args.push("-filter_complex".into());
    args.push(filter);
    args.push("-map".into());
    args.push("[out]".into());
    match format.as_str() {
        "mp3" => {
            args.extend(["-c:a".into(), "libmp3lame".into(), "-b:a".into(), "320k".into()]);
        }
        "flac" => {
            args.extend(["-c:a".into(), "flac".into()]);
        }
        _ => {
            // wav (기본) — FILE_FORMATS.md: 16bit 44100Hz
            args.extend(["-c:a".into(), "pcm_s16le".into(), "-ar".into(), "44100".into()]);
        }
    }
    args.push("-y".into());
    args.push(out_path.to_string_lossy().to_string());

    let mut child = TokioCommand::new(&ffmpeg)
        .args(&args)
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::VideoExtract))?;

    // 진행률: stderr time= 파싱 (video.rs 패턴). 총 길이는 프론트가 캐시한 duration.
    let total_sec = duration_sec.max(1);
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "내보내기 진행 정보를 읽을 수 없어요.".to_string())?;
    let on_progress_clone = on_progress.clone();
    let parse_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut last_emit = Instant::now() - EMIT_INTERVAL;
        let mut tail: Vec<String> = Vec::new();
        while let Ok(Some(raw_line)) = reader.next_line().await {
            for segment in raw_line.split('\r') {
                if let Some(t_idx) = segment.find("time=") {
                    let token = segment[t_idx + 5..].split_whitespace().next().unwrap_or("");
                    if let Some(secs) = parse_ffmpeg_time(token) {
                        let pct = ((secs as f64 / total_sec as f64) * 100.0).clamp(0.0, 99.0) as u32;
                        if last_emit.elapsed() >= EMIT_INTERVAL {
                            let _ = on_progress_clone.send(ExportProgress {
                                step: "내보내는 중...".into(),
                                percent: pct,
                            });
                            last_emit = Instant::now();
                        }
                    }
                } else if !segment.trim().is_empty() {
                    tail.push(segment.trim().to_string());
                    if tail.len() > 60 {
                        tail.remove(0);
                    }
                }
            }
        }
        tail.join("\n")
    });

    let status = child
        .wait()
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::VideoExtract))?;
    let stderr_buf = parse_task.await.unwrap_or_default();

    if !status.success() {
        let _ = tokio::fs::remove_file(&out_path).await;
        common::dev_log(
            &app,
            &format!(
                "export:export_mix failed: {}",
                stderr_buf.chars().take(2000).collect::<String>()
            ),
        );
        return Err(common::translate_error(&stderr_buf, ErrorContext::VideoExtract));
    }

    let _ = on_progress.send(ExportProgress {
        step: "완료".into(),
        percent: 100,
    });
    common::dev_log(&app, &format!("export:export_mix done -> {}", out_path.display()));
    Ok(out_path.to_string_lossy().to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3. Filter Graph
// ═══════════════════════════════════════════════════════════════════════════════

/// `[0:a]volume=0.85[a0];...;[a0][a1]..amix=inputs=N:normalize=0[m];[m]{pitch}[out]`
fn build_filter_graph(
    n: usize,
    audible: &[&StemConfig],
    semitones: i32,
    use_rubberband: bool,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut mix_inputs = String::new();
    for (i, s) in audible.iter().enumerate() {
        parts.push(format!("[{}:a]volume={:.3}[a{}]", i, s.volume, i));
        mix_inputs.push_str(&format!("[a{}]", i));
    }
    // normalize=0: amix 기본 1/N 감쇠 방지 — 스템 합이 원곡 밸런스
    let mix_out = if semitones == 0 { "[out]" } else { "[m]" };
    parts.push(format!(
        "{}amix=inputs={}:normalize=0{}",
        mix_inputs, n, mix_out
    ));
    if semitones != 0 {
        parts.push(format!("[m]{}[out]", pitch_filter(semitones, use_rubberband)));
    }
    parts.join(";")
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4. Pitch — rubberband 우선 + asetrate/atempo 폴백
// ═══════════════════════════════════════════════════════════════════════════════

/// ratio = 2^(semitones/12). rubberband은 템포 보존 피치 시프트.
/// 폴백: asetrate로 피치+템포 함께 변경 후 atempo로 템포 복원 (±12반음 = atempo 0.5~2.0 범위 내).
fn pitch_filter(semitones: i32, use_rubberband: bool) -> String {
    let ratio = 2f64.powf(semitones as f64 / 12.0);
    if use_rubberband {
        format!("rubberband=pitch={:.6}", ratio)
    } else {
        format!(
            "asetrate=44100*{:.6},aresample=44100,atempo={:.6}",
            ratio,
            1.0 / ratio
        )
    }
}

/// 번들 ffmpeg의 rubberband 필터 포함 여부 (`ffmpeg -filters` 조회).
async fn ffmpeg_has_rubberband(ffmpeg: &Path) -> bool {
    let Ok(output) = TokioCommand::new(ffmpeg)
        .args(["-hide_banner", "-filters"])
        .output()
        .await
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains(" rubberband ")
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5. Paths
// ═══════════════════════════════════════════════════════════════════════════════

fn ffmpeg_path(app: &AppHandle) -> Result<PathBuf, String> {
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
        .ok_or_else(|| "오디오 변환 도구를 찾을 수 없어요.".to_string())
}

/// Windows 파일명 불가 문자 제거 + 공백 정리. 비면 "MR" fallback.
fn sanitize_title(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let capped: String = trimmed.chars().take(80).collect();
    if capped.is_empty() {
        "MR".to_string()
    } else {
        capped
    }
}

/// CLAUDE.md Output Rules 파일명. 보컬이 들리면 _mix, 아니면 _instrumental.
/// 동일 파일 존재 시 " (1)", " (2)" ... 붙여 회피.
async fn resolve_output_path(
    app: &AppHandle,
    title: &str,
    stems: &[StemConfig],
    format: &str,
) -> Result<PathBuf, String> {
    let desktop = app
        .path()
        .desktop_dir()
        .map_err(|_| "바탕화면 경로를 찾을 수 없어요.".to_string())?;
    let out_dir = desktop.join("MR Extractor");
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::VideoExtract))?;

    // vocals 스템이 소리 나는지 → instrumental 여부 (파일명 vocals.wav 기준)
    let vocals_audible = stems.iter().any(|s| {
        !s.muted
            && s.volume > 0.0
            && Path::new(&s.path)
                .file_stem()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("vocals"))
    });
    let suffix = if vocals_audible { "mix" } else { "instrumental" };

    let (ext, bitrate_tag) = match format {
        "mp3" => ("mp3", "_320k"),
        "flac" => ("flac", ""),
        _ => ("wav", ""),
    };
    let base = format!("{}_{}{}", sanitize_title(title), suffix, bitrate_tag);

    let mut candidate = out_dir.join(format!("{}.{}", base, ext));
    let mut idx = 1;
    while candidate.exists() {
        candidate = out_dir.join(format!("{} ({}).{}", base, idx, ext));
        idx += 1;
    }
    Ok(candidate)
}

/// ffmpeg `time=HH:MM:SS.MS` → 초 (video.rs와 동일 파서).
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

// ═══════════════════════════════════════════════════════════════════════════════
// Tests — 필터 그래프 / 피치 / 파일명 (E2E 검증 필터 문자열 기준: 2026-07-14)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn stem(path: &str, volume: f32, muted: bool) -> StemConfig {
        StemConfig { path: path.into(), volume, muted }
    }

    #[test]
    fn filter_graph_without_pitch_maps_to_out() {
        let stems = [stem("a.wav", 1.0, false), stem("b.wav", 0.7, false)];
        let audible: Vec<&StemConfig> = stems.iter().collect();
        let f = build_filter_graph(2, &audible, 0, false);
        assert_eq!(
            f,
            "[0:a]volume=1.000[a0];[1:a]volume=0.700[a1];[a0][a1]amix=inputs=2:normalize=0[out]"
        );
    }

    #[test]
    fn filter_graph_with_rubberband_pitch() {
        let stems = [stem("a.wav", 1.0, false)];
        let audible: Vec<&StemConfig> = stems.iter().collect();
        let f = build_filter_graph(1, &audible, 2, true);
        // +2반음 = 2^(2/12) ≈ 1.122462
        assert!(f.ends_with("[m];[m]rubberband=pitch=1.122462[out]"), "{}", f);
    }

    #[test]
    fn pitch_fallback_atempo_stays_in_range() {
        // ±12반음에서 atempo 인자가 0.5~2.0 범위 내 (ffmpeg 제약)
        let down12 = pitch_filter(-12, false);
        assert!(down12.contains("atempo=2.000000"), "{}", down12);
        let up12 = pitch_filter(12, false);
        assert!(up12.contains("atempo=0.500000"), "{}", up12);
    }

    #[test]
    fn sanitize_title_strips_forbidden_chars() {
        assert_eq!(sanitize_title(r#"소란: 사랑/한? "마음""#), "소란 사랑 한 마음");
        assert_eq!(sanitize_title("   "), "MR");
    }

    #[test]
    fn ffmpeg_time_parses() {
        assert_eq!(parse_ffmpeg_time("00:01:23.45"), Some(83));
        assert_eq!(parse_ffmpeg_time("12.5"), Some(12));
        assert_eq!(parse_ffmpeg_time("bogus"), None);
    }
}
