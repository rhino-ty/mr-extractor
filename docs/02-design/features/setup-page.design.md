# setup-page Design Document

> **Summary**: 첫 실행 시 Embedded Python/demucs/sidecar 환경을 감지/설치하고 6가지 UI 상태를 가진 "클릭 0회" 경험. 동적 크기 probing + Phase별 분할 구현.
>
> **Project**: MR Extractor
> **Version**: 0.1.0
> **Author**: rhino-ty
> **Date**: 2026-04-24
> **Status**: Draft — Option C (Pragmatic) 제안, 사용자 확정 필요 (§2.0)
> **Planning Doc**: [setup-page.plan.md](../../01-plan/features/setup-page.plan.md) (v0.6)

## ab

## Context Anchor

> Copied from Plan v0.6. Design→Do 핸드오프에서 전략적 맥락 유지.

| Key         | Value                                                                                                                        |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------- |
| **WHY**     | Python/demucs 미설치는 앱을 못 쓰게 만들고, 설치 가이드 문서화는 사용자 이탈을 만든다. 번들 + 자동화로 "클릭 0회" 달성 필요. |
| **WHO**     | 비개발자 포함 일반 사용자. 개발자도 동일 경로.                                                                               |
| **RISK**    | ① 설치 파일 비대 (~250MB). ② demucs pip install 실패. ③ Windows SmartScreen/백신 오탐. ④ probing 실패 시 부정확한 크기 표시. |
| **SUCCESS** | 100Mbps 환경 첫 실행 후 **5분 이내** QueuePage 진입. 2회차 **2초 이내**.                                                     |
| **SCOPE**   | Phase 1: Foundation (sidecar + common + detect + UI shell), Phase 2: Install Pipeline, Phase 3: Error/Guard Paths.           |

---

## 1. Overview

### 1.1 Design Goals

1. **전략적**: "클릭 0회" + "기술 용어 0개" — 사용자는 기다리기만.
2. **구조적**: `common.rs`를 후속 피처 전부가 재사용할 기반 layer로 구축. probing/경로/subprocess 헬퍼 집중.
3. **동적**: 모든 크기/임계값은 런타임 probing 결과에서 파생. 하드코딩 literal은 `CONSERVATIVE_ESTIMATE_MB` 1개만 허용.
4. **Phase-safe**: Phase 1만 완료해도 `pnpm tauri dev`가 깨지지 않음. 중간 상태에서도 앱 실행 가능.
5. **Resilient**: 부분 설치/네트워크 끊김/디스크 부족/AV 격리 등 5가지 장애 경로를 모두 명시적 상태로 흡수.

### 1.2 Design Principles

- **Separation of Asset vs State**: 빌드 시점 에셋(`src-tauri/binaries/`)은 읽기 전용, 런타임 상태(`%APPDATA%/com.rhinoty.mr-extractor/`)는 쓰기 가능. `common.rs`에서 경로 엄격 분리.
- **Idempotent Install**: 모든 단계는 재실행해도 안전. health check로 진입 여부 결정, 마커 파일은 보조 힌트.
- **Progressive Disclosure**: 기술 에러는 토글로 접힘, 사용자 친화 메시지가 기본.
- **Dependency Inversion**: `common.rs`는 아무것도 import 안 함, `setup.rs`는 `common::*` 만 의존. 후속 피처(ModelSelector 등)도 동일 규칙.
- **Linear State Machine**: SetupPage는 6개 상태 + 명확한 전이. 백트래킹 없음.

---

## 2. Architecture Options (v1.7.0)

### 2.0 Architecture Comparison

Tauri v2 데스크톱 앱 + Svelte 5 기준. 세 가지 옵션:

| Criteria             |                 Option A: Minimal                  |                                           Option B: Clean                                           |                   Option C: Pragmatic Balance                   |
| -------------------- | :------------------------------------------------: | :-------------------------------------------------------------------------------------------------: | :-------------------------------------------------------------: |
| **Approach**         |        placeholder 그대로, setup.rs만 확장         |                      8+ 파일로 concern별 분리 (common/setup 각각 sub-modules)                       |           common.rs + setup.rs 2파일, 내부 섹션 분리            |
| **New Files**        |                   0 (기존 확장)                    | 10개 (common/mod, paths, sizing, disk, subprocess + setup/mod, detect, install, network + scripts/) | 3개 (common.rs, scripts/download-binaries.js, SetupPage 재작성) |
| **Modified Files**   | setup.rs, SetupPage, commands.ts, types.ts, lib.rs |        lib.rs, mod.rs, tauri.conf.json, capabilities, package.json, .gitignore, 프론트 전부         |           동일 but common.rs 분리로 재사용 경로 명확            |
| **Complexity**       |          Low (1 파일에 모든 로직 1000줄+)          |                                High (경로 이동 많음, 상호 의존 복잡)                                |                     Medium (300줄 × 2 파일)                     |
| **Maintainability**  |         Low — 후속 피처가 setup.rs import          |                                        High — 책임 완전 분리                                        |                  High — 경계 명확, 과분할 없음                  |
| **Effort**           |                 ~6h (Phase 1+2+3)                  |                                      ~12h (파일 분할 오버헤드)                                      |                        ~8h (Plan 예상치)                        |
| **Risk**             |         Medium (monolith, testing 어려움)          |                                Low (분리 잘됨) but 구조 잡다가 산만                                 |                         Low (실전 표준)                         |
| **후속 피처 재사용** |          어려움 — setup::sidecar_path 식           |                                 완벽 — common::paths::sidecar_path                                  |                   충분 — common::sidecar_path                   |
| **Recommendation**   |                     Quick hack                     |                                       Long-term 대형 프로젝트                                       |                      **Default choice** ⭐                      |

**Selected**: **Option C — Pragmatic Balance**

**Rationale**:

1. Option A는 지금은 빠르지만 video.rs/youtube.rs/separate.rs가 곧 `common::*`에 의존. Monolithic setup.rs를 해체하는 리팩터링 비용이 더 큼.
2. Option B는 10개 파일 분리하지만 현 프로젝트 규모(~2500 LOC Rust)에서 오버엔지니어링. 각 파일이 50~100줄짜리 `pub fn` 1~2개만 담게 됨.
3. Option C는 `common.rs` 단일 파일에 **논리적 섹션 구분자 주석**으로 영역 분리. 300줄 내외 예상, 한 파일에서 경로/probing/disk 모두 파악 가능 + 후속 피처 import는 `common::sidecar_path` 단순.

