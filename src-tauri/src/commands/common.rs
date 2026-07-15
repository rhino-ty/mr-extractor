// Design Ref: §2.2, §9, §11.3 — Foundation layer. 후속 피처 (separate/video/youtube/export) 가 공유.
// Plan SC-13 경계: setup-page는 기본 모델만 보장, 추가 모델은 common::probe_url_size 재사용하는 ModelSelector 책임.
//
// 섹션:
//   § 1. Paths            — sidecar / app-data / venv / torch-cache / setup-marker / queue-tmp
//   § 2. Probing          — pypi wheel size / HEAD content-length
//   § 3. Disk             — dir_size / check_disk_space / estimate_install_size
//   § 4. Subprocess       — python subprocess 환경변수 주입
//   § 5. Error Translation — raw 에러 → 한국어 친절 메시지 (queue-page Phase 2 추가)
//   § 6. Process Helpers   — kill_process_tree (queue-page Phase 2 추가, setup.rs에서 이전)
//
// 의존 역전 (Design §9.2): common은 AppHandle + std/tokio/reqwest/sysinfo 만 의존. 다른 commands::* import 금지.
//
// Phase 1에서 setup.rs가 미사용하는 API는 §11.3 Interface Contract에 명시된 Phase 2/3 export.
// 후속 피처(ModelSelector, separate.rs)가 재사용할 foundation이라 dead_code 허용.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

// ═══════════════════════════════════════════════════════════════════════════════
// § 0. Dev Logging — debug 빌드에서만 활성화
// ═══════════════════════════════════════════════════════════════════════════════
//
// 진단용 파일 로그 + eprintln stderr. release 빌드에선 no-op.
// 로그 위치: %APPDATA%/com.rhinoty.mr-extractor/setup.log
// 사용: common::dev_log(app, "메시지");
//
// 운영 노출: read_setup_log Tauri command + 에러 화면에서 [📋 로그 보기] 버튼.

#[cfg(debug_assertions)]
pub fn dev_log(app: &AppHandle, msg: &str) {
    eprintln!("[setup] {}", msg);
    let _ = (|| -> std::io::Result<()> {
        use std::io::Write;
        let dir = app_data_dir(app).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("setup.log");
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        writeln!(f, "[{}] {}", now, msg)
    })();
}

#[cfg(not(debug_assertions))]
pub fn dev_log(_app: &AppHandle, _msg: &str) {}

/// setup.log 경로 (없을 수도 있음).
pub fn setup_log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("setup.log"))
}

// ─── 상수 (Plan NFR: 크기 하드코딩 금지, CONSERVATIVE_ESTIMATE_MB 1개만 허용) ──────

/// Probing 실패 시 fallback 예상치. 허용되는 유일한 크기 상수.
/// torch(~2GB) + demucs(~small) + htdemucs_ft Bag-of-4(~1.3GB) ≈ 3.3GB.
pub const CONSERVATIVE_ESTIMATE_MB: u64 = 3500;

/// HTTP probing 타임아웃. Plan §8.2 Convention.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// Bundle identifier (tauri.conf.json과 일치).
pub const APP_ID: &str = "com.rhinoty.mr-extractor";

/// htdemucs_ft Bag-of-4 체크포인트 URL 목록.
/// Phase 1에서는 비어 있음 — model probing은 Phase 2에서 보강.
/// Plan SC-12: 비어있음 → probing 실패로 간주 → CONSERVATIVE fallback 동작 확인 경로.
pub const HTDEMUCS_FT_MODEL_URLS: &[&str] = &[];

// ═══════════════════════════════════════════════════════════════════════════════
// § 1. Paths
// ═══════════════════════════════════════════════════════════════════════════════

/// %APPDATA%/com.rhinoty.mr-extractor/ — 런타임 사용자 쓰기 가능 영역.
/// Plan §10.3 Decision B: venv/모델/마커 파일 위치.
pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// %APPDATA%/com.rhinoty.mr-extractor/venv/
pub fn venv_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("venv"))
}

