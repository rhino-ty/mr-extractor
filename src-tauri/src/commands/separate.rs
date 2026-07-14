// Design Ref: ProcessPage Plan §2.1 Phase 2 — queue-page Option C 패턴 답습 (단일 파일 + 섹션 주석)
// Plan FR-02/03/04/05/06/08/09/15 — demucs subprocess + TORCH_HOME 전파 + tqdm 진행률 파싱
//   + 결과 경로 재귀 탐색 (모델명 하드코딩 금지, SC-12) + QueueHandle cancel hook + 친절 에러
//
// 섹션:
//   § 1. Payloads          — SeparationProgress / SeparationResult
//   § 2. separate_audio    — Tauri command 본체
//   § 3. Subprocess        — venv python -m demucs + TORCH_HOME 환경변수 전파 (FR-04)
//   § 4. Progress Parsing  — tqdm `%|` 마커 파싱 (regex 의존 0, fix V) + bag-of-N 합산
//   § 5. Result Discovery  — {queue-tmp}/{id}/ 하위 재귀 탐색 (vocals/drums/bass/other.wav)
//   § 6. Cancel Hook       — QueueHandle PID 등록 (queue.rs::cancel_queue_item 재사용, FR-08)
//   § 7. Friendly Errors   — translate_error(ErrorContext::Separation) + 실패 시 out dir 정리
//
// 설계 결정 (Plan §7.2 → Design 확정):
//   - Python 경로: Plan §1.2 표기는 sidecar_dir이지만 demucs는 venv에 설치됨(setup.rs)
//     → common::venv_python_path 사용. TORCH_HOME 등은 common::python_env_vars가 일괄 주입.
//   - cancel 시 {queue-tmp}/{id}/ 삭제는 queue.rs 수정 없이 이 파일의 실패 경로에서 수행
//     (Plan §7.3 "queue.rs 변경 없음" 유지).
//   - timeout 없음 (FR-15, 큰 파일 의도적 허용). 제어는 cancel [✕]로만.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command as TokioCommand;

use crate::commands::common::{self, ErrorContext};
use crate::commands::queue::QueueHandle;

// ═══════════════════════════════════════════════════════════════════════════════
// § 1. Payloads (Plan §7.2 — { item_id, step, percent } 패턴, DownloadProgress 일관)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeparationProgress {
    pub item_id: String,
    /// 한국어 단계 텍스트 (FR-05): "모델 로드 중..." / "음원 분리 중..." / "스템 추출 중..."
    pub step: String,
    pub percent: u32,
}

/// 4 stems 절대경로 (FR-14 — QueueItem.outputs와 필드 일치).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeparationResult {
    pub item_id: String,
    pub vocals: String,
    pub drums: String,
    pub bass: String,
    pub other: String,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Channel emit 빈도 (NFR Performance: < 2초 단위). queue-page EMIT_INTERVAL 일관.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

/// 에러 진단용으로 보존할 stderr 비-진행률 라인 수 (tqdm 스팸 제외 후).
const STDERR_KEEP_LINES: usize = 120;

/// 결과 탐색 최대 깊이 — {out}/{model}/{track}/{stem}.wav 구조까지 커버 (여유 1단계).
const DISCOVER_MAX_DEPTH: u32 = 3;

const STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// 분리 드라이버 (E2E 검증 완료 — 2026-07-14).
/// `python -m demucs` CLI 대신 사용하는 이유: 신버전 torchaudio(≥2.9)의
/// ta.save가 torchcodec을 요구해 CLI 저장 단계가 깨짐 → soundfile로 직접 저장.
/// 출력 구조({out}/{model}/{track}/{stem}.wav)와 stdout "bag of N" 라인,
/// stderr tqdm 포맷은 CLI와 동일하게 유지 — § 4/§ 5 파서 그대로 동작.
/// 입력 로드는 demucs AudioFile(ffmpeg) — python_env_vars의 plain ffmpeg PATH 필요.
const PY_DRIVER: &str = r#"
import sys
from pathlib import Path
import torch
from demucs.apply import BagOfModels, apply_model
from demucs.audio import AudioFile
from demucs.pretrained import get_model
import soundfile as sf

inp, out_dir, model_name = sys.argv[1], sys.argv[2], sys.argv[3]
model = get_model(model_name)
model.eval()
n = len(model.models) if isinstance(model, BagOfModels) else 1
print(f"Selected model is a bag of {n} models.", flush=True)
wav = AudioFile(inp).read(streams=0, samplerate=model.samplerate, channels=model.audio_channels)
ref = wav.mean(0)
wav = (wav - ref.mean()) / (ref.std() + 1e-8)
device = "cuda" if torch.cuda.is_available() else "cpu"
sources = apply_model(model, wav[None], device=device, shifts=1, split=True, overlap=0.25, progress=True)[0]
sources = sources * (ref.std() + 1e-8) + ref.mean()
out = Path(out_dir) / model_name / Path(inp).stem
out.mkdir(parents=True, exist_ok=True)
for name, src in zip(model.sources, sources):
    sf.write(str(out / (name + ".wav")), src.cpu().numpy().T, model.samplerate, subtype="PCM_16")
