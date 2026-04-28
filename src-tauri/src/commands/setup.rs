// Design Ref: §3.1, §4.1, §4.2 — EnvStatus / InstallProgress 타입 + 4개 커맨드.
// Plan SC-7: check_environment가 5개 항목(ffmpeg/yt-dlp/python/demucs/model) 정확 반환.
// Plan SC-10: venv 수동 삭제 후 재실행 시 health check가 detect (FR-09).
// Plan SC-1: 클린 환경에서 100Mbps 5분 이내 QueuePage (Phase 2 install_dependencies).
// Plan SC-8: 기술 용어 노출 금지 — Channel step 문자열은 한국어 별칭만.
// Plan SC-11: 진행률 UI에 실측(current_size_mb) + 예상(estimated_final_mb) 동시 노출.
//
// Phase 1: check_environment 실구현.
// Phase 2: install_dependencies 5 phase 본구현 + write_setup_marker + health rollback.
// Phase 3: check_internet (HEAD) + cancel_install (State<InstallHandle> + tree kill).
#![allow(dead_code)]

use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_shell::ShellExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout as tokio_timeout;

use crate::commands::common;

/// 설치 중인 child process의 PID를 보관. cancel_install이 트리 kill에 사용.
/// Design §6.3: pip이 spawn한 자식까지 잡으려면 Windows taskkill /F /T /PID.
#[derive(Default)]
pub struct InstallHandle(pub Mutex<Option<u32>>);

impl InstallHandle {
    fn set_pid(&self, pid: Option<u32>) {
        if let Ok(mut guard) = self.0.lock() {
            *guard = pid;
        }
    }
    fn take_pid(&self) -> Option<u32> {
        self.0.lock().ok().and_then(|mut g| g.take())
    }
    fn current_pid(&self) -> Option<u32> {
        self.0.lock().ok().and_then(|g| *g)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Data Model (Design §3.1)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvItemStatus {
    Ready,
    Missing,
    Installing,
    Error,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvItem {
    /// 사용자에게 보이는 한국어 라벨 (ref COMMANDS.md 매핑 준수)
    pub label: String,
    pub status: EnvItemStatus,
    pub version: Option<String>,
}

/// `check_environment` 반환. Plan FR-02 + FR-14.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub items: Vec<EnvItem>,
    pub all_ready: bool,
    pub install_size_estimate_mb: u64,
    pub size_probe_succeeded: bool,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    ExtractPython,
    CreateVenv,
    InstallTorch,
    InstallDemucs,
    DownloadModel,
}

/// Channel payload (Design §4.2 install_dependencies).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub step: String,
    pub percent: u32,
    pub phase: InstallPhase,
    pub current_size_mb: Option<u32>,
    pub estimated_final_mb: u32,
}

/// .setup-complete 스키마 v1 (Design §3.1).
#[derive(Serialize, Deserialize)]
pub struct SetupMarker {
    pub version: u32,
    pub installed_at: String,
    pub demucs_version: String,
    pub model_sha256: Option<String>,
}

// ─── Phase 3: Disk Breakdown (Plan FR-11) ────────────────────────────────────

/// 디스크 부족 화면용 항목별 크기 (MB). Plan §3.2 NFR: 모든 값은 probe 결과에서 derive.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskBreakdown {
    /// torch + demucs (음원 분리 엔진)
    pub install: u64,
    /// htdemucs_ft (AI 모델)
    pub model: u64,
    /// pip 설치 중 임시 (총량의 약 15%)
    pub staging: u64,
    /// 권장 여유 = max(500MB, total × 0.2)
    pub headroom: u64,
    /// 합산 (= 사용 시 비교 대상)
    pub total: u64,
}

/// `check_disk_space` 반환값. Plan FR-11 / SC-9.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskCheck {
    pub fits: bool,
    pub free_mb: u64,
    pub breakdown: DiskBreakdown,
    /// SC-12: probe 실패 시 fallback 사용했는지 (UI에 "정확한 크기 확인 불가" 힌트)
    pub size_probe_succeeded: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// check_environment (Phase 1 실구현, Design §4.2)