> 이 설계는 Option C 기준. 사용자가 다른 옵션 선호 시 해당 섹션부터 재작성.

### 2.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                   Svelte 5 Frontend                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  SetupPage.svelte   ← 6-state machine (local $state) │    │
│  │    ├─ onMount:        invoke('check_environment')    │    │
│  │    ├─ progress:       Channel<InstallProgress>       │    │
│  │    └─ retry handlers: re-invoke commands             │    │
│  └──────────────────────────┬──────────────────────────┘    │
│                             │                                │
│  ┌──────────────────────────▼──────────────────────────┐    │
│  │  src/lib/commands.ts  ← Tauri invoke 래퍼 + Channel  │    │
│  │  src/lib/types.ts     ← EnvStatus, InstallProgress   │    │
│  └──────────────────────────┬──────────────────────────┘    │
└─────────────────────────────┼────────────────────────────────┘
                              │ Tauri IPC
┌─────────────────────────────▼────────────────────────────────┐
│                   Rust Backend (Tauri v2)                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  src-tauri/src/commands/setup.rs                     │    │
│  │    #[tauri::command]                                 │    │
│  │    ├─ check_environment   → EnvStatus               │    │
│  │    ├─ check_internet      → bool                    │    │
│  │    ├─ install_dependencies → Channel + Result       │    │
│  │    └─ cancel_install      → ()                       │    │
│  └──────────────────────────┬──────────────────────────┘    │
│                             │                                │
│  ┌──────────────────────────▼──────────────────────────┐    │
│  │  src-tauri/src/commands/common.rs ← 후속 피처 공유   │    │
│  │    § 1. Paths:     sidecar_path, app_data_dir, ...  │    │
│  │    § 2. Probing:   probe_pypi_wheel_size, ...       │    │
│  │    § 3. Disk:      check_disk_space, dir_size, ...  │    │
│  │    § 4. Subprocess: env injection, child tree kill  │    │
│  └──────────────────────────┬──────────────────────────┘    │
└─────────────────────────────┼────────────────────────────────┘
                              │ subprocess (tokio::Command)
┌─────────────────────────────▼────────────────────────────────┐
│ sidecar 바이너리 + Embedded Python + pip + fbaipublicfiles   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow (설치 시나리오)

```
 [앱 실행]
     │
     ▼
 SetupPage.onMount()
     │
     ▼
 invoke('check_environment')  ──────────────►  setup::check_environment
     │                                              │
     │                                              ├─ common::sidecar_path x3 (메타데이터 체크)
     │                                              ├─ common::venv_python_path (존재 체크)
     │                                              ├─ run "python -m demucs --help" (health)
     │                                              ├─ common::torch_cache_path/.th x4 (모델)
     │                                              └─ common::estimate_install_size()
     │                                                  ├─ 로컬: fs::metadata
     │                                                  └─ 원격: probe_pypi_wheel_size + probe_url_size
     │                                              ▼
     ◄─────────── EnvStatus { items, all_ready, install_size_estimate_mb } ─┘
     │
     ├─ all_ready == true → state = 'ready' → 1s 후 navigateTo('queue')
     │
     └─ missing 항목 존재:
         │
         ▼
     invoke('check_internet')  ──────────►  setup::check_internet
         │                                        (HEAD pypi.org)
         ◄── false → state = 'no-internet' → UI + [다시 확인]
         │
         ├─ true ↓
         ▼
     common::check_disk_space(estimate × 1.5)
         │
         ◄── false → state = 'disk-full' → breakdown UI + [다시 확인]
         │
         ├─ true ↓
         ▼
     state = 'installing'
     channel = new Channel<InstallProgress>()
     invoke('install_dependencies', { onProgress: channel })
         │
         ▼
     setup::install_dependencies (5 phases, 각 Channel event emit)
         ├─ ExtractPython  (5%)  → fs ops
         ├─ CreateVenv     (10%) → python -m venv
         ├─ InstallTorch   (45%) → pip install torch (stdout parse)
         ├─ InstallDemucs  (55%) → pip install demucs
         └─ DownloadModel  (100%) → python -c "from demucs.pretrained import ..."
         │
         ├─ 각 phase 완료마다 Channel.send({ step, percent, phase, current_size_mb })
         │
         ├─ 성공 → write .setup-complete → Result<(), _>::Ok
         │     ▼
         │   state = 'ready' → 1s 후 전환
         │
         └─ 실패 → Result<(), _>::Err(msg)
               ▼
             state = 'error' + err_msg + [다시 시도]
```

### 2.3 Dependencies

| Component                      | Depends On                                                 | Purpose                                                |
| ------------------------------ | ---------------------------------------------------------- | ------------------------------------------------------ |
| `setup.rs`                     | `common::*`                                                | 경로/probing/disk 전부 common 경유                     |
| `common.rs`                    | `tauri::AppHandle`, `sysinfo`, `reqwest`, `tokio::process` | foundation. 다른 commands에 의존 금지.                 |
| `SetupPage.svelte`             | `$lib/commands.ts`, `$lib/types.ts`                        | invoke 래퍼만 사용. 직접 invoke 금지 (CLAUDE.md 규칙). |
| 빌드 스크립트                  | Node 20+ 내장 fetch                                        | 외부 라이브러리 의존 없음 (zero-install).              |
| 후속 피처 (v1.1 ModelSelector) | `common::*`                                                | 이 피처가 그대로 재사용. setup.rs에는 의존 X.          |

---

## 3. Data Model

### 3.1 Rust Structs (src-tauri/src/commands/setup.rs)