print("DONE", flush=True)
"#;

// ═══════════════════════════════════════════════════════════════════════════════
// § 2. separate_audio — Tauri command 본체
// ═══════════════════════════════════════════════════════════════════════════════

/// Plan FR-03 — `python -m demucs -n {model} --out {queue-tmp}/{id}/ {file_path}`.
/// out 디렉토리 자체는 별도 생성하지 않음 (demucs가 자동 생성).
#[tauri::command]
pub async fn separate_audio(
    app: AppHandle,
    item_id: String,
    file_path: String,
    model: String,
    on_progress: Channel<SeparationProgress>,
    handle: State<'_, QueueHandle>,
) -> Result<SeparationResult, String> {
    common::dev_log(
        &app,
        &format!("process:separate_audio({}): start file={}", item_id, file_path),
    );

    // 입력 검증 — 큐 항목 wav가 사라진 경우 (사용자 수동 삭제 등) 조기 친절 에러
    if !Path::new(&file_path).exists() {
        return Err("분리할 파일을 찾을 수 없어요. 다시 추가해 주세요.".to_string());
    }

    // § 3. Subprocess 준비 — venv python + TORCH_HOME 환경 (FR-04)
    let venv_py = common::venv_python_path(&app)?;
    if !venv_py.exists() {
        return Err("음원 분리 엔진에 문제가 생겼어요. 설정 화면으로 돌아가 주세요.".to_string());
    }
    let env_vars = common::python_env_vars(&app)?;

    // SC-13 검증용 — 주입 env를 dev_log에 남김 (TORCH_HOME 전파 확인)
    for (k, v) in &env_vars {
        if k != "PATH" {
            common::dev_log(&app, &format!("process:separate_audio({}): env {}={}", item_id, k, v));
        }
    }

    // 입력 wav 위치의 부모 (queue-tmp)는 이미 존재하지만, 오디오 원본을 직접 쓰는
    // 경로(file sourceType)에서는 없을 수 있어 base만 보장. out dir은 미생성 (FR-03).
    let tmp_dir = common::queue_tmp_dir(&app)?;
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::Separation))?;
    let out_dir = tmp_dir.join(&item_id);

    // 드라이버 방식 (PY_DRIVER 주석 참조). 인자는 argv로만 전달 — 경로 인젝션 없음.
    let mut child = TokioCommand::new(&venv_py)
        .args([
            "-c",
            PY_DRIVER,
            &file_path,
            &out_dir.to_string_lossy(),
            &model,
        ])
        .envs(env_vars)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::Separation))?;

    // § 6. Cancel Hook — QueueHandle PID 등록 (FR-08, cancel_queue_item이 tree kill)
    let mut registered = false;
    if let Some(pid) = child.id() {
        handle.register(item_id.clone(), pid);
        registered = true;
    }

    // 모델 로드는 tqdm 출력 전까지 수십 초 걸릴 수 있음 → 즉시 첫 단계 emit (FR-05)
    let _ = on_progress.send(SeparationProgress {
        item_id: item_id.clone(),
        step: "모델 로드 중...".into(),
        percent: 0,
    });

    // § 4-a. stdout 파싱 task — "bag of N models" 라인에서 진행률 분모 추출.
    //   기본 모델은 Bag of 4 → tqdm 바가 4회 반복되므로 합산에 필요.
    //   모델명이 아닌 demucs 자체 출력에서 파생 → 모델명 하드코딩 없음 (SC-12).
    let bag_total = Arc::new(AtomicU32::new(1));
    let stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| "분리 진행 정보를 읽을 수 없어요.".to_string())?;
    let bag_for_stdout = bag_total.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout_pipe).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(n) = parse_bag_count(&line) {
                bag_for_stdout.store(n.max(1), Ordering::Relaxed);
            }
        }
    });

    // § 4-b. stderr 파싱 task — tqdm 진행률 (\r 단위 갱신이라 바이트 chunk로 직접 읽음)
    //   + 비-진행률 라인은 에러 진단 버퍼로 보존 (§ 7에서 translate_error 입력).
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "분리 진행 정보를 읽을 수 없어요.".to_string())?;
    let item_id_for_parse = item_id.clone();
    let on_progress_clone = on_progress.clone();
    let bag_for_stderr = bag_total.clone();
    let stderr_task = tokio::spawn(async move {
        parse_stderr_stream(
            stderr_pipe,
            item_id_for_parse,
            on_progress_clone,
            bag_for_stderr,
        )
        .await
    });

    // FR-15: timeout 없음 — 큰 파일 의도적 허용. 제어는 cancel [✕]만.
    let wait_result = child.wait().await;

    // cancel_queue_item이 먼저 PID를 take했다면 → 사용자 취소로 판정
    let was_cancelled = registered && handle.take(&item_id).is_none();

    stdout_task.abort();
    let stderr_buf = stderr_task.await.unwrap_or_default();

    let status = wait_result
        .map_err(|e| common::translate_error(&e.to_string(), ErrorContext::Separation))?;

    // 성공 판정이 우선 — 완료 직후 cancel이 경합해도 정상 결과를 삭제하지 않음
    if !status.success() {
        // § 7 — FR-08: 취소/실패 시 {queue-tmp}/{id}/ 부분 산출물 정리
        cleanup_out_dir(&app, &out_dir, &item_id).await;
        if was_cancelled {
            return Err("처리가 취소되었어요.".to_string());
        }
        common::dev_log(
            &app,
            &format!(
                "process:separate_audio({}): non-zero exit ({:?}): {}",
                item_id,
                status.code(),
                stderr_buf.chars().take(2000).collect::<String>()
            ),
        );
        return Err(common::translate_error(&stderr_buf, ErrorContext::Separation));
    }

    // § 5. Result Discovery — 스템 추출 단계 (FR-05 3번째 step)
    let _ = on_progress.send(SeparationProgress {
        item_id: item_id.clone(),
        step: "스템 추출 중...".into(),
        percent: 99,
    });

    let mut found: HashMap<String, PathBuf> = HashMap::new();
    discover_stems(&out_dir, 0, &mut found);

    if STEM_NAMES.iter().any(|s| !found.contains_key(*s)) {
        // Plan Risk: glob 실패 시 디렉토리 트리를 dev_log에 dump — 진단용으로 dir은 보존
        common::dev_log(
            &app,
            &format!(
                "process:separate_audio({}): 결과 탐색 실패. tree:\n{}",
                item_id,
                dump_dir_tree(&out_dir)
            ),
        );
        return Err("분리 결과를 찾을 수 없어요. 다시 시도해 주세요.".to_string());
    }

    let result = SeparationResult {
        item_id: item_id.clone(),
        vocals: found["vocals"].to_string_lossy().to_string(),
        drums: found["drums"].to_string_lossy().to_string(),
        bass: found["bass"].to_string_lossy().to_string(),
        other: found["other"].to_string_lossy().to_string(),
    };
    common::dev_log(
        &app,
        &format!("process:separate_audio({}): done -> {}", item_id, out_dir.display()),
    );
    Ok(result)
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4. Progress Parsing — tqdm (regex 의존 0, queue-page fix V 답습)
// ═══════════════════════════════════════════════════════════════════════════════