/// Plan SC-10: venv/Scripts/python.exe — health check 대상 실행 파일.
pub fn venv_python_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = venv_dir(app)?;
    #[cfg(windows)]
    let p = base.join("Scripts").join("python.exe");
    #[cfg(not(windows))]
    let p = base.join("bin").join("python");
    Ok(p)
}

/// %APPDATA%/com.rhinoty.mr-extractor/torch-cache/ — TORCH_HOME 리다이렉트 (FR-12).
pub fn torch_cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("torch-cache"))
}

/// htdemucs_ft 모델 파일 저장 디렉토리.
/// demucs: {TORCH_HOME}/hub/checkpoints/{model}-*.th
pub fn torch_checkpoints_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(torch_cache_path(app)?.join("hub").join("checkpoints"))
}

/// .setup-complete 마커 파일 (멱등성 힌트. 진실의 원천은 health check — Design §7).
pub fn setup_marker_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(".setup-complete"))
}

/// queue-page Phase 2 신규 — `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/`.
/// Plan FR-10 / Design §3 — 다운로드/추출 임시 파일 위치. uninstall 시 삭제는 v1.2 app-lifecycle 이관.
pub fn queue_tmp_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("queue-tmp"))
}

/// 번들된 Embedded Python 경로.
/// Production: resource_dir/python/python.exe (NSIS bundle resources 복사)
/// Dev: CARGO_MANIFEST_DIR/binaries/python/python.exe (Tauri는 dev에서 resources 자동 복사 X)
///
/// Plan §10.1 + Analysis G-I2: dev 모드에서 resource_dir이 target/debug/로 resolve되어
/// 빌드 산출물 옆에 python이 없는 문제를 cfg(debug_assertions) 가드로 회피.
pub fn embedded_python_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app.path().resource_dir().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let prod_path = base.join("python").join("python.exe");
    #[cfg(not(windows))]
    let prod_path = base.join("python").join("bin").join("python3");

    if prod_path.exists() {
        return Ok(prod_path);
    }

    // Dev 모드 fallback. release 빌드에선 prod_path.exists() 가 true여야 정상.
    #[cfg(debug_assertions)]
    {
        let dev_base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
        #[cfg(windows)]
        let dev_path = dev_base.join("python").join("python.exe");
        #[cfg(not(windows))]
        let dev_path = dev_base.join("python").join("bin").join("python3");
        if dev_path.exists() {
            return Ok(dev_path);
        }
        return Err(format!(
            "내장 실행 환경을 찾을 수 없어요. `pnpm setup:binaries` 실행 후 다시 시도해주세요. (확인: {} | {})",
            prod_path.display(),
            dev_path.display()
        ));
    }
    #[cfg(not(debug_assertions))]
    Err(format!(
        "내장 실행 환경을 찾을 수 없어요. (확인: {})",
        prod_path.display()
    ))
}

/// Sidecar 바이너리(ffmpeg/ffprobe/yt-dlp)가 위치한 디렉토리.
/// Production: resource_dir 옆 (Tauri v2 externalBin 규칙)
/// Dev: CARGO_MANIFEST_DIR/binaries/
///
/// PATH 환경변수 prepend 시 사용. demucs 등 subprocess가 ffmpeg를 찾을 때 필요.
pub fn sidecar_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resource = app.path().resource_dir().map_err(|e| e.to_string())?;
    // Production: externalBin은 resource_dir 옆 (실행파일과 동일 dir)
    if resource.join("ffmpeg-x86_64-pc-windows-msvc.exe").exists()
        || resource.join("ffmpeg.exe").exists()
    {
        return Ok(resource);
    }
    #[cfg(debug_assertions)]
    {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
        if dev.exists() {
            return Ok(dev);
        }
    }
    Ok(resource)
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 2. Probing — 원격 크기 확인 (Plan §8.2)
// ═══════════════════════════════════════════════════════════════════════════════