// ═══════════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn check_environment(app: AppHandle) -> Result<EnvStatus, String> {
    common::dev_log(&app, "check_environment: start");

    // 5개 항목을 Design §4.2 순서 그대로 (COMMANDS.md 라벨 매핑 준수)
    let ffmpeg = probe_sidecar(&app, "ffmpeg", "오디오 변환 도구").await;
    let ytdlp = probe_sidecar(&app, "yt-dlp", "유튜브 다운로더").await;
    let python = probe_python(&app, "실행 환경").await;
    let demucs = probe_demucs(&app, "음원 분리 엔진").await;
    let model = probe_model(&app, "AI 모델").await;

    let items = vec![ffmpeg, ytdlp, python, demucs, model];
    let all_ready = items.iter().all(|i| matches!(i.status, EnvItemStatus::Ready));

    common::dev_log(
        &app,
        &format!(
            "check_environment: result allReady={}, items=[{}]",
            all_ready,
            items
                .iter()
                .map(|i| format!("{}:{:?}", i.label, status_str(&i.status)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );

    let (estimate_mb, probe_ok) = common::estimate_install_size(&app).await;

    Ok(EnvStatus {
        items,
        all_ready,
        install_size_estimate_mb: estimate_mb,
        size_probe_succeeded: probe_ok,
    })
}

fn status_str(s: &EnvItemStatus) -> &'static str {
    match s {
        EnvItemStatus::Ready => "ready",
        EnvItemStatus::Missing => "missing",
        EnvItemStatus::Installing => "installing",
        EnvItemStatus::Error => "error",
    }
}

// ─────────────────────────────────────────────
// probe helpers (private to setup.rs)
// ─────────────────────────────────────────────

/// sidecar binary의 --version 호출로 Ready 여부 판정.
/// Tauri v2 `app.shell().sidecar(name)`는 target-triple 리졸브를 내부에서 처리.
///
/// Plan §5 Risks: Windows SmartScreen/Defender 오탐. 200MB+ ffmpeg 같은 큰 unsigned
/// 바이너리는 첫 실행 시 AV 스캔이 5~30초 걸릴 수 있어 타임아웃을 30s로 둠.
/// 두 번째 이후 실행은 즉시 응답 (OS 캐시).
async fn probe_sidecar(app: &AppHandle, name: &str, label: &str) -> EnvItem {
    let started = std::time::Instant::now();
    let Ok(cmd) = app.shell().sidecar(name) else {
        common::dev_log(
            app,
            &format!("probe_sidecar({}): sidecar resolve 실패 (capabilities/externalBin 누락?)", name),
        );
        return missing(label);
    };
    let run = tokio_timeout(Duration::from_secs(30), cmd.args(["--version"]).output()).await;
    let elapsed_ms = started.elapsed().as_millis();

    match run {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = parse_version(&stdout).or_else(|| parse_version(&stderr));
            common::dev_log(
                app,
                &format!(
                    "probe_sidecar({}): ready in {}ms, version={:?}",
                    name, elapsed_ms, version
                ),
            );
            ready(label, version)
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            common::dev_log(
                app,
                &format!(
                    "probe_sidecar({}): non-zero exit (status={:?}, {}ms): {}",
                    name,
                    output.status.code(),
                    elapsed_ms,
                    stderr.lines().next().unwrap_or("")
                ),
            );
            missing(label)
        }
        Ok(Err(e)) => {
            common::dev_log(
                app,
                &format!("probe_sidecar({}): spawn 실패: {}", name, e),
            );
            missing(label)
        }
        Err(_) => {
            common::dev_log(
                app,
                &format!(
                    "probe_sidecar({}): 30s 타임아웃 — AV 첫 스캔 또는 락업 가능성",
                    name
                ),
            );
            missing(label)
        }
    }
}

/// venv python 또는 embedded python health check.
/// Plan SC-10: venv/Scripts/python.exe 존재 + 실행 가능 여부.
async fn probe_python(app: &AppHandle, label: &str) -> EnvItem {
    // venv가 우선. 없으면 embedded (번들된) python fallback.
    let Ok(venv_py) = common::venv_python_path(app) else {
        common::dev_log(app, "probe_python: venv_python_path resolve 실패");
        return missing(label);
    };

    if !venv_py.exists() {
        common::dev_log(
            app,
            &format!("probe_python: venv python 없음 ({})", venv_py.display()),
        );
        return missing(label);
    }

    let run = tokio_timeout(
        Duration::from_secs(15),
        TokioCommand::new(&venv_py).arg("--version").output(),
    )
    .await;

    match run {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = parse_version(&stdout).or_else(|| parse_version(&stderr));
            common::dev_log(
                app,
                &format!("probe_python: ready, version={:?}", version),
            );
            ready(label, version)
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            common::dev_log(
                app,
                &format!(
                    "probe_python: non-zero exit ({:?}): {}",
                    output.status.code(),
                    stderr.lines().next().unwrap_or("")
                ),
            );
            missing(label)
        }
        Ok(Err(e)) => {
            common::dev_log(app, &format!("probe_python: spawn 실패: {}", e));
            missing(label)
        }
        Err(_) => {
            common::dev_log(
                app,
                &format!("probe_python: 15s 타임아웃 ({})", venv_py.display()),
            );
            missing(label)
        }
    }
}