/// demucs stdout 안내 라인에서 진행률 분모 추출.
/// 예: "Selected model is a bag of 4 models. You will see that many progress bars per track."
fn parse_bag_count(line: &str) -> Option<u32> {
    let idx = line.find("bag of ")?;
    let after = &line[idx + "bag of ".len()..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// tqdm 세그먼트에서 percent 추출. tqdm 표준 포맷 ` 23%|██▎ | ...`의 `%|` 마커를
/// 신뢰 신호로 사용 — 일반 에러 텍스트의 '%'와 오인 방지. 포맷 변경 시 None
/// (indeterminate fallback: step 텍스트만 유지되고 바는 정지).
fn parse_tqdm_percent(segment: &str) -> Option<u32> {
    let marker = segment.find("%|")?;
    let before = &segment[..marker];
    let digits: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let val: u32 = digits.chars().rev().collect::<String>().parse().ok()?;
    (val <= 100).then_some(val)
}

/// stderr 스트림 파싱 본체. tqdm은 \r로 in-place 갱신하므로 line reader 대신
/// 바이트 chunk를 읽어 \r/\n 양쪽에서 세그먼트를 분리한다 (SC-3: < 2초 갱신 보장).
/// 반환값: 비-진행률 라인 누적 (에러 진단 버퍼, 최근 STDERR_KEEP_LINES개).
async fn parse_stderr_stream(
    mut pipe: tokio::process::ChildStderr,
    item_id: String,
    on_progress: Channel<SeparationProgress>,
    bag_total: Arc<AtomicU32>,
) -> String {
    let mut err_lines: Vec<String> = Vec::new();
    let mut pending: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];

    let mut bars_done: u32 = 0; // 완료된 tqdm 바 수 (bag-of-N 합산용)
    let mut last_pct: u32 = 0;
    let mut max_overall: u32 = 0; // 진행률 역행 방지 (바 리셋 순간)
    let mut last_emit = Instant::now() - EMIT_INTERVAL;

    loop {
        let n = match pipe.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        pending.extend_from_slice(&chunk[..n]);

        // 완성된 세그먼트(\r 또는 \n 종결)까지만 처리, 나머지는 보류
        let Some(cut) = pending.iter().rposition(|&b| b == b'\r' || b == b'\n') else {
            continue;
        };
        let complete: Vec<u8> = pending.drain(..=cut).collect();
        for segment in String::from_utf8_lossy(&complete).split(['\r', '\n']) {
            let seg = segment.trim();
            if seg.is_empty() {
                continue;
            }
            match parse_tqdm_percent(seg) {
                Some(pct) => {
                    // 새 바 시작 감지 (percent 급락) → 완료 바 카운트 증가
                    if pct + 5 < last_pct {
                        bars_done += 1;
                    }
                    last_pct = pct;

                    let total = bag_total.load(Ordering::Relaxed).max(1);
                    let overall =
                        ((bars_done.min(total - 1) * 100 + pct) / total).min(99);
                    if overall > max_overall {
                        max_overall = overall;
                    }
                    if last_emit.elapsed() >= EMIT_INTERVAL {
                        let _ = on_progress.send(SeparationProgress {
                            item_id: item_id.clone(),
                            step: "음원 분리 중...".into(),
                            percent: max_overall,
                        });
                        last_emit = Instant::now();
                    }
                }
                None => {
                    // 진행률이 아닌 라인 → 에러 진단 버퍼 (traceback 등)
                    err_lines.push(seg.to_string());
                    if err_lines.len() > STDERR_KEEP_LINES {
                        err_lines.remove(0);
                    }
                }
            }
        }
    }

    // 잔여 버퍼도 진단에 포함
    if !pending.is_empty() {
        let tail = String::from_utf8_lossy(&pending).trim().to_string();
        if !tail.is_empty() && parse_tqdm_percent(&tail).is_none() {
            err_lines.push(tail);
        }
    }
    err_lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5. Result Discovery — 모델명 하드코딩 금지 (CLAUDE.md Do Not / SC-12)
// ═══════════════════════════════════════════════════════════════════════════════

/// {queue-tmp}/{id}/ 하위를 재귀 탐색해 vocals/drums/bass/other.wav 절대경로 수집.
/// demucs 출력 구조({out}/{model}/{track}/{stem}.wav)에 의존하지 않도록
/// 이름 기반 재귀 매칭 — 모델/트랙 디렉토리명 어떤 것이든 동작.
fn discover_stems(dir: &Path, depth: u32, found: &mut HashMap<String, PathBuf>) {
    if depth > DISCOVER_MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            discover_stems(&path, depth + 1, found);
            continue;
        }
        let is_wav = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
        if !is_wav {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if STEM_NAMES.contains(&stem) && !found.contains_key(stem) {
                found.insert(stem.to_string(), path);
            }
        }
    }
}