```rust
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

/// 환경 감지 결과 단일 항목
#[derive(Clone, Serialize)]
pub struct EnvItem {
    pub label: String,              // "오디오 변환 도구" 등 한국어 (ref COMMANDS.md 매핑)
    pub status: EnvItemStatus,
    pub version: Option<String>,    // "6.1" 등 (감지되면)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvItemStatus { Ready, Missing, Installing, Error }

/// check_environment 반환값
#[derive(Clone, Serialize)]
pub struct EnvStatus {
    pub items: Vec<EnvItem>,                // 5개 (ffmpeg/yt-dlp/python/demucs/model)
    pub all_ready: bool,                    // items 모두 Ready면 true
    pub install_size_estimate_mb: u64,      // common::estimate_install_size() 결과
    pub size_probe_succeeded: bool,         // false면 CONSERVATIVE_ESTIMATE 사용됨
}

/// install_dependencies Channel payload
#[derive(Clone, Serialize)]
pub struct InstallProgress {
    pub step: String,                       // "음원 분리 엔진 설치 중..."
    pub percent: u32,                       // 0~100 전체
    pub phase: InstallPhase,                // 내부 단계
    pub current_size_mb: Option<u32>,       // common::dir_size(app_data_dir) 실측
    pub estimated_final_mb: u32,            // probing 기반 예상
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    ExtractPython,
    CreateVenv,
    InstallTorch,
    InstallDemucs,
    DownloadModel,
}

/// .setup-complete 파일 JSON 스키마 v1
#[derive(Serialize, Deserialize)]
pub struct SetupMarker {
    pub version: u32,                       // schema version, 현재 1
    pub installed_at: String,               // ISO 8601
    pub demucs_version: String,             // "4.0.1"
    pub model_sha256: Option<String>,       // htdemucs_ft 첫 파일 SHA256
}
```

### 3.2 TypeScript Types (src/lib/types.ts 추가분)

```typescript
export type EnvItemStatus = 'ready' | 'missing' | 'installing' | 'error';

export interface EnvItem {
  label: string;
  status: EnvItemStatus;
  version: string | null;
}

export interface EnvStatus {
  items: EnvItem[];
  allReady: boolean; // serde rename from all_ready
  installSizeEstimateMb: number;
  sizeProbeSucceeded: boolean;
}

export type InstallPhase = 'extract_python' | 'create_venv' | 'install_torch' | 'install_demucs' | 'download_model';

export interface InstallProgress {
  step: string;
  percent: number;
  phase: InstallPhase;
  currentSizeMb: number | null;
  estimatedFinalMb: number;
}

/** SetupPage 내부 상태 머신 */
export type SetupPageState =
  | { kind: 'detecting' }
  | { kind: 'installing'; progress: InstallProgress; items: EnvItem[] }
  | { kind: 'ready'; items: EnvItem[]; sizeMb: number }
  | { kind: 'error'; items: EnvItem[]; message: string; detail: string }
  | { kind: 'no-internet' }
  | { kind: 'disk-full'; required: DiskBreakdown; current: number };

export interface DiskBreakdown {
  install: number; // torch + demucs
  model: number; // htdemucs_ft
  staging: number; // pip temp, 상수 (유일하게 허용되는 상수 1곳)
  headroom: number; // max(500MB, total × 0.2)
  total: number; // 합산
}
```

### 3.3 Serde Rename 규칙

Rust `snake_case` ↔ TypeScript `camelCase`:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus { ... }  // all_ready → allReady
```

enum은 `"lowercase"` 또는 `"snake_case"` 선택 — 위 예시 따름.

---

## 4. API Specification (Tauri Commands)

> REST API 아님. Tauri IPC invoke 기반. 모든 command는 `#[tauri::command]`.

### 4.1 Command List

| Name                   | Input                                                   | Return                      | Channel Event     | Notes                                         |
| ---------------------- | ------------------------------------------------------- | --------------------------- | ----------------- | --------------------------------------------- |
| `check_environment`    | `app: AppHandle`                                        | `Result<EnvStatus, String>` | —                 | 0.5~2초. FR-02, FR-09 health check 포함.      |
| `check_internet`       | —                                                       | `Result<bool, String>`      | —                 | HEAD pypi.org 3초 타임아웃. FR-07.            |
| `install_dependencies` | `app: AppHandle, on_progress: Channel<InstallProgress>` | `Result<(), String>`        | `InstallProgress` | 3~5분. FR-04/05/12/13/14.                     |
| `cancel_install`       | —                                                       | `Result<(), String>`        | —                 | 실행 중 install_dependencies child tree kill. |

### 4.2 Detailed Specification

#### `check_environment`

```rust
#[tauri::command]
pub async fn check_environment(app: AppHandle) -> Result<EnvStatus, String> {
    let mut items = Vec::with_capacity(5);

    // 1. ffmpeg sidecar (빌드 시점 번들)
    items.push(probe_sidecar(&app, "ffmpeg", "오디오 변환 도구").await);
    // 2. yt-dlp
    items.push(probe_sidecar(&app, "yt-dlp", "유튜브 다운로더").await);
    // 3. Embedded Python (런타임 venv)
    items.push(probe_python(&app, "실행 환경").await);
    // 4. demucs (health check: `python -m demucs --help` exit 0)
    items.push(probe_demucs(&app, "음원 분리 엔진").await);
    // 5. htdemucs_ft 모델 (torch-cache/hub/checkpoints/ 4개 파일)
    items.push(probe_model(&app, "htdemucs_ft", "AI 모델").await);

    let all_ready = items.iter().all(|i| matches!(i.status, EnvItemStatus::Ready));
    let (estimate, probe_ok) = common::estimate_install_size(&app).await;

    Ok(EnvStatus {
        items,
        all_ready,
        install_size_estimate_mb: estimate,
        size_probe_succeeded: probe_ok,
    })
}
```

**Error cases**: 거의 없음. 내부 probing 실패는 `EnvItemStatus::Error`로 캡처. 치명적 I/O 실패만 `Err`.

#### `check_internet`

```rust
#[tauri::command]
pub async fn check_internet() -> Result<bool, String> {
    let primary = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?
        .head("https://pypi.org")
        .send()
        .await;

    if primary.is_ok() { return Ok(true); }

    // fallback
    let fallback = /* same pattern for https://dl.fbaipublicfiles.com */;
    Ok(fallback.is_ok())
}
```

#### `install_dependencies`

```rust
#[tauri::command]
pub async fn install_dependencies(
    app: AppHandle,
    on_progress: Channel<InstallProgress>,
) -> Result<(), String> {
    let total_estimate = common::estimate_install_size(&app).await.0 as u32;
    let app_data = common::app_data_dir(&app)?;

    // helper macro로 phase마다 emit
    macro_rules! emit {
        ($phase:ident, $step:expr, $percent:expr) => {{
            let current = common::dir_size(&app_data).unwrap_or(0) / 1024 / 1024;
            on_progress.send(InstallProgress {
                step: $step.into(),
                percent: $percent,
                phase: InstallPhase::$phase,
                current_size_mb: Some(current as u32),
                estimated_final_mb: total_estimate,
            }).map_err(|e| e.to_string())?;
        }};
    }

    emit!(ExtractPython, "실행 환경 준비 중...", 5);
    extract_embedded_python(&app).await?;

    emit!(CreateVenv, "실행 환경 준비 중...", 10);
    create_venv(&app).await?;

    emit!(InstallTorch, "음원 분리 엔진 설치 중...", 15);
    pip_install(&app, "torch", &on_progress, 15..45).await?;

    emit!(InstallDemucs, "음원 분리 엔진 설치 중...", 50);
    pip_install(&app, "demucs", &on_progress, 50..55).await?;

    emit!(DownloadModel, "AI 모델 다운로드 중...", 60);
    download_model(&app, "htdemucs_ft", &on_progress, 60..100).await?;

    emit!(DownloadModel, "준비 완료!", 100);
    write_setup_marker(&app).await?;

    Ok(())
}
```