/// Plan FR-09: `python -m demucs --help` 실행. 파일 존재만으론 부족 (AV 격리 감지).
async fn probe_demucs(app: &AppHandle, label: &str) -> EnvItem {
    let Ok(venv_py) = common::venv_python_path(app) else {
        return missing(label);
    };
    if !venv_py.exists() {
        common::dev_log(app, "probe_demucs: venv python 없음");
        return missing(label);
    }

    let run = tokio_timeout(
        Duration::from_secs(15),
        TokioCommand::new(&venv_py)
            .args(["-m", "demucs", "--help"])
            .output(),
    )
    .await;

    match run {
        Ok(Ok(output)) if output.status.success() => {
            common::dev_log(app, "probe_demucs: ready");
            ready(label, None)
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            common::dev_log(
                app,
                &format!(
                    "probe_demucs: non-zero exit ({:?}): {}",
                    output.status.code(),
                    stderr.lines().take(3).collect::<Vec<_>>().join(" | ")
                ),
            );
            missing(label)
        }
        Ok(Err(e)) => {
            common::dev_log(app, &format!("probe_demucs: spawn 실패: {}", e));
            missing(label)
        }
        Err(_) => {
            common::dev_log(app, "probe_demucs: 15s 타임아웃");
            missing(label)
        }
    }
}

/// Plan FR-09: torch-cache/hub/checkpoints/ 에 htdemucs_ft 관련 .th 파일이 4개 이상 있으면 Ready.
/// Phase 1에선 파일 존재 감지만. 무결성(해시)은 Phase 2 보강.
async fn probe_model(app: &AppHandle, label: &str) -> EnvItem {
    let Ok(dir) = common::torch_checkpoints_dir(app) else {
        common::dev_log(app, "probe_model: torch_checkpoints_dir resolve 실패");
        return missing(label);
    };
    if !dir.exists() {
        common::dev_log(
            app,
            &format!("probe_model: 디렉토리 없음 ({})", dir.display()),
        );
        return missing(label);
    }

    let Ok(entries) = std::fs::read_dir(&dir) else {
        common::dev_log(
            app,
            &format!("probe_model: read_dir 실패 ({})", dir.display()),
        );
        return missing(label);
    };

    let mut count = 0u32;
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if is_model_file(&path) {
            count += 1;
            if let Some(n) = path.file_name().and_then(|n| n.to_str()) {
                names.push(n.to_string());
            }
        }
    }

    common::dev_log(
        app,
        &format!(
            "probe_model: count={} (need >=4), files=[{}], dir={}",
            count,
            names.join(", "),
            dir.display()
        ),
    );

    // htdemucs_ft = Bag of 4
    if count >= 4 {
        ready(label, None)
    } else {
        missing(label)
    }
}

fn is_model_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("th")
}

fn ready(label: &str, version: Option<String>) -> EnvItem {
    EnvItem {
        label: label.into(),
        status: EnvItemStatus::Ready,
        version,
    }
}

fn missing(label: &str) -> EnvItem {
    EnvItem {
        label: label.into(),
        status: EnvItemStatus::Missing,
        version: None,
    }
}

/// 첫 숫자 토큰을 버전으로 추출. "ffmpeg version 6.1.1" → "6.1.1".
fn parse_version(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|s| s.trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.').to_string())
        .filter(|s| !s.is_empty())
}

// ═══════════════════════════════════════════════════════════════════════════════
// install_dependencies (Phase 2 실구현, Design §4.2)
// ═══════════════════════════════════════════════════════════════════════════════