/// glob 실패 진단용 — out dir 트리를 문자열로 dump (dev_log 전용).
fn dump_dir_tree(dir: &Path) -> String {
    fn walk(dir: &Path, depth: u32, out: &mut String) {
        if depth > DISCOVER_MAX_DEPTH {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            out.push_str(&format!(
                "{}{}\n",
                "  ".repeat(depth as usize),
                entry.file_name().to_string_lossy()
            ));
            if path.is_dir() {
                walk(&path, depth + 1, out);
            }
        }
    }
    let mut out = format!("{}\n", dir.display());
    walk(dir, 1, &mut out);
    out
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 7. Cleanup — 취소/실패 시 부분 산출물 정리 (FR-08)
// ═══════════════════════════════════════════════════════════════════════════════

/// {queue-tmp}/{id}/ best-effort 삭제. 이 함수 자체는 입력 wav({queue-tmp}/{id}.wav)를
/// 건드리지 않지만, 사용자 취소 경로에서는 queue.rs::cancel_queue_item이 {id} prefix
/// 파일 전체를 별도 정리한다 (queue-page cancel 의미론 — 취소 = 임시 산출물 전부 폐기).
async fn cleanup_out_dir(app: &AppHandle, out_dir: &Path, item_id: &str) {
    if tokio::fs::remove_dir_all(out_dir).await.is_ok() {
        common::dev_log(
            app,
            &format!("process:separate_audio({}): out dir 정리 완료", item_id),
        );
    }
}