/// pypi 패키지의 최신 wheel 크기 추정. Windows x64 우선, 없으면 pure wheel.
/// Plan FR-14: 원격 크기 미리 확인해서 UI에 표시.
pub async fn probe_pypi_wheel_size(pkg: &str) -> Result<u64, ()> {
    let url = format!("https://pypi.org/pypi/{}/json", pkg);
    let client = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .build()
        .map_err(|_| ())?;
    let resp = client.get(&url).send().await.map_err(|_| ())?;
    if !resp.status().is_success() {
        return Err(());
    }
    let json: serde_json::Value = resp.json().await.map_err(|_| ())?;
    let urls = json.get("urls").and_then(|v| v.as_array()).ok_or(())?;

    // Preference 1: Windows x64 wheel
    for entry in urls {
        let filename = entry.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        if filename.contains("win_amd64") && filename.ends_with(".whl") {
            if let Some(size) = entry.get("size").and_then(|v| v.as_u64()) {
                return Ok(size);
            }
        }
    }
    // Preference 2: any-platform wheel
    for entry in urls {
        let filename = entry.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        if filename.contains("-none-any") && filename.ends_with(".whl") {
            if let Some(size) = entry.get("size").and_then(|v| v.as_u64()) {
                return Ok(size);
            }
        }
    }
    // Preference 3: first wheel of any kind
    for entry in urls {
        let filename = entry.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        if filename.ends_with(".whl") {
            if let Some(size) = entry.get("size").and_then(|v| v.as_u64()) {
                return Ok(size);
            }
        }
    }
    Err(())
}

/// HTTP HEAD로 Content-Length 확인. 모델 파일(dl.fbaipublicfiles.com) probing에 사용.
pub async fn probe_url_size(url: &str) -> Result<u64, ()> {
    let client = reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .build()
        .map_err(|_| ())?;
    let resp = client.head(url).send().await.map_err(|_| ())?;
    if !resp.status().is_success() {
        return Err(());
    }
    resp.content_length().ok_or(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3. Disk — 실측 / 여유 공간 / 종합 추정
// ═══════════════════════════════════════════════════════════════════════════════

/// 디렉토리 총 바이트 (재귀). FR-13: 실측 사용량 표시에 사용.
/// 경로가 없거나 권한 거부 시 0 반환 (부분 설치 중에도 안전).
pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    let mut total: u64 = 0;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            total = total.saturating_add(dir_size(&entry.path()));
        } else {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

/// `path`가 위치한 디스크에서 `required_mb` MB 이상 확보 가능한지.
/// Plan FR-11: 설치 시작 전 호출. 부족 시 disk-full 상태 진입.
pub fn check_disk_space(path: &Path, required_mb: u64) -> Result<bool, String> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();

    // 가장 구체적으로 일치하는 마운트 포인트(경로 prefix) 선택
    let mut best_match_len: usize = 0;
    let mut best_free: Option<u64> = None;
    for d in disks.list() {
        let mount = d.mount_point();
        if path.starts_with(mount) {
            let mount_len = mount.as_os_str().len();
            if mount_len >= best_match_len {
                best_match_len = mount_len;
                best_free = Some(d.available_space());
            }
        }
    }
    let free_bytes = best_free.ok_or_else(|| "디스크 정보를 확인할 수 없어요.".to_string())?;
    let free_mb = free_bytes / 1024 / 1024;
    Ok(free_mb >= required_mb)
}

/// torch + demucs(pypi) + htdemucs_ft 모델(HEAD)을 합산한 예상 설치량(MB).
/// Plan FR-14: 하나라도 probing 실패 → CONSERVATIVE_ESTIMATE_MB + probe_ok=false.
pub async fn estimate_install_size(_app: &AppHandle) -> (u64, bool) {
    let breakdown = estimate_install_size_breakdown().await;
    (breakdown.install_mb + breakdown.model_mb, breakdown.probe_ok)
}

/// install / model 분리된 추정치. Plan FR-11 (disk breakdown UI)에 사용.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeEstimate {
    /// torch + demucs (MB). probing 실패 시 CONSERVATIVE × ratio.
    pub install_mb: u64,
    /// htdemucs_ft Bag-of-4 (MB). probing 실패 시 CONSERVATIVE × (1 - ratio).
    pub model_mb: u64,
    /// 전체 probe 성공 여부. false면 fallback 값 사용. SC-12 hint 트리거.
    pub probe_ok: bool,
}