/// Plan FR-05 / FR-13. 5 phase sequential 설치.
///   ① ExtractPython  ( 5%) — embedded python 검증
///   ② CreateVenv     (10%) — `embedded -m venv {venv_dir}`
///   ③ InstallTorch   (15→45%) — `venv_python -m pip install torch`
///   ④ InstallDemucs  (50→55%) — `venv_python -m pip install demucs`
///   ⑤ DownloadModel  (60→100%) — `python -c "from demucs.pretrained import get_model; ..."`
///   완료: write .setup-complete (멱등성 마커)
///
/// 실패 시 health rollback: venv 통째로 삭제 후 Err 반환 (Plan §11.2 step 8).
/// Phase 3: child PID를 InstallHandle에 등록 → cancel_install이 tree kill 가능.
#[tauri::command]
pub async fn install_dependencies(
    app: AppHandle,
    on_progress: Channel<InstallProgress>,
    handle: State<'_, InstallHandle>,
) -> Result<(), String> {
    let result = install_inner(&app, &on_progress, &handle).await;
    // 정리: 실패하든 성공하든 PID 슬롯 비움
    handle.set_pid(None);
    if let Err(ref msg) = result {
        // Plan §11.2: 부분 설치 rollback. venv가 깨지면 통째 삭제하여 다음 시도 깨끗하게.
        let _ = rollback_on_failure(&app).await;
        // 사용자 친화 메시지로 1차 매핑 (frontend errorMessages.ts가 2차 방어)
        return Err(translate_error(msg));
    }
    result
}

async fn install_inner(
    app: &AppHandle,
    on_progress: &Channel<InstallProgress>,
    handle: &State<'_, InstallHandle>,
) -> Result<(), String> {
    common::dev_log(app, "install_dependencies: start");
    let app_data = common::app_data_dir(app)?;
    tokio::fs::create_dir_all(&app_data)
        .await
        .map_err(|e| format!("앱 폴더를 만들 수 없어요. ({})", e))?;

    let (estimate_mb, _probe_ok) = common::estimate_install_size(app).await;
    let estimated_final_mb = estimate_mb as u32;

    // ① ExtractPython
    emit_progress(
        on_progress,
        &app_data,
        InstallPhase::ExtractPython,
        "실행 환경 준비 중...",
        5,
        estimated_final_mb,
    )?;
    extract_embedded_python(app).await?;

    // ② CreateVenv
    emit_progress(
        on_progress,
        &app_data,
        InstallPhase::CreateVenv,
        "실행 환경 준비 중...",
        10,
        estimated_final_mb,
    )?;
    create_venv(app).await?;

    // ③ InstallTorch (15 → 45%)
    emit_progress(
        on_progress,
        &app_data,
        InstallPhase::InstallTorch,
        "음원 분리 엔진 설치 중...",
        15,
        estimated_final_mb,
    )?;
    pip_install(
        app,
        "torch",
        on_progress,
        handle,
        InstallPhase::InstallTorch,
        "음원 분리 엔진 설치 중...",
        15,
        45,
        estimated_final_mb,
    )
    .await?;

    // ④ InstallDemucs (50 → 55%)
    emit_progress(
        on_progress,
        &app_data,
        InstallPhase::InstallDemucs,
        "음원 분리 엔진 설치 중...",
        50,
        estimated_final_mb,
    )?;
    pip_install(
        app,
        "demucs",
        on_progress,
        handle,
        InstallPhase::InstallDemucs,
        "음원 분리 엔진 설치 중...",
        50,
        55,
        estimated_final_mb,
    )
    .await?;

    // ⑤ DownloadModel (60 → 100%)
    emit_progress(
        on_progress,
        &app_data,
        InstallPhase::DownloadModel,
        "AI 모델 다운로드 중...",
        60,
        estimated_final_mb,
    )?;
    download_model(app, on_progress, handle, estimated_final_mb).await?;

    // 완료 마커 + 100% emit
    write_setup_marker(app).await?;
    common::dev_log(app, "install_dependencies: done (100%)");
    emit_progress(
        on_progress,
        &app_data,
        InstallPhase::DownloadModel,
        "준비 완료!",
        100,
        estimated_final_mb,
    )?;

    Ok(())
}

// ─────────────────────────────────────────────
// Phase 2 helpers
// ─────────────────────────────────────────────

/// Channel emit + dir_size 실측. Plan SC-11 (실측/예상 동시 노출).
fn emit_progress(
    channel: &Channel<InstallProgress>,
    app_data: &Path,
    phase: InstallPhase,
    step: &str,
    percent: u32,
    estimated_final_mb: u32,
) -> Result<(), String> {
    let current_mb = (common::dir_size(app_data) / 1024 / 1024) as u32;
    channel
        .send(InstallProgress {
            step: step.into(),
            percent,
            phase,
            current_size_mb: Some(current_mb),
            estimated_final_mb,
        })
        .map_err(|e| format!("진행률 전송 실패: {}", e))
}