**Error 반환 예시**:

- `"venv 생성에 실패했어요. (백신 프로그램이 Python 파일을 차단하고 있을 수 있습니다)"`
- `"인터넷 연결이 끊겨 설치를 완료할 수 없어요. 다시 시도해주세요."`
- `"AI 모델 파일이 손상되었어요. 재시도하면 해결될 수 있습니다."`

#### `cancel_install`

```rust
#[tauri::command]
pub async fn cancel_install(state: State<InstallHandle>) -> Result<(), String> {
    // InstallHandle = Mutex<Option<Child>>. child tree kill (Windows: taskkill /F /T /PID)
    // 상세: Design §6.3 참조.
}
```

### 4.3 Frontend Invoke Wrappers (src/lib/commands.ts)

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';
import type { EnvStatus, InstallProgress } from './types';

export async function checkEnvironment(): Promise<EnvStatus> {
  return invoke<EnvStatus>('check_environment');
}

export async function checkInternet(): Promise<boolean> {
  return invoke<boolean>('check_internet');
}

export async function installDependencies(onProgress: (p: InstallProgress) => void): Promise<void> {
  const channel = new Channel<InstallProgress>();
  channel.onmessage = onProgress;
  return invoke<void>('install_dependencies', { onProgress: channel });
}

export async function cancelInstall(): Promise<void> {
  return invoke<void>('cancel_install');
}
```

---

## 5. UI/UX Design

### 5.1 SetupPage 6-State Machine

```
         ┌──────────────────┐
         │   detecting      │◄─── onMount
         └────────┬─────────┘
                  │ check_environment
                  ▼
          all_ready?
              │
    ┌─── Yes  │  No ──────┐
    ▼         ▼            ▼
 ready    check_internet   ?
    │         │
 1s 후        │ false ─────► no-internet ──[다시 확인]──► detecting
    │         │
 navigateTo   │ true
   queue      ▼
         check_disk_space
              │
              │ false ─────► disk-full ───[다시 확인]──► detecting
              │
              │ true
              ▼
         installing ────── 성공 ──► ready (위와 동일)
              │
              │ 실패
              ▼
          error ──[다시 시도]──► installing
                 └─[Esc]──── (개발자용) 에러 상세 토글