/// torch+demucs+model을 분리해서 추정. Plan §3.2 NFR: 모든 값은 probe 또는
/// CONSERVATIVE_ESTIMATE_MB의 비율로만 derive. 그 외 하드코딩 금지.
pub async fn estimate_install_size_breakdown() -> SizeEstimate {
    // 알려진 비율(Plan §10.4 + 실측 근거): torch+demucs ≈ 62%, htdemucs_ft ≈ 38%.
    // probing 실패 시 이 비율로 CONSERVATIVE_ESTIMATE_MB 분배.
    const INSTALL_RATIO_PCT: u64 = 62;

    let torch_res = probe_pypi_wheel_size("torch").await;
    let demucs_res = probe_pypi_wheel_size("demucs").await;
    let model_res = probe_model_total_size().await;

    match (torch_res, demucs_res, model_res) {
        (Ok(t), Ok(d), Some(m)) => SizeEstimate {
            install_mb: (t + d) / 1024 / 1024,
            model_mb: m / 1024 / 1024,
            probe_ok: true,
        },
        _ => {
            let install_mb = CONSERVATIVE_ESTIMATE_MB * INSTALL_RATIO_PCT / 100;
            let model_mb = CONSERVATIVE_ESTIMATE_MB.saturating_sub(install_mb);
            SizeEstimate {
                install_mb,
                model_mb,
                probe_ok: false,
            }
        }
    }
}

/// `path` 가 위치한 디스크의 가용 공간(MB). Plan FR-11 disk-full UI에 사용.
/// 디스크 정보 미상 시 0 반환 (호출자가 fits=false로 처리).
pub fn available_space_mb(path: &Path) -> u64 {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let mut best_match_len: usize = 0;
    let mut best_free: u64 = 0;
    for d in disks.list() {
        let mount = d.mount_point();
        if path.starts_with(mount) {
            let mount_len = mount.as_os_str().len();
            if mount_len >= best_match_len {
                best_match_len = mount_len;
                best_free = d.available_space();
            }
        }
    }
    best_free / 1024 / 1024
}

/// htdemucs_ft Bag-of-4 체크포인트 URL 목록 전체 크기. 하나라도 실패 시 None.
/// Phase 1: URL 목록이 비어있음 → None. Phase 2에서 실제 URL 추가.
async fn probe_model_total_size() -> Option<u64> {
    if HTDEMUCS_FT_MODEL_URLS.is_empty() {
        return None;
    }
    let mut total: u64 = 0;
    for url in HTDEMUCS_FT_MODEL_URLS {
        match probe_url_size(url).await {
            Ok(s) => total = total.saturating_add(s),
            Err(_) => return None,
        }
    }
    Some(total)
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 4. Subprocess — 환경 변수 주입 규칙 (Plan §8.2)
// ═══════════════════════════════════════════════════════════════════════════════

/// demucs 내부 ffmpeg 폴백은 PATH에서 `ffmpeg`/`ffprobe` **plain 이름**을 찾는다.
/// sidecar는 `ffmpeg-x86_64-pc-windows-msvc.exe`로 번들되므로 이름이 안 맞음
/// (process-page E2E에서 발견) → plain 이름 복사본을 app_data/bin/에 1회 준비.
pub fn ensure_plain_ffmpeg_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let bin_dir = app_data_dir(app)?.join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
    let sidecar = sidecar_dir(app)?;
    let ext = if cfg!(windows) { ".exe" } else { "" };
    for name in ["ffmpeg", "ffprobe"] {
        let dest = bin_dir.join(format!("{}{}", name, ext));
        if dest.exists() {
            continue;
        }
        let candidates = [
            sidecar.join(format!("{}-x86_64-pc-windows-msvc.exe", name)),
            sidecar.join(format!("{}{}", name, ext)),
        ];
        if let Some(src) = candidates.iter().find(|p| p.exists()) {
            let _ = std::fs::copy(src, &dest);
        }
    }
    Ok(bin_dir)
}

