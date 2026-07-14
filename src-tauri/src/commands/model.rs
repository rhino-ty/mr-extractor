// Design Ref: MODEL_SELECTOR.md — 모델 목록 + on-demand 다운로드.
// setup-page download_model 패턴 재사용 (venv python -c get_model + TORCH_HOME env).
//
// 섹션:
//   § 1. Payloads        — ModelInfo / ModelDownloadProgress
//   § 2. Installed 판정  — htdemucs_ft: checkpoints .th ≥ 4 (setup-page 보장)
//                          기타 모델: 다운로드 성공 시 기록하는 marker 파일
//   § 3. list_models     — 3종 모델 상태 반환
//   § 4. download_model_by_name — get_model('{name}') subprocess + tqdm 진행률

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::commands::common::{self, ErrorContext};

// ═══════════════════════════════════════════════════════════════════════════════
// § 1. Payloads
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub installed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub model: String,
    pub step: String,
    pub percent: u32,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// 지원 모델 화이트리스트 (MODEL_SELECTOR.md 모델 목록).
const SUPPORTED_MODELS: [&str; 3] = ["htdemucs", "htdemucs_ft", "htdemucs_6s"];

/// 다운로드 전 디스크 여유 요구치. 최대 모델(~300MB) + 여유.
/// CONSERVATIVE_ESTIMATE_MB와 동일한 "유일 허용 상수" 패턴 (원격 크기 목록 미확보 시 fallback).
const MODEL_CONSERVATIVE_MB: u64 = 500;

const EMIT_INTERVAL: Duration = Duration::from_millis(500);

// ═══════════════════════════════════════════════════════════════════════════════
// § 2. Installed 판정
// ═══════════════════════════════════════════════════════════════════════════════

/// 다운로드 성공 기록 marker: %APPDATA%/models/{name}.installed
/// demucs 체크포인트 파일명이 해시 기반이라 파일명으로 모델 구분 불가 → marker 방식.
fn model_marker_path(app: &AppHandle, model: &str) -> Result<PathBuf, String> {
    Ok(common::app_data_dir(app)?
        .join("models")
        .join(format!("{}.installed", model)))
}

fn checkpoints_th_count(app: &AppHandle) -> u32 {
    let Ok(dir) = common::torch_checkpoints_dir(app) else {
        return 0;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            e.path().extension().and_then(|x| x.to_str()) == Some("th")
        })
        .count() as u32
}

fn is_installed(app: &AppHandle, model: &str) -> bool {
    // htdemucs_ft(Bag of 4)는 setup-page가 설치 — checkpoints ≥ 4로 판정 (probe_model 일관)
    if model == "htdemucs_ft" && checkpoints_th_count(app) >= 4 {
        return true;
    }
    model_marker_path(app, model)
        .map(|p| p.exists())
        .unwrap_or(false)
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3. list_models
// ═══════════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn list_models(app: AppHandle) -> Result<Vec<ModelInfo>, String> {
    Ok(SUPPORTED_MODELS
        .iter()
        .map(|id| ModelInfo {
            id: (*id).to_string(),
            installed: is_installed(&app, id),
        })
        .collect())
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4. download_model_by_name
// ═══════════════════════════════════════════════════════════════════════════════

/// on-demand 모델 다운로드 (MODEL_SELECTOR.md 플로우 3~5단계).
/// torch.hub가 tqdm으로 stderr에 진행률 출력 → separate.rs와 동일한 `%|` 파싱.
#[tauri::command]
pub async fn download_model_by_name(
    app: AppHandle,
    model: String,
    on_progress: Channel<ModelDownloadProgress>,
) -> Result<(), String> {
    if !SUPPORTED_MODELS.contains(&model.as_str()) {
        return Err("지원하지 않는 모델이에요.".to_string());
    }
    common::dev_log(&app, &format!("model:download({}): start", model));

    // 디스크 체크 (MODEL_SELECTOR.md 플로우 2단계)
    let app_data = common::app_data_dir(&app)?;
    if !common::check_disk_space(&app_data, MODEL_CONSERVATIVE_MB)? {
        return Err("저장 공간이 부족해요. 정리 후 다시 시도해주세요.".to_string());
    }

    let venv_py = common::venv_python_path(&app)?;
    if !venv_py.exists() {
        return Err("음원 분리 엔진에 문제가 생겼어요. 설정 화면으로 돌아가 주세요.".to_string());
    }
    let env_vars = common::python_env_vars(&app)?;

    let py_code = format!(
        "from demucs.pretrained import get_model; get_model('{}'); print('OK')",
        model
    );
    let mut child = TokioCommand::new(&venv_py)
        .args(["-c", &py_code])
        .envs(env_vars)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::Setup))?;

    let _ = on_progress.send(ModelDownloadProgress {
        model: model.clone(),
        step: "다운로드 준비 중...".into(),
        percent: 0,
    });

    // stderr: torch.hub tqdm (`23%|██ ...`) — \r 갱신이라 바이트 chunk 파싱 (separate.rs 패턴)
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "다운로드 진행 정보를 읽을 수 없어요.".to_string())?;
    let model_for_parse = model.clone();
    let progress_clone = on_progress.clone();
    let stderr_task = tokio::spawn(async move {
        let mut pipe = stderr_pipe;
        let mut err_lines: Vec<String> = Vec::new();
        let mut pending: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut last_emit = Instant::now() - EMIT_INTERVAL;
        loop {
            let n = match pipe.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&chunk[..n]);
            let Some(cut) = pending.iter().rposition(|&b| b == b'\r' || b == b'\n') else {
                continue;
            };
            let complete: Vec<u8> = pending.drain(..=cut).collect();
            for segment in String::from_utf8_lossy(&complete).split(['\r', '\n']) {
                let seg = segment.trim();
                if seg.is_empty() {
                    continue;
                }
                if let Some(marker) = seg.find("%|") {
                    let digits: String = seg[..marker]
                        .chars()
                        .rev()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(pct) = digits.chars().rev().collect::<String>().parse::<u32>() {
                        if pct <= 100 && last_emit.elapsed() >= EMIT_INTERVAL {
                            let _ = progress_clone.send(ModelDownloadProgress {
                                model: model_for_parse.clone(),
                                step: "모델 다운로드 중...".into(),
                                percent: pct.min(99),
                            });
                            last_emit = Instant::now();
                        }
                    }
                } else {
                    err_lines.push(seg.to_string());
                    if err_lines.len() > 60 {
                        err_lines.remove(0);
                    }
                }
            }
        }
        err_lines.join("\n")
    });

    let status = child
        .wait()
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::Setup))?;
    let stderr_buf = stderr_task.await.unwrap_or_default();

    if !status.success() {
        common::dev_log(
            &app,
            &format!(
                "model:download({}): failed: {}",
                model,
                stderr_buf.chars().take(1500).collect::<String>()
            ),
        );
        return Err(common::translate_error(&stderr_buf, ErrorContext::Setup));
    }

    // marker 기록 (§ 2)
    let marker = model_marker_path(&app, &model)?;
    if let Some(dir) = marker.parent() {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let _ = tokio::fs::write(&marker, "ok").await;

    let _ = on_progress.send(ModelDownloadProgress {
        model: model.clone(),
        step: "완료".into(),
        percent: 100,
    });
    common::dev_log(&app, &format!("model:download({}): done", model));
    Ok(())
}