```

전이 규칙:

- `no-internet`/`disk-full` → 사용자 액션 후 `detecting` 재진입 (re-check)
- `error` 후 재시도는 `installing`으로 직접 (이미 detect는 완료)
- 2회차 이후 첫 `check_environment`에서 `all_ready=true`이면 `detecting → ready` 바로 스킵

### 5.2 Wireframes

> 전체 와이어프레임은 [ref UX_BEHAVIORS.md §SetupPage 화면 상태](../../references/UX_BEHAVIORS.md) 참조. 아래는 Design-specific 보완.

**detecting (0.5~2초, 2회차에선 거의 즉시)**

```
┌─────────────────────────────────────────────────────┐
│              🎵 MR Extractor                         │
│         환경을 확인하고 있어요...                      │
│                ⏳                                     │
└─────────────────────────────────────────────────────┘
```

**기타 5가지**는 ref 문서 그대로 구현.

### 5.3 Component List

| Component              | Location                 | Responsibility                              |
| ---------------------- | ------------------------ | ------------------------------------------- |
| `SetupPage.svelte`     | `src/pages/`             | 6-state 머신 + Channel listener + 전환 로직 |
| `EnvItemRow.svelte`    | `src/components/setup/`  | 개별 항목 (✅/⏳/○/❌ + 라벨 + 서브텍스트)  |
| `ProgressBar.svelte`   | `src/components/common/` | ━●━ 바 + 퍼센트 + 이전 값 smooth transition |
| `SizeBreakdown.svelte` | `src/components/setup/`  | 디스크 부족 breakdown 테이블                |
| `ErrorDetail.svelte`   | `src/components/common/` | `[▼ 상세]` 토글 + `[📋 복사]` 버튼          |

Svelte 5 컨벤션: 모든 컴포넌트는 `$props()`/`$state()` 사용.

### 5.4 Page UI Checklist

#### SetupPage (Full)

**detecting 상태:**

- [ ] Header: 🎵 MR Extractor 로고
- [ ] Subtext: "환경을 확인하고 있어요..."
- [ ] Spinner: ⏳ 아이콘 또는 CSS loader
- [ ] 최대 노출 시간 2초 (FR Performance)

**installing 상태:**

- [ ] Header: "앱을 사용할 준비를 하고 있어요..."
- [ ] EnvItemRow × 5 (오디오 변환 도구/유튜브 다운로더/실행 환경/음원 분리 엔진/AI 모델) — 현재 단계만 ⏳, 나머지는 ✅ 또는 ○
- [ ] ProgressBar: 전체 퍼센트 (0~100)
- [ ] 하단 size 표시: "사용 중: 1.7 GB / 예상: 2.4 GB" — 동적 수치
- [ ] 안내 문구: "처음 실행 시 한 번만 설치됩니다. 인터넷 필요. (약 3~5분)"
- [ ] 진행률 바 최소 2초마다 업데이트 (멈춰 보이면 안 됨)
- [ ] 어떤 문자열에도 "Python", "pip", "PyTorch", "demucs", "torch" 노출 금지 (SC-8)

**ready 상태:**

- [ ] Header: "모든 준비가 완료되었어요! ✨"
- [ ] EnvItemRow × 5, 모두 ✅
- [ ] `📊 사용 중인 공간: {dynamic_mb} GB` 표시
- [ ] "(추가 모델은 사용 시 자동 다운로드)" 서브텍스트
- [ ] 1초 후 자동 navigateTo('queue') — 사용자 클릭 불필요

**error 상태:**

- [ ] Header: "⚠ 설치 중 문제가 발생했어요"
- [ ] EnvItemRow × 5, 실패한 항목 ❌
- [ ] 친절한 한국어 메시지 (Python 모듈명 금지)
- [ ] `[▼ 오류 상세 보기]` 토글 → 접힌 상태 기본
- [ ] 펼치면: monospace 박스에 raw error
- [ ] `[🔄 다시 시도]` 버튼
- [ ] `[📋 오류 복사]` 버튼 → 클립보드 복사 → "복사되었습니다" 토스트

**no-internet 상태:**

- [ ] Header: "📡 인터넷 연결이 필요해요"
- [ ] 설명 2줄: "처음 사용하시는 경우" / "Wi-Fi 또는 유선..."
- [ ] `[🔄 다시 확인]` 버튼만

**disk-full 상태 (v0.5 신규):**

- [ ] Header: "💾 저장 공간이 부족해요"
- [ ] Breakdown 테이블:
  - 설치 필요: {install_mb} GB (동적)
    - 음원 분리 엔진 {torch+demucs_mb} GB
    - AI 모델 {model_mb} GB
  - 설치 중 임시: {staging_mb} GB
  - 권장 여유: {headroom_mb} GB
  - ───
  - 총 필요 공간: {total_mb} GB
  - 현재 공간: {free_mb} GB ❌
- [ ] `[🔄 다시 확인]` 버튼

### 5.5 Responsive/Theme

- **다크 테마 전용** (CLAUDE.md 규칙). 라이트 모드 CSS 안 씀.
- 화면 최소 너비 480px 가정. SetupPage는 고정 카드 레이아웃 (600px max-width, 중앙 정렬).
- Color tokens: `--color-bg`, `--color-surface`, `--color-accent`, `--color-success`, `--color-warn`, `--color-danger`, `--color-muted`, `--color-text`, `--color-text-secondary`.

---

## 6. Error Handling

### 6.1 Error Categories

| Category          | Examples                                   | User Message Strategy                                | Rust Handling                       |
| ----------------- | ------------------------------------------ | ---------------------------------------------------- | ----------------------------------- |
| **네트워크**      | pypi 3초 timeout, wheel 다운로드 중단      | "인터넷 연결이 끊겼어요. 다시 시도..."               | `Err(String)` + 재시도 가능         |
| **디스크**        | write 실패, 공간 부족 (설치 시작 후)       | "저장 공간이 부족해요. 정리 후 재시도..."            | error 상태, 부분 설치 rollback      |
| **권한**          | `%APPDATA%` 쓰기 거부 (드물지만)           | "파일 쓰기 권한이 없어요. 관리자 권한..."            | `Err` + 상세 stderr 노출            |
| **AV 격리**       | pip 설치 중 DLL 격리, venv python.exe 삭제 | "백신 프로그램이 앱 파일을 차단했어요. 예외 처리..." | health check가 탐지, 재설치         |
| **pip 버전 충돌** | torch wheel 호환 안 됨                     | "음원 분리 엔진 설치에 실패했어요."                  | `Err` + pip stdout/stderr tail 노출 |
| **probing 실패**  | pypi 차단된 네트워크                       | "정확한 크기를 확인할 수 없어요." 힌트               | CONSERVATIVE_ESTIMATE로 계속 진행   |
| **사용자 취소**   | `cancel_install` 호출                      | "설치가 취소되었어요."                               | child tree kill, venv 유지          |

### 6.2 Error Response Format

Rust `Err(String)`은 프론트엔드에서 그대로 받음. 파싱 없이 `err_detail`에 저장.

```typescript
try {
  await installDependencies(onProgress);
} catch (e) {
  state = {
    kind: 'error',
    items: /* 마지막 snapshot */,
    message: translateToFriendlyMessage(e as string),
    detail: e as string,
  };
}
```

사용자 친화 메시지 매핑 테이블 (`src/lib/errorMessages.ts`):

```typescript
const PATTERNS: Array<[RegExp, string]> = [
  [/No space left/i, '저장 공간이 부족해요. 정리 후 다시 시도해주세요.'],
  [/ConnectionError|timeout/i, '인터넷 연결이 끊겼어요. 다시 시도해주세요.'],
  [/Access.?denied|permission/i, '파일 쓰기 권한이 없어요. 관리자 권한으로 실행하거나 백신 예외에 추가해주세요.'],
  [/antivirus|defender/i, '백신 프로그램이 앱 파일을 차단하고 있어요. 예외 처리 후 다시 시도해주세요.'],
];
```

### 6.3 Child Process Tree Kill (cancel_install)

Rust `Child::kill()`은 직접 자식만 종료. pip가 spawn한 subprocess 누락됨. Windows에서는 `taskkill /F /T /PID`로 트리 전체 kill.

```rust
#[cfg(windows)]
fn kill_process_tree(pid: u32) -> Result<(), String> {
    std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

---

## 7. Security Considerations

- [ ] **Subprocess 인젝션 방지**: `Command::arg()` 사용, `Command::new("cmd /c ...")` 금지. URL/path를 subprocess 인자로 넘길 때 shell escape 고려.
- [ ] **sidecar 검증**: 빌드 스크립트에서 각 바이너리 SHA256 검증 (미리 수록된 hash와 대조). 변조된 바이너리 배포 방지.
- [ ] **TORCH_HOME 주입**: Python subprocess env를 명시적으로 설정, 사용자 홈의 민감한 경로 접근 차단.
- [ ] **.setup-complete 신뢰하지 않음**: 외부에서 조작 가능. 실제 health check가 진실의 원천.
- [ ] **HTTPS 강제**: pypi/fbaipublicfiles 모든 HTTP 호출 `https://`. 평문 HTTP 설정 금지.
- [ ] **민감 정보 로그 금지**: Rust log에 사용자 경로 전체 출력 금지 (`{user}` 같은 홈 디렉토리 마스킹).
- [ ] **Capabilities 최소권한**: `shell:allow-execute`를 sidecar + python 경로로만 제한. `shell:allow-open` 등 불필요 scope 추가 금지.

---

## 8. Test Plan (v2.3.0)

> Do phase에서 구현 + 테스트 1 set. Check phase에서 실행만.

### 8.1 Test Scope

| Type                 | Target                                    | Tool                    | Phase |
| -------------------- | ----------------------------------------- | ----------------------- | ----- |
| L1: Command Tests    | 4 Tauri commands — 반환 타입, 에러 경로   | Rust `#[tokio::test]`   | Do    |
| L2: UI State Tests   | SetupPage 6 상태 전이, 컴포넌트 렌더      | Playwright (Tauri mock) | Do    |
| L3: E2E Install Flow | 클린 VM → 앱 실행 → 5분 내 QueuePage 진입 | 수동 (Windows VM)       | Do    |

### 8.2 L1: Command Test Scenarios

| #   | Command                | Scenario                              | Expected                                                 |
| --- | ---------------------- | ------------------------------------- | -------------------------------------------------------- |
| 1   | `check_environment`    | 클린 머신 (아무것도 설치 안 됨)       | `items.all(Missing)`, `all_ready=false`                  |
| 2   | `check_environment`    | 2회차 (모두 설치됨)                   | `items.all(Ready)`, `all_ready=true`, size > 2GB         |
| 3   | `check_environment`    | venv만 손상 (Scripts/python.exe 삭제) | python 항목 Missing, demucs/model도 Missing              |
| 4   | `check_internet`       | Wi-Fi OFF                             | `Ok(false)`                                              |
| 5   | `check_internet`       | pypi.org 차단 + fbaipublicfiles 허용  | `Ok(true)` (fallback)                                    |
| 6   | `install_dependencies` | 설치 중 Channel 이벤트 빈도           | 2초당 최소 1회, phase 순서 맞음                          |
| 7   | `install_dependencies` | pip wheel 다운로드 중 네트워크 끊기   | `Err("인터넷 연결이 끊겼어요...")`, venv는 rollback 가능 |
| 8   | `cancel_install`       | 설치 중 호출                          | pip subprocess + 자식들 모두 종료, Ok(())                |

### 8.3 L2: UI State Tests

| #   | Scenario            | Steps                                  | Expected                                              |
| --- | ------------------- | -------------------------------------- | ----------------------------------------------------- |
| 1   | 첫 실행 정상 경로   | mock(`check_environment`, missing 5개) | detecting → installing → ready → navigate             |
| 2   | 2회차 바로 진입     | mock(all_ready=true)                   | detecting → ready (1s) → navigate                     |
| 3   | no-internet         | mock(`check_internet`, false)          | detecting → no-internet. [다시 확인] 클릭 → detecting |
| 4   | disk-full           | mock(free_space=1GB, estimate=2GB)     | detecting → disk-full. breakdown 모든 row 노출        |
| 5   | install error       | mock(install_dependencies throws)      | installing → error. [▼ 상세] 토글. [📋 복사] 동작     |
| 6   | 재시도              | error → [🔄 다시 시도] 클릭            | installing으로 재진입 (detecting 거치지 않음)         |
| 7   | 기술 용어 누출 검증 | 전 상태 snapshot 수집                  | "python", "pip", "torch", "demucs" 문자열 0회 노출    |

### 8.4 L3: E2E Install Flow

| #   | Scenario                    | Steps                                           | Success Criteria                                                          |
| --- | --------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------- |
| 1   | 클린 Windows VM 첫 실행     | `pnpm tauri build` 설치 파일 → VM에 설치 → 실행 | 100Mbps: 5분 내 QueuePage. 진행률 바 멈춤 없음.                           |
| 2   | 이미 설치된 상태 재실행     | 1번 완료 후 앱 재시작                           | 2초 내 QueuePage. installing 상태 경유 안 함.                             |
| 3   | 설치 중 전원 끊기 후 재시작 | 1번 Phase 3 중 강제 종료 → 재실행               | detecting → health check 실패 → installing 재진입 (중복 설치 없이 이어서) |
| 4   | 오프라인 첫 실행            | Wi-Fi OFF → 앱 실행                             | no-internet 화면. Wi-Fi ON 후 [다시 확인] → 정상 설치                     |
| 5   | 디스크 거의 찬 환경         | Free space 1GB로 제한 → 앱 실행                 | disk-full breakdown 화면. 정리 후 [다시 확인] → 정상 설치                 |

### 8.5 Seed Data / Fixtures

`tests/fixtures/`:

- `env_status_all_missing.json` — 첫 실행 mock
- `env_status_all_ready.json` — 2회차 mock
- `install_progress_sample.ndjson` — 5개 phase emit 시퀀스

---

## 9. Clean Architecture

### 9.1 Layer Structure (Tauri + Svelte variant)

| Layer              | Responsibility                      | Location                                          |
| ------------------ | ----------------------------------- | ------------------------------------------------- |
| **Presentation**   | Svelte 컴포넌트, 상태 머신          | `src/pages/`, `src/components/`                   |
| **Application**    | Tauri invoke 래퍼, 에러 메시지 매핑 | `src/lib/commands.ts`, `src/lib/errorMessages.ts` |
| **Domain**         | 순수 타입, 상수                     | `src/lib/types.ts`                                |
| **Infrastructure** | Rust commands, subprocess, I/O      | `src-tauri/src/commands/`                         |
| **Foundation**     | 공통 유틸 (경로/probing/disk)       | `src-tauri/src/commands/common.rs`                |

### 9.2 Dependency Rules

```
[SetupPage.svelte] ──→ [commands.ts + types.ts] ──→ [Tauri IPC]
                                                         │
                                                         ▼
                        [setup.rs] ──→ [common.rs] ──→ [tokio, reqwest, sysinfo, std]
                              ▲                ▲
                              │                │
                    (후속)  video.rs       ModelSelector 등 후속 피처
```

**역방향 금지**:

- `common.rs`는 `setup.rs`를 모름
- `setup.rs`는 `SetupPage.svelte`를 모름
- `types.ts`는 어떤 로직도 import 안 함 (pure types)

### 9.3 File Import Rules

| From               | Can Import                                                     | Cannot Import                                 |
| ------------------ | -------------------------------------------------------------- | --------------------------------------------- |
| `SetupPage.svelte` | `$lib/commands`, `$lib/types`, `svelte`, `@tauri-apps/*`       | `$lib/stores.ts` 직접 (상태는 컴포넌트 local) |
| `commands.ts`      | `@tauri-apps/api/core`, `./types`                              | `.svelte` 파일                                |
| `setup.rs`         | `crate::commands::common::*`, `tauri::*`                       | `crate::commands::{video, youtube, ...}`      |
| `common.rs`        | `std::*`, `tokio::*`, `reqwest`, `sysinfo`, `tauri::AppHandle` | 다른 commands/\*                              |

### 9.4 This Feature's Layer Assignment

| Component                    | Layer          | Location                                          |
| ---------------------------- | -------------- | ------------------------------------------------- |
| SetupPage                    | Presentation   | `src/pages/SetupPage.svelte`                      |
| EnvItemRow, ProgressBar, ... | Presentation   | `src/components/setup/`, `src/components/common/` |
| commands.ts                  | Application    | `src/lib/commands.ts`                             |
| errorMessages.ts             | Application    | `src/lib/errorMessages.ts`                        |
| types.ts                     | Domain         | `src/lib/types.ts`                                |
| setup.rs                     | Infrastructure | `src-tauri/src/commands/setup.rs`                 |
| common.rs                    | Foundation     | `src-tauri/src/commands/common.rs`                |
| scripts/download-binaries.js | Build          | `scripts/download-binaries.js`                    |

---

## 10. Coding Convention Reference

> Reference: [CLAUDE.md Design & Implementation Checklist](../../../CLAUDE.md)

### 10.1 Naming (이 피처에서 적용)

| Target             | Rule             | Example                                       |
| ------------------ | ---------------- | --------------------------------------------- |
| Svelte 컴포넌트    | PascalCase       | `SetupPage.svelte`, `EnvItemRow.svelte`       |
| TS 함수            | camelCase        | `checkEnvironment()`, `installDependencies()` |
| TS 타입/인터페이스 | PascalCase       | `EnvStatus`, `InstallProgress`                |
| Rust struct/enum   | PascalCase       | `EnvStatus`, `InstallPhase`                   |
| Rust fn            | snake_case       | `check_environment`, `probe_pypi_wheel_size`  |
| Rust const         | UPPER_SNAKE_CASE | `CONSERVATIVE_ESTIMATE_MB`                    |
| 폴더               | kebab-case       | `src/components/setup/`                       |

### 10.2 Import Order (Svelte)

```typescript
// 1. External
import { onMount } from 'svelte';
import { fade } from 'svelte/transition';

// 2. Tauri
import { invoke, Channel } from '@tauri-apps/api/core';

// 3. Internal ($lib alias)
import { checkEnvironment, installDependencies } from '$lib/commands';
import { translateToFriendlyMessage } from '$lib/errorMessages';

// 4. Type-only
import type { EnvStatus, InstallProgress, SetupPageState } from '$lib/types';

// 5. Components
import EnvItemRow from '../components/setup/EnvItemRow.svelte';
```

### 10.3 Rust Import Order

```rust
// 1. std
use std::time::Duration;

// 2. External crates
use serde::{Serialize, Deserialize};
use tauri::{AppHandle, ipc::Channel};
use tokio::process::Command;

// 3. Internal (crate::)
use crate::commands::common;
```

### 10.4 This Feature's Conventions

| Item                   | Convention                                                                       |
| ---------------------- | -------------------------------------------------------------------------------- |
| State 관리             | Component-local `$state` (stores 금지)                                           |
| 상태 전이              | Svelte 5 runes, tagged union discriminated by `kind`                             |
| 에러 메시지            | `errorMessages.ts`의 pattern matching 테이블 기반 (하드코딩된 문자열은 ref)      |
| Rust 에러              | `Result<T, String>` 반환, `?` operator 적극 사용                                 |
| 하드코딩               | **금지** — 크기 숫자는 probing 결과, 단 `CONSERVATIVE_ESTIMATE_MB = 3500`만 허용 |
| Channel payload 스키마 | Plan §8.2 Convention Table 준수 (size 필드 포함)                                 |

---

## 11. Implementation Guide

### 11.1 File Structure (구현 후)

```
mr_extractor/
├── scripts/
│   └── download-binaries.js         # Phase 1 (신규)
├── src-tauri/
│   ├── binaries/                    # Phase 1 — 빌드 스크립트가 채움 (.gitignore)
│   ├── capabilities/default.json    # Phase 1 — 권한 추가
│   ├── tauri.conf.json              # Phase 1 — externalBin, beforeBuildCommand
│   └── src/commands/
│       ├── common.rs                # Phase 1 (신규) ~300줄, 4개 섹션
│       ├── setup.rs                 # Phase 1~3 — placeholder → 본 구현
│       ├── mod.rs                   # Phase 1 — `pub mod common;` 추가
│       └── (기존) video/youtube/separate/export.rs
├── src/
│   ├── lib/
│   │   ├── commands.ts              # Phase 1/2 — 4 wrapper 추가
│   │   ├── errorMessages.ts         # Phase 3 (신규)
│   │   └── types.ts                 # Phase 1 — EnvItem/EnvStatus/InstallProgress
│   ├── components/
│   │   ├── setup/
│   │   │   ├── EnvItemRow.svelte    # Phase 1 (신규)
│   │   │   └── SizeBreakdown.svelte # Phase 3 (신규)
│   │   └── common/
│   │       ├── ProgressBar.svelte   # Phase 2 (신규)
│   │       └── ErrorDetail.svelte   # Phase 3 (신규)
│   └── pages/SetupPage.svelte       # Phase 1/2/3 — 빈 셸 → 6 상태 UI
└── package.json                     # Phase 1 — scripts.setup:binaries
```

### 11.2 Implementation Order

1. **빌드 토대** (Phase 1 시작): `scripts/download-binaries.js` 작성 → `pnpm setup:binaries` 실행해서 sidecar 확보 → `tauri.conf.json`, `capabilities/default.json` 업데이트
2. **타입 확정** (Phase 1): `types.ts` + Rust structs (양쪽 동시 정의, serde rename 검증)
3. **common.rs** (Phase 1): 4개 섹션 순서대로 — paths → probing → disk → subprocess. 각 함수 단위 테스트.
4. **check_environment 본체** (Phase 1): `common::*` 조합. Mock 환경에서 items 5개 반환 검증.
5. **UI Skeleton** (Phase 1): SetupPage.svelte + EnvItemRow. detecting/ready 상태만 먼저. `pnpm tauri dev`로 2회차 경로 검증.
6. **install_dependencies 본체** (Phase 2 시작): 5 phase sequential 구현. 각 phase 단위로 `pnpm tauri dev` 돌려 Channel 이벤트 확인.
7. **installing 상태 UI** (Phase 2): ProgressBar + size streaming. 실측 수치가 표시되는지 확인.
8. **health check + 재실행 멱등성** (Phase 2 말미): venv 수동 삭제 후 재시도 시 정상 복구.
9. **error 경로 + errorMessages** (Phase 3): 억지 에러 발생시키며 UI 검증. 클립보드 복사.
10. **no-internet + disk-full** (Phase 3): Wi-Fi off, 디스크 채운 VM에서 각 화면 렌더.
11. **cancel_install + child tree kill** (Phase 3 말미): 설치 중 강제 취소, 좀비 프로세스 없음 확인.
12. **L1/L2 테스트** (Phase 3 말미): fixtures 만들고 Rust + Playwright 실행.

### 11.3 Session Guide

> Plan §10.6 결정에 따라 Plan/Design은 통합, Do는 `--scope`로 3분할.

#### Module Map

| Module                | Scope Key | Description                                                                   | Estimated Turns |
| --------------------- | --------- | ----------------------------------------------------------------------------- | :-------------: |
| Phase 1 — Foundation  | `phase-1` | 빌드 스크립트 + common.rs + check_environment + UI Skeleton (detecting/ready) |      40~50      |
| Phase 2 — Install     | `phase-2` | install_dependencies + Channel + installing UI + health check rollback        |      40~50      |
| Phase 3 — Error Paths | `phase-3` | check_internet + check_disk_space + error/no-internet/disk-full UI + cancel   |      30~40      |

#### Phase 간 Interface Contract

**Phase 1 export** (이게 안정화돼야 Phase 2/3 진입 가능):

```rust
// common.rs (외부 노출 API)
pub async fn sidecar_path(app: &AppHandle, name: &str) -> Result<PathBuf, String>;
pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String>;
pub fn venv_python_path(app: &AppHandle) -> Result<PathBuf, String>;
pub fn torch_cache_path(app: &AppHandle) -> Result<PathBuf, String>;
pub async fn probe_pypi_wheel_size(pkg: &str) -> Result<u64, ()>;
pub async fn probe_url_size(url: &str) -> Result<u64, ()>;
pub fn dir_size(path: &Path) -> u64;
pub async fn estimate_install_size(app: &AppHandle) -> (u64, bool); // (mb, probe_ok)
pub fn check_disk_space(path: &Path, required_mb: u64) -> Result<bool, String>;
pub const CONSERVATIVE_ESTIMATE_MB: u64 = 3500;

// setup.rs (Phase 1만)
#[tauri::command] pub async fn check_environment(app: AppHandle) -> Result<EnvStatus, String>;

// types.ts
export interface EnvStatus { /* full */ }
export type SetupPageState = /* tagged union, phase 1에선 detecting/ready만 구현 */
```

**Phase 2 추가분**:

```rust
// setup.rs
#[tauri::command] pub async fn install_dependencies(app: AppHandle, on_progress: Channel<InstallProgress>) -> Result<(), String>;

// SetupPageState에 installing 케이스 활성화
```

**Phase 3 추가분**:

```rust
// setup.rs
#[tauri::command] pub async fn check_internet() -> Result<bool, String>;
#[tauri::command] pub async fn cancel_install(state: State<InstallHandle>) -> Result<(), String>;

// SetupPageState에 error/no-internet/disk-full 케이스 + errorMessages.ts
```

#### Phase 의존성 그래프

```
Phase 1 (Foundation)
  │
  │ exports: common::* 전체 API, EnvStatus, SetupPageState {detecting|ready}
  │
  ├──────┬──────────┐
  │      │          │
  ▼      │          │
Phase 2  │          │
(Install)│          │
  │      │          │
  │ uses │          │
  │ common::* + EnvStatus │
  │      │          │
  │ exports: InstallProgress Channel + SetupPageState {installing}
  │      │          │
  └──────┼──────────┤
         ▼          │
       Phase 3      │
       (Errors)     │
         │          │
         │ uses: common::check_internet/disk_space + SetupPageState 전체
         │
         │ exports: errorMessages.ts + 나머지 상태 UI
         ▼
      완료
```

**Phase 1 단독 완료 조건**:

- `pnpm tauri dev` 실행 시 SetupPage가 detecting 잠깐 → (2회차 가정) ready → queue 전환
- 설치 안 된 상태에서 앱 실행 시 UI가 깨지지 않고 "설치 필요" 상태 표시 (installing은 placeholder)
- common.rs API가 모두 Ok() 반환 (probing이 성공하든 실패하든)

**Phase 2 단독 완료 조건**:

- install_dependencies를 수동 호출 (dev tools) 하면 실제 설치 진행
- Channel 이벤트가 UI에 반영됨
- 완료 후 2회차 실행 시 skip

**Phase 3 단독 완료 조건**:

- 모든 에러 경로가 명시적 UI로 표시
- 클립보드 복사 동작
- cancel_install 호출 시 좀비 프로세스 없음

#### Success Criteria ↔ Phase 매핑

| Phase   | Addressed SCs                                                                                                                                       |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1 | SC-2 (2회차 2초), SC-5 (build 성공), SC-6 (컴파일 경고 0), SC-7 (check_environment 5항목), SC-10 (venv 수동 삭제 감지), SC-13 (모델 on-demand 경계) |
| Phase 2 | SC-1 (첫 실행 5분), SC-11 (실측/예상 동시 노출), SC-8 (기술 용어 금지 — installing UI 전수)                                                         |
| Phase 3 | SC-3 (Wi-Fi off), SC-4 (설치 중 Wi-Fi off), SC-9 (디스크 부족), SC-12 (probing 실패 fallback)                                                       |

#### Recommended Session Plan

| Session   | Phase          | Scope             |   Turns   |
| --------- | -------------- | ----------------- | :-------: |
| Session 1 | Plan + Design  | 전체              | 이미 완료 |
| Session 2 | Do Phase 1     | `--scope phase-1` |   40~50   |
| Session 3 | Do Phase 2     | `--scope phase-2` |   40~50   |
| Session 4 | Do Phase 3     | `--scope phase-3` |   30~40   |
| Session 5 | Check + Report | 전체              |   30~40   |

---

## Version History

| Version | Date       | Changes                                                                                                                                                                                               | Author   |
| ------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| 0.1     | 2026-04-24 | Initial draft. Option C (Pragmatic) 제안. 3 architecture options 비교, 6-state machine, Rust/TS 타입 정의, L1~L3 test plan, Phase 1/2/3 interface contract 및 의존성 그래프, Session Guide + SC 매핑. | rhino-ty |