/// Python subprocess에 주입할 환경 변수 셋.
///   - TORCH_HOME        : 모델 캐시 리다이렉트 (~/.cache 오염 방지, FR-12)
///   - PIP_CACHE_DIR     : pip 캐시 격리
///   - PYTHONUNBUFFERED  : 진행률 stream 버퍼링 방지
///   - PATH 선두에 plain 이름 ffmpeg bin + sidecar dir prepend — demucs 폴백 대응
///
/// Phase 2에서 실제 subprocess 실행 시 사용. Phase 1에서는 API 계약만 확정.
pub fn python_env_vars(app: &AppHandle) -> Result<Vec<(String, String)>, String> {
    let appdata = app_data_dir(app)?;
    let torch = torch_cache_path(app)?;
    let pip_cache = appdata.join("pip-cache");

    // ffmpeg sidecar 디렉토리: dev/prod 모두에서 안정적으로 resolve (Analysis G-I2 fix).
    let ffmpeg_dir = sidecar_dir(app)?;

    // plain 이름 bin이 최우선 — demucs가 `ffmpeg`를 즉시 찾도록
    let mut path_val = match ensure_plain_ffmpeg_dir(app) {
        Ok(bin) => {
            let mut v = bin.to_string_lossy().to_string();
            v.push(if cfg!(windows) { ';' } else { ':' });
            v.push_str(&ffmpeg_dir.to_string_lossy());
            v
        }
        Err(_) => ffmpeg_dir.to_string_lossy().to_string(),
    };
    if let Ok(existing) = std::env::var("PATH") {
        path_val.push(if cfg!(windows) { ';' } else { ':' });
        path_val.push_str(&existing);
    }

    Ok(vec![
        ("TORCH_HOME".into(), torch.to_string_lossy().to_string()),
        ("PIP_CACHE_DIR".into(), pip_cache.to_string_lossy().to_string()),
        ("PYTHONUNBUFFERED".into(), "1".into()),
        ("PATH".into(), path_val),
    ])
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 5. Error Translation — raw 에러 → 한국어 친절 메시지 (queue-page Phase 2 신규)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Design Ref: §6.2 — setup.rs::translate_error에서 이전. ErrorContext로 호출 위치별 분기.
// queue-page Phase 2/3에서 video.rs / youtube.rs 모두 사용. setup.rs 호출부는
// `translate_error(msg, ErrorContext::Setup)`로 마이그레이션 (fix #3 / fix N).
//
// 패턴 평가 순서 (fix II): context-specific FIRST, generic SECOND, raw fallback LAST.

pub enum ErrorContext {
    Setup,
    YoutubeDownload,
    VideoExtract,
    FetchMetadata,
    /// process-page Phase 2 — separate.rs (demucs) 전용 분기. Plan FR-09.
    Separation,
}

pub fn translate_error(raw: &str, ctx: ErrorContext) -> String {
    let lower = raw.to_lowercase();

    // 1. Context-specific patterns FIRST
    match ctx {
        ErrorContext::VideoExtract => {
            if lower.contains("invalid data") || lower.contains("could not find codec") {
                return "이 파일을 읽을 수 없어요. 손상된 영상일 수 있어요.".into();
            }
            if lower.contains("timeout") {
                return "처리 시간이 너무 오래 걸려 중단했어요.".into();
            }
        }
        ErrorContext::YoutubeDownload => {
            if lower.contains("private video") || lower.contains("unavailable") {
                return "이 영상은 비공개이거나 접근할 수 없어요.".into();
            }
            if lower.contains("not available in your") || lower.contains("blocked in") {
                return "이 영상은 현재 지역에서 접근할 수 없어요.".into();
            }
            if lower.contains("requested format") {
                return "이 영상의 형식을 처리할 수 없어요.".into();
            }
        }
        ErrorContext::FetchMetadata => {
            if lower.contains("invalid data") || lower.contains("could not find codec") {
                return "이 파일의 정보를 읽을 수 없어요.".into();
            }
            if lower.contains("private video") || lower.contains("unavailable") {
                return "이 영상은 비공개이거나 접근할 수 없어요.".into();
            }
        }
        ErrorContext::Setup => {
            if lower.contains("antivirus") || lower.contains("defender") {
                return "백신 프로그램이 앱 파일을 차단하고 있어요. 예외 처리 후 다시 시도해주세요.".into();
            }
        }
        // process-page Phase 3 — Plan FR-09 친절 에러 매핑 4종 (+ 일반 Python 에러).
        // 평가 순서: OOM → ImportError → 모델 캐시 미스 → 일반 traceback (fix II 순서 규칙).
        ErrorContext::Separation => {
            if lower.contains("out of memory") {
                return "그래픽 카드 메모리가 부족해요. 더 작은 파일로 시도해 주세요.".into();
            }
            if lower.contains("importerror")
                || lower.contains("modulenotfounderror")
                || lower.contains("no module named")
            {
                return "음원 분리 엔진에 문제가 생겼어요. 설정 화면으로 돌아가 주세요.".into();
            }
            if lower.contains("no such file") || lower.contains("filenotfounderror") {
                return "AI 모델을 찾을 수 없어요. 설정을 다시 확인해 주세요.".into();
            }
            if lower.contains("traceback") {
                return "음원 분리 중 문제가 발생했어요. 다시 시도해 주세요.".into();
            }
        }
    }

    // 2. Generic patterns (모든 컨텍스트 공유)
    if lower.contains("no space left") {
        return "저장 공간이 부족해요. 정리 후 다시 시도해주세요.".into();
    }
    if lower.contains("connectionerror") || lower.contains("connect") || lower.contains("dns") {
        return "인터넷 연결이 끊겼어요. 다시 시도해주세요.".into();
    }
    if (lower.contains("access") && lower.contains("deni")) || lower.contains("permission") {
        return "파일 쓰기 권한이 없어요. 관리자 권한으로 실행하거나 백신 예외에 추가해주세요.".into();
    }

    // 3. Fallback: raw 그대로 반환 (UI [상세] 토글)
    raw.to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 6. Process Helpers — Subprocess 트리 종료 (queue-page Phase 2 신규)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Design Ref: §11.2 step 6c — setup.rs::kill_process_tree에서 이전.
// queue-page Phase 3 cancel_queue_item + setup-page cancel_install이 공유.
// Plan §2.2 Out of Scope: macOS/Linux는 v2 백로그.

#[cfg(windows)]
pub fn kill_process_tree(pid: u32) -> Result<(), String> {
    let output = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("프로세스 종료 실패: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("not found") && !stderr.contains("찾을 수 없") {
            return Err(format!("taskkill 실패: {}", stderr.trim()));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn kill_process_tree(pid: u32) -> Result<(), String> {
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests — Error Translation (process-page FR-09 매핑)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separation_maps_gpu_oom() {
        let msg = translate_error("RuntimeError: CUDA out of memory. Tried to allocate...", ErrorContext::Separation);
        assert!(msg.contains("그래픽 카드 메모리"));
    }

    #[test]
    fn separation_maps_import_error_before_no_such_file() {
        // E2E 실측 stderr: torchcodec ImportError traceback (파일 경로 라인 포함)
        let raw = "Traceback (most recent call last):\n  File \"...torchaudio\\_torchcodec.py\", line 84\nImportError: TorchCodec is required";
        let msg = translate_error(raw, ErrorContext::Separation);
        assert!(msg.contains("음원 분리 엔진에 문제"), "{}", msg);
    }

    #[test]
    fn separation_maps_model_cache_miss() {
        let msg = translate_error("FileNotFoundError: No such file or directory: 'checkpoint.th'", ErrorContext::Separation);
        // ImportError 시그널이 없으므로 모델 캐시 미스로 매핑
        assert!(msg.contains("AI 모델"), "{}", msg);
    }

    #[test]
    fn separation_maps_generic_traceback() {
        let msg = translate_error("Traceback (most recent call last):\nValueError: bad input", ErrorContext::Separation);
        assert!(msg.contains("음원 분리 중 문제"), "{}", msg);
    }

    #[test]
    fn separation_falls_back_to_raw() {
        let msg = translate_error("some unknown failure", ErrorContext::Separation);
        assert_eq!(msg, "some unknown failure");
    }
}