/// Embedded python 검증. download-binaries.js가 빌드 시점에 풀어둠.
/// Phase 2에서는 path resolve + 실행 가능 여부 확인.
/// Analysis G-I2 fix: dev/prod fallback은 common::embedded_python_path가 이미 처리함.
async fn extract_embedded_python(app: &AppHandle) -> Result<(), String> {
    // 존재 검증은 embedded_python_path 내부에서 수행. exists() 통과 못 하면 Err 반환.
    let _py = common::embedded_python_path(app)?;
    Ok(())
}

/// `embedded_python -m venv {venv_dir}`. Plan §10.3 결정 B.
async fn create_venv(app: &AppHandle) -> Result<(), String> {
    let venv_dir = common::venv_dir(app)?;
    let py = common::embedded_python_path(app)?;

    // 이미 정상 venv면 skip (멱등성). venv/Scripts/python.exe 실행 가능 여부 검증.
    let venv_py = common::venv_python_path(app)?;
    if venv_py.exists() {
        let ok = tokio_timeout(
            Duration::from_secs(5),
            TokioCommand::new(&venv_py).arg("--version").output(),
        )
        .await;
        if let Ok(Ok(o)) = ok {
            if o.status.success() {
                return Ok(());
            }
        }
        // 깨진 venv → 삭제 후 재생성
        let _ = tokio::fs::remove_dir_all(&venv_dir).await;
    }

    let output = tokio_timeout(
        Duration::from_secs(60),
        TokioCommand::new(&py)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .output(),
    )
    .await
    .map_err(|_| "실행 환경 생성이 시간 초과되었어요.".to_string())?
    .map_err(|e| format!("실행 환경 생성 실패: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("실행 환경 생성에 실패했어요. {}", stderr.trim()));
    }
    Ok(())
}

/// venv python으로 pip install. stdout/stderr 라인 단위 스트리밍 → 진행률 보간.
/// pip은 일관된 progress 포맷이 없어 같은 phase 내 percent_min~percent_max 사이를
/// 시간 기반 ease 보간 + dir_size 실측 emit. Plan §10.4 + Design §4.2.
async fn pip_install(
    app: &AppHandle,
    pkg: &str,
    on_progress: &Channel<InstallProgress>,
    handle: &State<'_, InstallHandle>,
    phase: InstallPhase,
    step: &str,
    percent_min: u32,
    percent_max: u32,
    estimated_final_mb: u32,
) -> Result<(), String> {
    common::dev_log(app, &format!("pip_install({}): start", pkg));
    let venv_py = common::venv_python_path(app)?;
    let env = common::python_env_vars(app)?;
    let app_data = common::app_data_dir(app)?;

    let mut cmd = TokioCommand::new(&venv_py);
    cmd.args(["-m", "pip", "install", "--no-warn-script-location", "--disable-pip-version-check", pkg])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("패키지 설치 시작 실패: {}", e))?;

    // Phase 3: cancel_install이 트리 kill 할 수 있도록 PID 등록.
    if let Some(pid) = child.id() {
        handle.set_pid(Some(pid));
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // 진행률 보간: 30초마다 한 칸씩 (대략 InstallTorch 2분 → 4 step)
    let span = (percent_max.saturating_sub(percent_min)).max(1);
    let started = SystemTime::now();
    let app_data_clone = app_data.clone();
    let channel_clone = on_progress.clone();
    let step_str = step.to_string();
    let phase_clone = phase;
    let ticker = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let elapsed = started.elapsed().unwrap_or(Duration::ZERO).as_secs();
            // ease curve: 60s 후 60% 진척 가정, 이후 점근. percent_min + span × (1 - exp(-t/60))
            let t = elapsed as f64;
            let ratio = 1.0 - (-t / 60.0).exp();
            let pct = percent_min + ((span as f64) * ratio).round() as u32;
            let pct = pct.min(percent_max.saturating_sub(1));
            let current_mb = (common::dir_size(&app_data_clone) / 1024 / 1024) as u32;
            let _ = channel_clone.send(InstallProgress {
                step: step_str.clone(),
                percent: pct,
                phase: phase_clone,
                current_size_mb: Some(current_mb),
                estimated_final_mb,
            });
        }
    });

    // stdout/stderr drain (Plan SC-8: 사용자 노출 X, 디버그용 로그만)
    let drain_out = tokio::spawn(async move {
        if let Some(out) = stdout {
            let mut reader = BufReader::new(out).lines();
            while let Ok(Some(_)) = reader.next_line().await {
                // pip 출력은 사용자 UI에 노출하지 않음 (기술 용어 금지)
            }
        }
    });
    let drain_err = tokio::spawn(async move {
        let mut tail = String::new();
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                tail.push_str(&line);
                tail.push('\n');
                // tail 8KB만 유지
                if tail.len() > 8192 {
                    let cut = tail.len() - 8192;
                    tail.drain(..cut);
                }
            }
        }
        tail
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("패키지 설치 대기 실패: {}", e))?;

    ticker.abort();
    let _ = drain_out.await;
    let stderr_tail = drain_err.await.unwrap_or_default();

    if !status.success() {
        let snippet = stderr_tail.trim();
        common::dev_log(
            app,
            &format!(
                "pip_install({}): exit non-zero. stderr tail: {}",
                pkg,
                snippet.lines().rev().take(5).collect::<Vec<_>>().join(" | ")
            ),
        );
        return Err(format!(
            "패키지 설치 실패 ({}). 상세: {}",
            pkg,
            if snippet.is_empty() { "exit non-zero" } else { snippet }
        ));
    }

    // 완료 직후 실측 emit
    let current_mb = (common::dir_size(&app_data) / 1024 / 1024) as u32;
    common::dev_log(
        app,
        &format!("pip_install({}): done, dir_size={}MB", pkg, current_mb),
    );
    on_progress
        .send(InstallProgress {
            step: step.into(),
            percent: percent_max,
            phase,
            current_size_mb: Some(current_mb),
            estimated_final_mb,
        })
        .map_err(|e| format!("진행률 전송 실패: {}", e))?;

    Ok(())
}

/// htdemucs_ft 모델 자동 다운로드. demucs.pretrained.get_model 호출.
/// TORCH_HOME 환경변수로 `%APPDATA%/torch-cache/`에 저장. SC-13 경계 (기본 모델만).
async fn download_model(
    app: &AppHandle,
    on_progress: &Channel<InstallProgress>,
    handle: &State<'_, InstallHandle>,
    estimated_final_mb: u32,
) -> Result<(), String> {
    let venv_py = common::venv_python_path(app)?;
    let env = common::python_env_vars(app)?;
    let app_data = common::app_data_dir(app)?;

    let py_code =
        "from demucs.pretrained import get_model; m = get_model('htdemucs_ft'); print('OK')";

    let mut cmd = TokioCommand::new(&venv_py);
    cmd.args(["-c", py_code])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("AI 모델 다운로드 시작 실패: {}", e))?;
    if let Some(pid) = child.id() {
        handle.set_pid(Some(pid));
    }
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // 60→100% 보간. 100Mbps 기준 ~2분 (Bag-of-4 ~1.3GB).
    let percent_min = 60u32;
    let percent_max = 99u32;
    let span = percent_max - percent_min;
    let started = SystemTime::now();
    let app_data_clone = app_data.clone();
    let channel_clone = on_progress.clone();
    let ticker = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let t = started.elapsed().unwrap_or(Duration::ZERO).as_secs() as f64;
            // 120초 가정 ease
            let ratio = 1.0 - (-t / 120.0).exp();
            let pct = percent_min + ((span as f64) * ratio).round() as u32;
            let pct = pct.min(percent_max);
            let current_mb = (common::dir_size(&app_data_clone) / 1024 / 1024) as u32;
            let _ = channel_clone.send(InstallProgress {
                step: "AI 모델 다운로드 중...".into(),
                percent: pct,
                phase: InstallPhase::DownloadModel,
                current_size_mb: Some(current_mb),
                estimated_final_mb,
            });
        }
    });

    let drain_out = tokio::spawn(async move {
        if let Some(out) = stdout {
            let mut reader = BufReader::new(out).lines();
            while let Ok(Some(_)) = reader.next_line().await {}
        }
    });
    let drain_err = tokio::spawn(async move {
        let mut tail = String::new();
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                tail.push_str(&line);
                tail.push('\n');
                if tail.len() > 8192 {
                    let cut = tail.len() - 8192;
                    tail.drain(..cut);
                }
            }
        }
        tail
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("AI 모델 다운로드 대기 실패: {}", e))?;

    ticker.abort();
    let _ = drain_out.await;
    let stderr_tail = drain_err.await.unwrap_or_default();

    if !status.success() {
        let snippet = stderr_tail.trim();
        common::dev_log(
            app,
            &format!(
                "download_model: exit non-zero. stderr tail: {}",
                snippet.lines().rev().take(5).collect::<Vec<_>>().join(" | ")
            ),
        );
        return Err(format!(
            "AI 모델 다운로드 실패. {}",
            if snippet.is_empty() { "" } else { snippet }
        ));
    }

    common::dev_log(
        app,
        &format!(
            "download_model: exit 0. final dir_size={}MB. checking torch-cache files...",
            common::dir_size(&app_data) / 1024 / 1024
        ),
    );

    // demucs 실제 다운로드 위치 진단: torch-cache/hub/checkpoints/ 내용 로그
    if let Ok(checkpoints) = common::torch_checkpoints_dir(app) {
        if checkpoints.exists() {
            if let Ok(entries) = std::fs::read_dir(&checkpoints) {
                let names: Vec<String> = entries
                    .flatten()
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect();
                common::dev_log(
                    app,
                    &format!(
                        "download_model: checkpoints/ files=[{}], path={}",
                        names.join(", "),
                        checkpoints.display()
                    ),
                );
            }
        } else {
            common::dev_log(
                app,
                &format!(
                    "download_model: checkpoints dir 없음 ({}). TORCH_HOME 미적용 가능성!",
                    checkpoints.display()
                ),
            );
        }
    }

    // SC-13: 4개 .th 파일 검증. demucs 캐시 디렉터리는 환경마다 다를 수 있으므로
    // 카운트 미달이어도 즉시 실패 처리는 안 함 (Phase 3 health check가 다시 검증).
    Ok(())
}

/// .setup-complete JSON 마커. 멱등성 보조용 (진실의 원천은 health check).
async fn write_setup_marker(app: &AppHandle) -> Result<(), String> {
    let path = common::setup_marker_path(app)?;
    let installed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            // RFC3339-ish (간단 ISO) — chrono 의존 없이.
            format!("{}", secs)
        })
        .unwrap_or_else(|_| "0".into());

    let marker = SetupMarker {
        version: 1,
        installed_at,
        demucs_version: "unknown".into(),
        model_sha256: None,
    };
    let json = serde_json::to_string_pretty(&marker)
        .map_err(|e| format!("마커 직렬화 실패: {}", e))?;
    tokio::fs::write(&path, json)
        .await
        .map_err(|e| format!("마커 저장 실패: {}", e))?;
    Ok(())
}

/// venv 통째 삭제. 부분 설치 상태에서 다음 시도가 깨끗하게 시작되도록.
async fn rollback_on_failure(app: &AppHandle) -> Result<(), String> {
    let venv = common::venv_dir(app)?;
    if venv.exists() {
        let _ = tokio::fs::remove_dir_all(&venv).await;
    }
    let marker = common::setup_marker_path(app)?;
    if marker.exists() {
        let _ = tokio::fs::remove_file(&marker).await;
    }
    Ok(())
}

/// Plan SC-8 + Design §6.2: Rust raw 에러 → 한국어 친절 메시지 1차 매핑.
/// Phase 3에서 errorMessages.ts로 분리될 예정. 우선 Rust 측에서 패턴 매칭.
fn translate_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("no space left") {
        return "저장 공간이 부족해요. 정리 후 다시 시도해주세요.".into();
    }
    if lower.contains("connectionerror") || lower.contains("timeout") || lower.contains("connect") {
        return "인터넷 연결이 끊겼어요. 다시 시도해주세요.".into();
    }
    if lower.contains("access") && lower.contains("deni") || lower.contains("permission") {
        return "파일 쓰기 권한이 없어요. 관리자 권한으로 실행하거나 백신 예외에 추가해주세요.".into();
    }
    if lower.contains("antivirus") || lower.contains("defender") {
        return "백신 프로그램이 앱 파일을 차단하고 있어요. 예외 처리 후 다시 시도해주세요.".into();
    }
    // 기본값: 원본을 그대로 반환 (UI의 [상세] 토글에서만 노출)
    raw.to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 3 commands (실구현)
// ═══════════════════════════════════════════════════════════════════════════════

/// Plan FR-07 / SC-3 / SC-4. HEAD pypi.org → 실패 시 fbaipublicfiles fallback.
/// 둘 다 실패 → Ok(false). 호출 자체 실패만 Err.
#[tauri::command]
pub async fn check_internet() -> Result<bool, String> {
    let timeout = common::PROBE_TIMEOUT;
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("네트워크 클라이언트 초기화 실패: {}", e))?;

    // Plan §8.2: 1차 pypi.org
    if client.head("https://pypi.org").send().await.is_ok() {
        return Ok(true);
    }
    // Plan §8.2: 2차 fbaipublicfiles
    if client
        .head("https://dl.fbaipublicfiles.com")
        .send()
        .await
        .is_ok()
    {
        return Ok(true);
    }
    Ok(false)
}

/// Plan FR-11 / SC-9. 설치 시작 전 디스크 여유 공간 확인 + breakdown UI 데이터 제공.
/// 임계값: 예상 설치량 × 1.5배 (동적). breakdown은 SizeEstimate에서 derive.
#[tauri::command]
pub async fn check_disk_space(app: AppHandle) -> Result<DiskCheck, String> {
    let estimate = common::estimate_install_size_breakdown().await;

    // Plan §3.2 NFR: 모든 값은 probe 또는 비례 derive. 그 외 하드코딩 금지.
    // staging은 install + model의 ~20% (pip cache temp 추정), headroom = max(500, total × 0.2).
    let install_mb = estimate.install_mb;
    let model_mb = estimate.model_mb;
    let core_mb = install_mb + model_mb;
    let staging_mb = core_mb / 5; // ~20% of core (pip cache + 임시)
    let pre_total_mb = core_mb + staging_mb;
    // FR-11: 임계값 = estimate × 1.5 = pre_total × ~1.2 (헤드룸 부분)
    // headroom = max(500, pre_total × 0.2). 500MB는 디스크 최소 안전선 가이드 (Plan §10.1 / common 별도 상수 없음).
    let min_headroom_mb: u64 = 500;
    let proportional = pre_total_mb / 5;
    let headroom_mb = if proportional > min_headroom_mb {
        proportional
    } else {
        min_headroom_mb
    };
    let total_mb = pre_total_mb + headroom_mb;

    let app_data = common::app_data_dir(&app)?;
    let free_mb = common::available_space_mb(&app_data);

    Ok(DiskCheck {
        fits: free_mb >= total_mb,
        free_mb,
        breakdown: DiskBreakdown {
            install: install_mb,
            model: model_mb,
            staging: staging_mb,
            headroom: headroom_mb,
            total: total_mb,
        },
        size_probe_succeeded: estimate.probe_ok,
    })
}

/// Plan §6.3 / Design §6.3 / SC-4. 진행 중인 install_dependencies child 트리 kill.
/// Windows: `taskkill /F /T /PID {pid}` — pip이 spawn한 자식까지 잡음.
#[tauri::command]
pub async fn cancel_install(handle: State<'_, InstallHandle>) -> Result<(), String> {
    let Some(pid) = handle.take_pid() else {
        return Ok(()); // 진행 중인 작업 없음 — 멱등성
    };
    kill_process_tree(pid)
}

#[cfg(windows)]
fn kill_process_tree(pid: u32) -> Result<(), String> {
    let output = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("프로세스 종료 실패: {}", e))?;
    if !output.status.success() {
        // 이미 종료된 PID는 에러 — 무시 가능.
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("not found") && !stderr.contains("찾을 수 없") {
            return Err(format!("taskkill 실패: {}", stderr.trim()));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn kill_process_tree(pid: u32) -> Result<(), String> {
    // Plan §2.2 Out of Scope: macOS/Linux는 v2 백로그. fallback으로 단일 kill.
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Dev Logging API — debug 빌드 전용. release 빌드엔 컴파일 자체에서 제외됨.
// ═══════════════════════════════════════════════════════════════════════════════

/// `%APPDATA%/com.rhinoty.mr-extractor/setup.log` 내용을 반환.
/// 파일 없으면 빈 문자열.
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn read_setup_log(app: AppHandle) -> Result<String, String> {
    let path = common::setup_log_path(&app)?;
    if !path.exists() {
        return Ok(String::new());
    }
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("로그 파일 읽기 실패: {}", e))
}

/// 로그 파일 비우기 (재시도 시 깨끗한 상태로 시작).
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn clear_setup_log(app: AppHandle) -> Result<(), String> {
    let path = common::setup_log_path(&app)?;
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("로그 파일 삭제 실패: {}", e))?;
    }
    common::dev_log(&app, "─── log cleared ───");
    Ok(())
}

/// 로그 파일 경로 반환 (외부 에디터로 열기 안내용).
#[cfg(debug_assertions)]
#[tauri::command]
pub fn setup_log_path(app: AppHandle) -> Result<String, String> {
    Ok(common::setup_log_path(&app)?.to_string_lossy().to_string())
}
