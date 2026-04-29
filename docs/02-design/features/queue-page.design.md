# queue-page Design Document

> **Summary**: 사용자 메인 허브. URL/파일 입력 → 큐 적재 → 다중 선택 + [▶ 분리 시작] → ProcessPage. setup-page Foundation API 첫 재사용. **Option C (Pragmatic)** 채택 — video.rs / youtube.rs 단일 파일 + common::§5 Error Translation + QueueHandle 별도.
>
> **Project**: MR Extractor
> **Version**: 0.1
> **Author**: rhino-ty
> **Date**: 2026-04-29
> **Status**: Draft v0.1 — Option C 확정 (사용자 승인)
> **Planning Doc**: [queue-page.plan.md v0.6](../../01-plan/features/queue-page.plan.md)

---

## Context Anchor

> Plan v0.6에서 복사. Design→Do 핸드오프 시 전략적 맥락 유지.

| Key | Value |
|---|---|
| **WHY** | 사용자는 URL 1개씩 또는 파일 1개씩 처리하는 워크플로우를 견디지 못함. 큐 + 배치 처리 + 영속화로 "여러 노래 한꺼번에 작업 후 자리 비움" 시나리오 지원. |
| **WHO** | setup-page 통과한 사용자 전원. URL 음원을 즐기는 일반인 + 로컬 음원 라이브러리 가진 음악 애호가. |
| **RISK** | ① yt-dlp stdout 포맷 변경 → 진행률 파싱. ② corrupt video → ffmpeg hang. ③ 영속화 race. ④ 임시 파일 누수. ⑤ 파일 경로 mismatch. ⑥ 비공개/지역차단. ⑦ 긴 영상. ⑧ 1000+ 큐 lag. ⑨ 좀비 프로세스. ⑩ metadata 시간 제약 위반. ⑪ yt-dlp 출력 위치 미강제. ⑫ ModelSelector 슬롯. |
| **SUCCESS** | URL/파일 입력 → 1초 내 placeholder + 5초 내 metadata. 다중 선택 → [▶ 분리 시작] → ProcessPage. 앱 재시작 시 pending 복원. 처리 중 추가 + 개별 cancel + 좀비 0. |
| **SCOPE** | Phase 1 (~3h): UI shell + Toast + 큐 stores + 다중 선택. Phase 2 (~3h): video.rs + ffprobe metadata. Phase 3 (~2h): youtube.rs + cancel + ProcessPage 라우팅. **합계 ~8h**. |

---

## 1. Overview

### 1.1 Design Goals

1. **사용자 친화**: 입력 후 1초 내 시각 피드백 (placeholder 카드). metadata 비동기 수신.
2. **Foundation 재사용 검증**: setup-page common::* API (sidecar_dir / app_data_dir / dir_size / dev_log / translate_error)가 실제 후속 피처 build에 충분한지 검증하는 첫 사례.
3. **Phase 분할 안전**: Phase 1 (UI only)만 완료해도 앱 동작 + 큐 적재 OK. Phase 2/3 미구현 상태에서도 [▶ 분리 시작] disabled tooltip으로 안내.
4. **상태 복원성**: pending 큐 영속화. 앱 재시작 시 hydrate. 파일 mismatch 자동 정리.
5. **Subprocess Resilience**: 개별 cancel + 트리 kill + 좀비 0. queue-tmp orphan 24h grace 후 자동 정리.
6. **Naming Consistency**: HISTORY.md `source_type` 통일 (queue ↔ history 호환).

### 1.2 Design Principles

- **Option C — Pragmatic Balance**: video.rs / youtube.rs 각 단일 파일 + 섹션 구분자 주석 (setup-page 패턴 답습)
- **2-step metadata 패턴**: placeholder 카드 즉시 등장 → 백그라운드 metadata fetch → 라벨 갱신
- **Korean-only UI**: yt-dlp / ffmpeg / demucs / pip / torch UI 노출 0건. dev_log는 debug 빌드 전용
- **Frontend ID 생성**: `crypto.randomUUID()` — 서버 측 uuid crate 미사용 (ID는 항상 frontend에서 출발)
- **Channel-based progress**: Tauri v2 `Channel<DownloadProgress>` / `Channel<ExtractProgress>` (setup-page InstallProgress 패턴 답습)

---

## 2. Architecture

### 2.0 Architecture Decision

**Selected**: **Option C — Pragmatic Balance** (사용자 승인)

| Criteria | Option A (Minimal) | Option B (Clean) | **Option C (Pragmatic)** ⭐ |
|---|:-:|:-:|:-:|
| New Rust files | 0 | 7+ | 0 |
| New Svelte files | 4 | 8+ | 5 |
| Phase 1 effort | ~3h | ~5h | ~3h |
| Total effort | ~8h | ~13h | ~8h |
| Plan §10.2 결정 일관 | 부분 | 모두 | 모두 |
| setup-page 패턴 일관 | 부분 | 다름 | 동일 |
| Risk | Medium (큰 파일) | Low / 산만 | Low (검증됨) |

**Selection rationale**:
1. setup-page에서 Match Rate 98.4%로 검증된 패턴 답습 → 학습 비용 0
2. Plan §10.2 모든 결정과 일관 (Toast 자체 구현 + common §5 Error Translation + QueueHandle 별도 + 5 components)
3. ~8h Plan 추정과 일치
4. K8 (subprocess Handle 일반화)은 v1.2+ 별도 리팩터 — 3가지 use case (setup install / queue / v1.1 ModelSelector 다운로드) 모이면 그때 일반화

### 2.1 Component Diagram

```
┌────────────────────────────────────────────────────────────────┐
│                     Svelte 5 Frontend                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  QueuePage.svelte ← 라우팅 진입점 + 단축키 listener         │   │
│  │  ┌─ DropZone ──┐  ┌─ UrlInput ──┐  [ModelSelector slot]   │   │
│  │  │ (drag drop) │  │ (URL+Enter) │  (v1.1 reserve, empty)  │   │
│  │  └─────────────┘  └─────────────┘                          │   │
│  │  ┌─ FileCard × N ──────────────────────────────────────┐   │   │
│  │  │ 🎵/🎬/🔗 + label + duration + status + progress + ✕ │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  │  ↓ (큐 비었으면)                                            │   │
│  │  ┌─ EmptyState ────────────────────────────────────────┐   │   │
│  │  │ 🎵 큐가 비었어요 / 🔗 URL 또는 파일을 끌어다 놓으세요 │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  │  ────── 하단 액션 바 ──────                                 │   │
│  │  [🗑 삭제 (N)]  [▶ 분리 시작 (N)]                            │   │
│  └────────────────────┬─────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  src/components/common/Toast.svelte (자체 구현)         │       │
│  │  toastStore.subscribe → 우상단 띄움, 3초 후 제거          │       │
│  └──────────────────────────────────────────────────────┘       │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  src/lib/                                              │       │
│  │   ├─ stores.ts: queueStore + isProcessing + toastStore │       │
│  │   │                + navigateTo(page, payload?) 확장    │       │
│  │   ├─ queue.ts: normalizeUrl / classifyFile             │       │
│  │   │            isDuplicate / addToQueue / removeFromQueue │     │
│  │   ├─ commands.ts: extractAudio / downloadYoutube       │       │
│  │   │                fetchYoutubeMetadata / cancelQueueItem │     │
│  │   ├─ types.ts: QueueItem, QueueItemStatus, ...         │       │
│  │   └─ errorMessages.ts (이미 setup-page에서 도입)         │       │
│  └─────────────────┬────────────────────────────────────┘       │
└────────────────────┼─────────────────────────────────────────────┘
                     │ Tauri IPC + Channel
┌────────────────────▼─────────────────────────────────────────────┐
│                     Rust Backend                                  │
│  ┌──────────────────────────────────────────────────────┐        │
│  │  src-tauri/src/commands/                               │        │
│  │   ├─ video.rs (신규 본구현, 단일 파일)                   │        │
│  │   │   § 1. ffprobe metadata (단일 JSON 호출)            │        │
│  │   │   § 2. extract_audio (ffmpeg + Channel emit)       │        │
│  │   │   § 3. cancel hook (QueueHandle PID 등록)          │        │
│  │   ├─ youtube.rs (신규 본구현, 단일 파일)                 │        │
│  │   │   § 1. fetch_metadata (--skip-download)            │        │
│  │   │   § 2. download_youtube (--output {tmp}/{id}.%ext) │        │
│  │   │   § 3. cancel hook (QueueHandle PID 등록)          │        │
│  │   │   § 4. friendly error (translate_error w/ ctx)     │        │
│  │   └─ common.rs (확장)                                   │        │
│  │       § 1. queue_tmp_dir(app) 신규                     │        │
│  │       § 5. Error Translation (신규, setup translate_error 이전) │ │
│  │           - translate_error(raw, ctx: ErrorContext)    │        │
│  │           - ErrorContext::Setup / YoutubeDownload /    │        │
│  │             VideoExtract / FetchMetadata               │        │
│  └──────────────────┬───────────────────────────────────┘        │
│  ┌──────────────────▼───────────────────────────────────┐        │
│  │  src/lib.rs (확장)                                     │        │
│  │   .manage(InstallHandle::default())  // 기존            │        │
│  │   .manage(QueueHandle::default())    // 신규 (FR-18)   │        │
│  │   .invoke_handler(generate_handler![                  │        │
│  │     ... setup commands (8개) ...,                     │        │
│  │     video::extract_audio,                              │        │
│  │     video::fetch_video_metadata,                       │        │
│  │     youtube::download_youtube,                         │        │
│  │     youtube::fetch_youtube_metadata,                   │        │
│  │     queue::cancel_queue_item,    // 신규               │        │
│  │   ])                                                   │        │
│  │                                                        │        │
│  │  QueueHandle: Mutex<HashMap<String, u32>>             │        │
│  │  (item id → child PID 매핑, FR-18)                     │        │
│  └────────────────────┬───────────────────────────────┘        │
└─────────────────────────┼──────────────────────────────────────┘
                          │ subprocess (tokio::Command)
┌─────────────────────────▼──────────────────────────────────────┐
│ ffmpeg / ffprobe sidecar  +  yt-dlp sidecar (setup-page 자산)   │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow (사용자 시나리오)

#### 시나리오 A: YouTube URL 입력

```
[사용자] URL 붙여넣기 + Enter
     │
     ▼
UrlInput.svelte
     │ normalizeUrl + isDuplicate 체크
     │
     ├─ 중복 → toastStore.push("이미 큐에 있어요") → return
     │
     ├─ invalid URL → inline error → return
     │
     └─ OK
         │
         ▼
     queue.ts::addToQueue({sourceType: "youtube", source, label: "다운로드 준비 중..."})
         │
         │ ⚡ 1초 내 FileCard 등장 (placeholder)  ← SC-1
         │ → queueStore.update + Tauri Store sync (debounce 500ms)
         │
         ├─────────────────────────────┐
         │                             │
         │ background metadata fetch    │
         │                             │
         ▼                             │
     invoke('fetch_youtube_metadata', {itemId, url})
         │                             │
         │ yt-dlp --skip-download       │
         │  --print "%(title)s\n%(duration)s"
         │
         ◄─ Result<{title, durationSec}, String> ─┘
         │
         │ ⚡ ~5초 내 라벨 갱신          ← SC-19
         │ queueStore.updateItem(id, {label: "{title} ({mm:ss})", durationSec})
         │
         └─ 사용자가 [▶ 분리 시작] 클릭 시 → 시나리오 C
```

#### 시나리오 B: 영상 파일 드래그

```
[사용자] .mp4 파일 드래그
     │
     ▼
DropZone.svelte
     │ classifyFile (확장자) + isDuplicate (절대경로)
     │
     ├─ 중복 → toast → return
     ├─ 미지원 확장자 → toast → return
     │
     └─ OK (audio 또는 video)
         │
         ▼
     queue.ts::addToQueue({sourceType: "file" 또는 "video", source: absPath, label: basename})
         │
         │ ⚡ 1초 내 FileCard 등장        ← SC-2
         │
         ▼
     invoke('fetch_video_metadata', {itemId, path})  // ffprobe (~수백ms)
         │
         ├─ corrupt (duration=0) → toast "이 파일을 읽을 수 없어요" + status=error  ← SC-15
         │
         └─ OK
             │
             │ queueStore.updateItem(id, {durationSec, label: "{basename} ({mm:ss})"})
             │
             └─ 사용자가 [▶ 분리 시작] 클릭 시 → 시나리오 C
```

#### 시나리오 C: 분리 시작 (URL/파일 공통)

```
[사용자] 다중 선택 → [▶ 분리 시작 (N)] 클릭
     │
     ▼
QueuePage.svelte
     │ selected = queue.filter(s ∈ selectedIds)
     │
     ├─ for each item in selected:
     │   ├─ if status === "pending" && sourceType === "youtube"
     │   │     → invoke('download_youtube', {itemId, url, channel})
     │   │       → status='downloading' → progress emit → status='ready-to-separate'
     │   │       → 영상이면 시나리오 B의 fetch_video_metadata + extract_audio 체이닝
     │   │
     │   ├─ if status === "pending" && sourceType === "video"
     │   │     → invoke('extract_audio', {itemId, path, channel})
     │   │       → status='extracting' → progress emit → status='ready-to-separate'
     │   │
     │   ├─ if status === "pending" && sourceType === "file" (audio)
     │   │     → tmpPath = source 그대로 (변환 불필요)
     │   │       → status='ready-to-separate' (즉시)
     │
     ├─ 모든 항목 ready-to-separate 도달
     │
     └─ navigateTo("process", { ids: [...], model: "htdemucs_ft" })  ← FR-05/19
         │ stores.ts navigateTo(page, payload?) 시그니처 확장 활용
         │
         └─ ProcessPage 진입 (별도 피처 책임)
```

#### 시나리오 D: 개별 cancel

```
[사용자] 처리 중 (status downloading/extracting) FileCard 의 [✕] 클릭
     │
     ▼
queue.ts::removeFromQueue(id)
     │
     ├─ status === downloading || extracting?
     │     → invoke('cancel_queue_item', {itemId})
     │       → QueueHandle.lock() → PID 조회 → taskkill /F /T /PID → cleanup tmp file
     │
     │ → queueStore.removeItem(id)
     │
     └─ 토스트 "취소되었어요" (선택)
```

### 2.3 Dependencies

| Component | Depends On | Purpose |
|---|---|---|
| `video.rs` | `common::*` (sidecar_dir / queue_tmp_dir / dev_log / translate_error / ErrorContext::VideoExtract) + `tokio::process` + `tauri::ipc::Channel` | ffprobe + ffmpeg subprocess + 진행률 + 친절 에러 |
| `youtube.rs` | `common::*` (동일 + ErrorContext::YoutubeDownload) + `tokio::process` + `tauri::ipc::Channel` | yt-dlp subprocess + metadata + 진행률 + 친절 에러 |
| `common.rs §5 Error Translation` | `std` only | 에러 매핑 단일 출처. setup.rs translate_error 이전 |
| `QueueHandle` (lib.rs) | `std::sync::Mutex` + `HashMap` | item id → PID 매핑. cancel_queue_item이 사용 |
| `QueuePage.svelte` | `$lib/stores` (queueStore + navigateTo) + `$lib/queue` + `$lib/commands` + `$lib/errorMessages` | UI 진입점 + 단축키 + 라우팅 |
| `DropZone / UrlInput / FileCard / EmptyState` | `$lib/queue` + `queueStore` | UI primitives |
| `Toast.svelte` | `$lib/stores::toastStore` | UI primitives. 다른 페이지에서도 재사용 |
| `setup.rs` | (no change in API) | translate_error만 common.rs로 이전, 호출부 수정 |

**역방향 금지**:
- `common.rs`는 `video/youtube/queue` 모름
- `video.rs` ↔ `youtube.rs` 서로 모름 (병렬)
- `lib.rs`는 모든 commands 등록만 책임

---

## 3. Data Model

### 3.1 Rust Structs (video.rs / youtube.rs / queue handle)

```rust
// video.rs
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub item_id: String,
    pub duration_sec: u32,  // ffprobe duration. 0이면 corrupt 시그널
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractProgress {
    pub item_id: String,
    pub percent: u32,  // 0~100
}

// youtube.rs
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
    pub step: String,    // "다운로드 중..." 한국어 (Plan FR-13 + SC-11)
    pub percent: u32,
}

// lib.rs
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct QueueHandle(pub Mutex<HashMap<String, u32>>);  // item id → child PID

impl QueueHandle {
    pub fn register(&self, id: String, pid: u32) {
        if let Ok(mut g) = self.0.lock() { g.insert(id, pid); }
    }
    pub fn take(&self, id: &str) -> Option<u32> {
        self.0.lock().ok().and_then(|mut g| g.remove(id))
    }
}
```

### 3.2 TypeScript Types (src/lib/types.ts 추가분)

```typescript
// HISTORY.md JSON 스키마와 source_type 명명 일치 (Plan Iteration 1 fix C1)
export type QueueSourceType = "youtube" | "file" | "video";

export type QueueItemStatus =
  | "pending"            // 큐 적재됨, 처리 대기
  | "fetching-metadata"  // yt-dlp --skip-download 또는 ffprobe 실행 중
  | "downloading"        // yt-dlp 다운로드 중 (URL only)
  | "extracting"         // ffmpeg 오디오 추출 중 (영상 only)
  | "ready-to-separate"  // 추출/다운로드 완료, ProcessPage 진입 가능
  | "in-progress"        // ProcessPage가 separate.rs 실행 중 (queue read-only)
  | "done"               // 완료 (HISTORY로 이관됨, 휘발)
  | "error";             // 실패

export interface QueueItem {
  id: string;                    // crypto.randomUUID()
  sourceType: QueueSourceType;
  source: string;                // URL 또는 절대경로 (HISTORY 명명과 일치)
  label: string;                 // 사용자 표시. placeholder → metadata 갱신
  durationSec?: number;          // 영상/오디오 길이 (FR-17). 미상 시 undefined
  tmpPath?: string;              // 다운로드/추출 후 wav 경로
  progress: number;              // 0~100
  step?: string;                 // "다운로드 중..." / "오디오 추출 중..." (FR-13)
  status: QueueItemStatus;
  errorDetail?: string;          // status==="error" 시 raw stderr
  addedAt: string;               // ISO 8601, 정렬 기준 (오래된→최신, top-down)
}

export interface VideoMetadata {
  itemId: string;
  durationSec: number;
}

export interface YoutubeMetadata {
  itemId: string;
  title: string;
  durationSec: number;
}

export interface DownloadProgress {
  itemId: string;
  step: string;
  percent: number;
}

export interface ExtractProgress {
  itemId: string;
  percent: number;
}

// Toast (Phase 1, Plan §10.2 자체 구현)
export type ToastKind = "info" | "success" | "warn" | "error";
export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  durationMs: number;  // default 3000
}

// 페이지별 navigateTo payload
export type NavigatePayload =
  | { kind: "queue" }
  | { kind: "process"; ids: string[]; model: "htdemucs_ft" }  // FR-19 model 고정
  | { kind: "player"; itemId: string }
  | { kind: "history" }
  | { kind: "settings" }
  | { kind: "setup" };
```

### 3.3 Serde Rename 규칙

setup-page와 동일: Rust `snake_case` ↔ TS `camelCase`. 모든 struct에 `#[serde(rename_all = "camelCase")]`.

---

## 4. API Specification (Tauri Commands)

### 4.1 Command List

| Name | Input | Return | Channel Event | Notes |
|---|---|---|---|---|
| `fetch_video_metadata` | `app: AppHandle, item_id: String, path: String` | `Result<VideoMetadata, String>` | — | ffprobe 단일 호출. duration=0 → corrupt 시그널 |
| `extract_audio` | `app, item_id, path, on_progress: Channel<ExtractProgress>, handle: State<QueueHandle>` | `Result<String, String>` (tmpPath) | `ExtractProgress` | ffmpeg subprocess + Channel + 30분 timeout |
| `fetch_youtube_metadata` | `app, item_id: String, url: String` | `Result<YoutubeMetadata, String>` | — | yt-dlp `--skip-download --print` (~3~5초). 실패 시 fallback Err |
| `download_youtube` | `app, item_id, url, on_progress: Channel<DownloadProgress>, handle: State<QueueHandle>` | `Result<String, String>` (tmpPath) | `DownloadProgress` | yt-dlp `--output {tmp}/{id}.%(ext)s --no-playlist --no-mtime --no-warnings`. timeout 없음 (NFR Limits) |
| `cancel_queue_item` | `item_id: String, handle: State<QueueHandle>` | `Result<(), String>` | — | QueueHandle.take(id) → tree kill (Windows taskkill /F /T /PID) + tmp cleanup |

### 4.2 Detailed Specifications

#### `fetch_video_metadata`

```rust
#[tauri::command]
pub async fn fetch_video_metadata(
    app: AppHandle,
    item_id: String,
    path: String,
) -> Result<VideoMetadata, String> {
    common::dev_log(&app, &format!("queue:fetch_video_metadata({}): start", item_id));

    let ffprobe = common::sidecar_dir(&app)?.join("ffprobe-x86_64-pc-windows-msvc.exe");
    let output = tokio_timeout(
        Duration::from_secs(10),
        TokioCommand::new(&ffprobe)
            .args(["-v", "quiet", "-print_format", "json", "-show_format", &path])
            .output(),
    )
    .await
    .map_err(|_| common::translate_error("timeout", common::ErrorContext::FetchMetadata))?
    .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(common::translate_error(
            &String::from_utf8_lossy(&output.stderr),
            common::ErrorContext::FetchMetadata,
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
        return Err("이 파일을 읽을 수 없어요. 손상된 영상일 수 있어요.".into());  // SC-15
    }

    Ok(VideoMetadata { item_id, duration_sec })
}
```

#### `extract_audio`

```rust
#[tauri::command]
pub async fn extract_audio(
    app: AppHandle,
    item_id: String,
    path: String,
    on_progress: Channel<ExtractProgress>,
    handle: State<'_, QueueHandle>,
) -> Result<String, String> {
    let total_sec = (fetch_video_metadata(app.clone(), item_id.clone(), path.clone()).await?).duration_sec;
    let tmp_path = common::queue_tmp_dir(&app)?.join(format!("{}.wav", item_id));

    let ffmpeg = common::sidecar_dir(&app)?.join("ffmpeg-x86_64-pc-windows-msvc.exe");
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
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| common::translate_error(&e.to_string(), common::ErrorContext::VideoExtract))?;

    if let Some(pid) = child.id() {
        handle.register(item_id.clone(), pid);
    }

    // stderr time= 파싱 → percent 계산 → Channel emit (2초마다)
    // (구현 디테일: setup::pip_install ticker 패턴 응용, 30분 timeout)
    // ...

    handle.take(&item_id);  // 정리

    if !child.wait().await.map_err(|e| e.to_string())?.success() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(common::translate_error("ffmpeg fail", common::ErrorContext::VideoExtract));
    }

    Ok(tmp_path.to_string_lossy().to_string())
}
```

#### `fetch_youtube_metadata`

```rust
#[tauri::command]
pub async fn fetch_youtube_metadata(
    app: AppHandle,
    item_id: String,
    url: String,
) -> Result<YoutubeMetadata, String> {
    let ytdlp = app.shell().sidecar("yt-dlp")
        .map_err(|e| common::translate_error(&e.to_string(), common::ErrorContext::FetchMetadata))?;

    let output = tokio_timeout(
        Duration::from_secs(10),
        ytdlp.args([
            "--skip-download",
            "--no-playlist",
            "--no-warnings",
            "--print", "%(title)s",
            "--print", "%(duration)s",
            &url,
        ]).output(),
    )
    .await
    .map_err(|_| common::translate_error("timeout", common::ErrorContext::FetchMetadata))?
    .map_err(|e| common::translate_error(&e.to_string(), common::ErrorContext::FetchMetadata))?;

    if !output.status.success() {
        return Err(common::translate_error(
            &String::from_utf8_lossy(&output.stderr),
            common::ErrorContext::YoutubeDownload,  // 비공개/지역 차단 메시지
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let title = lines.next().unwrap_or("").to_string();
    let duration_sec = lines.next().and_then(|s| s.trim().parse::<u32>().ok()).unwrap_or(0);

    Ok(YoutubeMetadata { item_id, title, duration_sec })
}
```

#### `download_youtube`

```rust
#[tauri::command]
pub async fn download_youtube(
    app: AppHandle,
    item_id: String,
    url: String,
    on_progress: Channel<DownloadProgress>,
    handle: State<'_, QueueHandle>,
) -> Result<String, String> {
    let tmp_dir = common::queue_tmp_dir(&app)?;
    let output_pattern = tmp_dir.join(format!("{}.%(ext)s", item_id));

    let ytdlp = app.shell().sidecar("yt-dlp")
        .map_err(|e| common::translate_error(&e.to_string(), common::ErrorContext::YoutubeDownload))?;

    // FR-15/16: --no-playlist + --output 강제
    let cmd = ytdlp.args([
        "--output", &output_pattern.to_string_lossy(),
        "--no-playlist",
        "--no-mtime",
        "--no-warnings",
        &url,
    ]);

    // [download] N% / N MB 라인 정규식 파싱 → Channel emit
    // QueueHandle.register(item_id, pid)
    // (디테일은 setup::pip_install 패턴 응용)

    // 실제 다운로드 파일 이름: {item_id}.{webm|mp4|...}
    // glob으로 찾아서 반환
    // ...
    todo!("download_youtube 본구현 — Phase 3");
}
```

#### `cancel_queue_item`

```rust
#[tauri::command]
pub async fn cancel_queue_item(
    item_id: String,
    handle: State<'_, QueueHandle>,
    app: AppHandle,
) -> Result<(), String> {
    let Some(pid) = handle.take(&item_id) else {
        return Ok(());  // 멱등성 — 이미 종료됨
    };
    common::dev_log(&app, &format!("queue:cancel_queue_item({}): kill PID {}", item_id, pid));
    kill_process_tree(pid)?;
    // tmp 파일 cleanup (best-effort)
    let tmp_pattern = common::queue_tmp_dir(&app)?;
    let _ = tokio::fs::remove_file(tmp_pattern.join(format!("{}.wav", item_id))).await;
    Ok(())
}

// kill_process_tree는 setup.rs에서 이전 (translate_error와 함께)
// 또는 setup.rs / queue 양쪽이 common::kill_process_tree 호출하도록 이전
```

### 4.3 Frontend Invoke Wrappers (src/lib/commands.ts 추가분)

```typescript
import { Channel, invoke } from "@tauri-apps/api/core";
import type { DownloadProgress, ExtractProgress, VideoMetadata, YoutubeMetadata } from "./types";

export async function fetchVideoMetadata(itemId: string, path: string): Promise<VideoMetadata> {
  return invoke<VideoMetadata>("fetch_video_metadata", { itemId, path });
}

export async function extractAudio(
  itemId: string,
  path: string,
  onProgress: (p: ExtractProgress) => void,
): Promise<string> {
  const channel = new Channel<ExtractProgress>();
  channel.onmessage = onProgress;
  return invoke<string>("extract_audio", { itemId, path, onProgress: channel });
}

export async function fetchYoutubeMetadata(itemId: string, url: string): Promise<YoutubeMetadata> {
  return invoke<YoutubeMetadata>("fetch_youtube_metadata", { itemId, url });
}

export async function downloadYoutube(
  itemId: string,
  url: string,
  onProgress: (p: DownloadProgress) => void,
): Promise<string> {
  const channel = new Channel<DownloadProgress>();
  channel.onmessage = onProgress;
  return invoke<string>("download_youtube", { itemId, url, onProgress: channel });
}

export async function cancelQueueItem(itemId: string): Promise<void> {
  return invoke<void>("cancel_queue_item", { itemId });
}
```

---

## 5. UI/UX Design

### 5.1 QueuePage Layout

```
┌──────────────────────────────────────────────────────────────┐
│  ┌── 상단 입력 영역 ──────────────────────────────────────┐   │
│  │  ┌── DropZone ──────────┐  ┌── UrlInput ────────────┐ │   │
│  │  │  📂 파일을 끌어다 놓기  │  │  🔗 YouTube URL...  ➤ │ │   │
│  │  │  (또는 Ctrl+O)         │  │                          │ │   │
│  │  └────────────────────────┘  └────────────────────────┘ │   │
│  │  [ ModelSelector slot — v1.1 reserve ]                  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ── 큐 영역 (큐 비었으면 EmptyState 표시) ──                      │
│                                                                  │
│  ┌── FileCard #1 ──────────────────────────────────────────┐   │
│  │  🔗  소란 - 사랑한 마음엔 죄가 없다 (3:42)               │   │
│  │      ━━━━━━━●━━━━━━━━━━━  45%  다운로드 중...    [✕]   │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌── FileCard #2 (선택됨, bg-accent/20) ────────────────────┐   │
│  │  🎵  Adele - Hello.mp3 (4:55)                  [Ready]  │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌── FileCard #3 ──────────────────────────────────────────┐   │
│  │  🎬  movie.mp4 (영상에서 추출, 2:30)                     │   │
│  │      ━━━━━━━━━━━━━━●━━━  85%  오디오 추출 중...  [✕]    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ── 하단 액션 바 (선택 1개 이상 시) ──                            │
│  [🗑 삭제 (1)]                          [▶ 분리 시작 (1)]      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘

EmptyState (큐 비었을 때):
┌──────────────────────────────────────────────────────────────┐
│                                                                │
│                          🎵                                    │
│                    큐가 비었어요                                │
│                                                                │
│            🔗 URL을 붙여넣거나 파일을 끌어다 놓으세요            │
│                                                                │
└──────────────────────────────────────────────────────────────┘
```

### 5.2 Component List

| Component | Location | Responsibility |
|---|---|---|
| `QueuePage.svelte` | `src/pages/` | 라우팅 진입 + 단축키 listener (Delete/Ctrl+A/Ctrl+O/Escape) + ModelSelector 슬롯 |
| `DropZone.svelte` | `src/components/queue/` | 파일 드래그 + drag-over visual + Ctrl+O 파일 다이얼로그 fallback |
| `UrlInput.svelte` | `src/components/queue/` | URL 입력 + normalize + invalid inline error |
| `FileCard.svelte` | `src/components/queue/` | 큐 항목 1개 (아이콘+라벨+길이+상태+진행률+✕) |
| `EmptyState.svelte` | `src/components/queue/` | 빈 큐 일러스트 (이모지 2종 + 1줄 안내) |
| `Toast.svelte` | `src/components/common/` | 자체 구현 (Plan §10.2). 우상단 stack, 3초 후 dismiss |

Svelte 5 컨벤션: 모든 컴포넌트 `$props()` / `$state()` / `$derived` 사용.

### 5.3 Page UI Checklist

#### EmptyState
- [ ] 🎵 이모지 + "큐가 비었어요" 헤드
- [ ] 🔗 이모지 + "URL을 붙여넣거나 파일을 끌어다 놓으세요" 1줄 안내
- [ ] DropZone/UrlInput은 EmptyState 위에 그대로 노출 (액션 가능 상태)

#### DropZone
- [ ] 정상 상태: `border-border` 점선 + "📂 파일을 끌어다 놓기 (또는 Ctrl+O)"
- [ ] drag-over: `border-accent` + `bg-accent/10` (응답 < 100ms, NFR)
- [ ] Ctrl+O 클릭 가능 (전역 단축키, 다른 페이지 active 시 navigateTo)
- [ ] 미지원 확장자 파일 드롭 → 토스트 "지원하지 않는 파일 형식이에요"
- [ ] 다중 파일 드롭 → N개 동시 추가 (FR-02)

#### UrlInput
- [ ] placeholder: "🔗 YouTube URL을 붙여넣고 Enter"
- [ ] Enter 키 = 추가
- [ ] invalid URL → input 하단 inline error (한국어)
- [ ] 추가 후 input 비움

#### FileCard
- [ ] 아이콘: sourceType별 (🔗 youtube / 🎵 file audio / 🎬 video)
- [ ] 라벨: placeholder ("다운로드 준비 중..." or basename) → metadata 갱신 후 "{title} ({mm:ss})"
- [ ] sourceType="video"인 경우 "(영상에서 추출)" 서브 표시
- [ ] status별 배지: pending=○ / fetching-metadata=⏳ / downloading=⏳+% / extracting=⏳+% / ready-to-separate=✅ / in-progress=▶ / done=✓ / error=❌
- [ ] 처리 중 (downloading/extracting): 진행률 바 + step 텍스트 "다운로드 중..." 또는 "오디오 추출 중..."
- [ ] 처리 중 [✕] 버튼 우측 표시 (FR-18 cancel)
- [ ] 선택됨: `bg-accent/20` 하이라이트 (체크박스 미사용, Plan Convention)
- [ ] 클릭: 선택 토글. Ctrl+클릭 = 추가/제거. Shift+클릭 = 범위.

#### 액션 바
- [ ] 선택 0개: 액션 바 숨김
- [ ] 선택 ≥1: [🗑 삭제 (N)] + [▶ 분리 시작 (N)] 표시
- [ ] [▶ 분리 시작]: Phase 1에서는 disabled + tooltip "Phase 2/3에서 사용 가능해요" (FR-05)

#### Toast
- [ ] 우상단 stack, 가장 최근이 위
- [ ] 3초 후 자동 dismiss
- [ ] kind별 색: info=accent / success=success / warn=warn / error=danger

### 5.4 Responsive / Theme

- 다크 테마 전용 (`--color-*` CSS 변수)
- 최소 너비 480px 가정. 큐 항목은 한 줄 (FileCard width 100% — 라벨 길면 ellipsis)
- ModelSelector 슬롯: Design 결정 — **상단 헤더 영역, DropZone 옆 작은 셀렉터 자리** (가장 자연스러운 위치)

---

## 6. Error Handling

### 6.1 Error Categories

| Category | Examples | User Message Strategy | Source |
|---|---|---|---|
| **invalid URL** | "youtube com 빠짐", protocol 없음 | UrlInput inline error: "올바른 URL이 아니에요" | UrlInput.svelte normalizeUrl 검증 |
| **unsupported file** | .doc, .pdf 등 | toast: "지원하지 않는 파일 형식이에요" | DropZone.svelte classifyFile |
| **duplicate** | 이미 큐에 있는 URL/파일 | toast: "이미 큐에 있어요" | queue.ts isDuplicate |
| **network** | yt-dlp 다운로드 중 disconnect | toast (FileCard error 상태): "인터넷 연결이 끊겼어요" | translate_error YoutubeDownload |
| **YouTube 비공개/지역** | yt-dlp stderr "Private video" / "Video unavailable" | "이 영상은 비공개이거나 접근할 수 없어요" | translate_error YoutubeDownload |
| **YouTube 코덱** | "Requested format is not available" | "이 영상의 형식을 처리할 수 없어요" | translate_error YoutubeDownload |
| **corrupt video** | ffprobe duration=0 | toast: "이 파일을 읽을 수 없어요. 손상된 영상일 수 있어요." | fetch_video_metadata Err |
| **ffmpeg timeout** | 30분 초과 | toast: "처리 시간이 너무 오래 걸려 중단했어요" | extract_audio timeout |
| **사용자 취소** | [✕] 클릭 | toast: "취소되었어요" (선택) | cancel_queue_item Ok |
| **디스크 부족** | extract 도중 No space left | toast: "저장 공간이 부족해요" | translate_error generic |
| **권한** | %APPDATA% 쓰기 거부 | toast: "파일 쓰기 권한이 없어요" | translate_error generic |

### 6.2 common::§5 Error Translation (신규)

```rust
// common.rs §5 — Plan §10.2 결정으로 신규 추가, setup.rs translate_error 이전
use std::collections::HashMap;

pub enum ErrorContext {
    Setup,             // setup-page 호환 (기존 translate_error 동작)
    YoutubeDownload,   // youtube.rs 전용
    VideoExtract,      // video.rs 전용
    FetchMetadata,     // metadata 추출 (URL/file 공용)
}

pub fn translate_error(raw: &str, ctx: ErrorContext) -> String {
    let lower = raw.to_lowercase();

    // 1. Generic patterns (모든 컨텍스트 공유)
    if lower.contains("no space left") {
        return "저장 공간이 부족해요. 정리 후 다시 시도해주세요.".into();
    }
    if lower.contains("connection") || lower.contains("timeout") || lower.contains("dns") {
        return "인터넷 연결이 끊겼어요. 다시 시도해주세요.".into();
    }
    if lower.contains("access denied") || lower.contains("permission") {
        return "파일 쓰기 권한이 없어요.".into();
    }

    // 2. Context-specific patterns
    match ctx {
        ErrorContext::Setup => {
            if lower.contains("antivirus") || lower.contains("defender") {
                return "백신 프로그램이 앱 파일을 차단하고 있어요. 예외 처리 후 다시 시도해주세요.".into();
            }
        }
        ErrorContext::YoutubeDownload | ErrorContext::FetchMetadata => {
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
        ErrorContext::VideoExtract => {
            if lower.contains("invalid data") || lower.contains("could not find codec") {
                return "이 파일을 읽을 수 없어요. 손상된 영상일 수 있어요.".into();
            }
            if lower.contains("timeout") {
                return "처리 시간이 너무 오래 걸려 중단했어요.".into();
            }
        }
    }

    // 3. Fallback: raw 그대로 반환 (UI에서 [상세] 토글 노출)
    raw.to_string()
}
```

### 6.3 Frontend Error Display

- 큐 항목 error: FileCard ❌ + 라벨 옆 message + [상세] 토글 (raw stderr)
- 사용자 액션 안내: Toast (3초)
- inline validation: UrlInput 하단 텍스트

---

## 7. Security Considerations

- [ ] **Subprocess 인젝션 방지**: yt-dlp/ffmpeg/ffprobe 모두 `Command::arg()` 사용. URL/path를 shell 문자열로 결합 금지
- [ ] **URL 검증**: normalizeUrl에서 `new URL()` 파싱 실패 시 reject. youtube.com/youtu.be 외 도메인은 inline error
- [ ] **파일 경로 traversal**: DropZone은 OS native picker 결과만 신뢰 (사용자 직접 입력 path 미허용)
- [ ] **임시 파일 격리**: queue-tmp/는 `%APPDATA%/com.rhinoty.mr-extractor/` 내. 외부 접근 X
- [ ] **HTTPS 강제**: yt-dlp 자체가 HTTPS 사용. plain HTTP URL 입력은 normalize 시 https로 변환 시도
- [ ] **민감 정보 로그 금지**: dev_log에 사용자 홈 경로 마스킹 권장 (절대경로 → "{user}/...")
- [ ] **Capabilities 최소권한**: 추가 X (sidecar/fs/store/dialog 모두 setup-page에서 등록됨)
- [ ] **PID 기반 cancel**: QueueHandle은 메모리 only. 디스크 영속 X (보안 + 단순성)

---

## 8. Test Plan

### 8.1 Test Scope

| Type | Target | Tool | Phase |
|---|---|---|---|
| L1: Command Tests | 5 Tauri commands — 반환 타입, 에러 경로 | Rust `#[tokio::test]` | Do |
| L2: UI State Tests | QueuePage 7 상태 전이, FileCard 진행률 갱신 | Playwright (Tauri mock) | Do |
| L3: E2E Flow | 실제 환경 → URL/파일 추가 → 분리 시작 → ProcessPage placeholder 도달 | 수동 (Windows) | Do |

### 8.2 L1: Command Test Scenarios

| # | Command | Scenario | Expected |
|---|---|---|---|
| 1 | `fetch_video_metadata` | 정상 mp4 (10초) | duration_sec=10, Ok |
| 2 | `fetch_video_metadata` | corrupt video (0 byte) | Err "이 파일을 읽을 수 없어요" |
| 3 | `fetch_video_metadata` | non-existent path | Err |
| 4 | `extract_audio` | 정상 mp4 → wav 출력 | tmp_path 반환, file 존재 |
| 5 | `extract_audio` | 도중 cancel_queue_item | subprocess 종료 + tmp 파일 cleanup + Err 반환 |
| 6 | `fetch_youtube_metadata` | 정상 URL | title + duration_sec |
| 7 | `fetch_youtube_metadata` | 비공개 영상 URL | Err "이 영상은 비공개이거나 접근할 수 없어요" |
| 8 | `download_youtube` | 정상 URL | tmp_path 반환, file 존재 |
| 9 | `download_youtube` | 출력 위치 강제 검증 | file이 정확히 `queue_tmp_dir/{id}.{ext}` 위치 (작업 디렉토리 X) |
| 10 | `cancel_queue_item` | 진행 중 PID 등록 후 호출 | Ok, taskkill 성공 |
| 11 | `cancel_queue_item` | 등록되지 않은 id 호출 | Ok (멱등성) |

### 8.3 L2: UI State Tests

| # | Scenario | Steps | Expected |
|---|---|---|---|
| 1 | URL 입력 → 카드 등장 | UrlInput에 valid URL + Enter | < 1초 내 FileCard placeholder ("다운로드 준비 중...") |
| 2 | metadata fetch 갱신 | mock fetch_youtube_metadata return | 라벨 "{title} (3:42)"로 갱신 |
| 3 | 다중 파일 드롭 | 3개 .mp4 동시 드래그 | 3개 카드 즉시 등장 |
| 4 | 다중 선택 + 삭제 | Ctrl+클릭 2회 + Shift+클릭 → Delete | 정확히 N개 제거 |
| 5 | 중복 추가 | 동일 URL 두 번 입력 | 두 번째는 토스트 + 큐 그대로 |
| 6 | 앱 재시작 hydrate | pending 3건 + 재시작 | 3건 복원 (파일 항목은 exists 체크) |
| 7 | Phase 1 disabled 버튼 | 큐 1개 + 선택 → [▶ 분리 시작] | disabled + tooltip 표시 |
| 8 | EmptyState 표시 | 큐 빈 상태 | EmptyState 일러스트 보임 |
| 9 | 단축키 Ctrl+A | 큐 5개 + Ctrl+A | 모두 선택됨 |
| 10 | DropZone drag-over | 파일 hover | border-accent + bg-accent/10 (< 100ms) |

### 8.4 L3: E2E Flow

| # | Scenario | Steps | Success Criteria |
|---|---|---|---|
| 1 | YouTube URL 분리 흐름 | URL 붙여넣기 → 카드 등장 → metadata → 다운로드 → 추출 → ProcessPage placeholder 진입 | 전 흐름 에러 없음, 각 단계 progress UI 멈춤 0회 |
| 2 | 영상 파일 분리 흐름 | .mp4 드롭 → ffprobe → 추출 → ProcessPage 진입 | 동일 |
| 3 | 다중 항목 배치 | URL 2개 + .mp4 1개 → 모두 선택 → [▶ 분리 시작] | 3개 모두 ready-to-separate → ProcessPage |
| 4 | 처리 중 cancel | 다운로드 50% 도중 [✕] | 좀비 0 (작업관리자 확인) + tmp 파일 정리 |
| 5 | 앱 재시작 시나리오 | 큐 3개 (pending) + 종료 + 재시작 | 3개 복원, [▶ 분리 시작] 정상 동작 |
| 6 | 비공개 URL | 비공개 영상 URL | toast "이 영상은 비공개이거나..." |
| 7 | corrupt 파일 | 0-byte .mp4 드롭 | toast "이 파일을 읽을 수 없어요" |

### 8.5 Seed Data / Fixtures

`tests/fixtures/queue/`:
- `sample_audio.mp3` — 정상 (3초)
- `sample_video.mp4` — 정상 (10초)
- `corrupt_video.mp4` — 0-byte
- `youtube_metadata_sample.json` — yt-dlp --print mock 응답
- `download_progress_sample.ndjson` — yt-dlp [download] 라인 시퀀스

---

## 9. Clean Architecture

### 9.1 Layer Structure

| Layer | Responsibility | Location |
|---|---|---|
| **Presentation** | Svelte 컴포넌트 | `src/pages/QueuePage.svelte`, `src/components/queue/`, `src/components/common/Toast.svelte` |
| **Application** | Tauri invoke wrapper, Toast/queue 헬퍼 | `src/lib/commands.ts`, `src/lib/queue.ts`, `src/lib/errorMessages.ts` |
| **Domain** | 순수 타입 + stores 계약 | `src/lib/types.ts`, `src/lib/stores.ts` |
| **Infrastructure** | Rust commands + subprocess | `src-tauri/src/commands/video.rs`, `youtube.rs`, `lib.rs (QueueHandle)` |
| **Foundation** | 공통 utility (재사용) | `src-tauri/src/commands/common.rs` (§5 Error Translation 신규 추가) |

### 9.2 Dependency Rules

```
[QueuePage] ──→ [stores + queue + commands] ──→ [Tauri IPC + Channel]
                                                       │
                                                       ▼
[video.rs / youtube.rs] ──→ [common.rs (§1-§5)] ──→ [tokio + sysinfo + std]
        ▲              ▲
        │              │
        └─ (no peer)   └─ setup.rs도 common::translate_error 호출 (위치만 이전)
```

**역방향 금지**:
- common.rs는 video/youtube/queue/setup 모름
- video.rs ↔ youtube.rs 서로 모름
- types.ts는 어떤 로직도 import 안 함

### 9.3 File Import Rules

| From | Can Import | Cannot Import |
|---|---|---|
| `QueuePage.svelte` | `$lib/*`, `svelte`, `@tauri-apps/*` | 다른 페이지 컴포넌트 |
| `commands.ts` | `@tauri-apps/api/core`, `./types` | `.svelte` 파일 |
| `video.rs` | `crate::commands::common::*`, `tauri::*`, `tokio::*` | `crate::commands::{youtube, separate, ...}` |
| `youtube.rs` | 동일 | 동일 |
| `common.rs` | `std::*`, `tokio::*`, `reqwest`, `sysinfo`, `tauri::AppHandle`, `serde_json` | 다른 commands/* |

---

## 10. Coding Convention Reference

> Plan §8 Convention Prerequisites + setup-page Design §10 그대로 적용. queue-page 추가 사항만:

### 10.1 Naming

- 큐 컴포넌트: PascalCase (`DropZone.svelte`, `FileCard.svelte`)
- TS 함수: camelCase (`addToQueue`, `normalizeUrl`)
- Rust struct: PascalCase (`QueueHandle`, `VideoMetadata`)
- Rust fn: snake_case (`fetch_video_metadata`, `extract_audio`)
- 폴더: kebab-case (`src/components/queue/`)

### 10.2 Import Order (Svelte)

```typescript
// 1. External
import { onMount } from "svelte";
import { fade } from "svelte/transition";

// 2. Tauri
import { invoke, Channel } from "@tauri-apps/api/core";

// 3. Internal
import { addToQueue, removeFromQueue, normalizeUrl } from "$lib/queue";
import { extractAudio, downloadYoutube, cancelQueueItem } from "$lib/commands";
import { queueStore, navigateTo, toastStore } from "$lib/stores";
import { translateToFriendlyMessage } from "$lib/errorMessages";

// 4. Type-only
import type { QueueItem, DownloadProgress } from "$lib/types";

// 5. Components
import DropZone from "../components/queue/DropZone.svelte";
import UrlInput from "../components/queue/UrlInput.svelte";
import FileCard from "../components/queue/FileCard.svelte";
import EmptyState from "../components/queue/EmptyState.svelte";
import Toast from "../components/common/Toast.svelte";
```

---

## 11. Implementation Guide

### 11.1 File Structure (구현 후)

```
mr_extractor/
├── src-tauri/src/commands/
│   ├── video.rs           (Phase 2 신규 본구현, ~300줄)
│   ├── youtube.rs         (Phase 3 신규 본구현, ~350줄)
│   ├── common.rs          (확장 — §1 queue_tmp_dir + §5 Error Translation)
│   └── ... (setup.rs translate_error 호출부 수정)
├── src-tauri/src/
│   └── lib.rs             (QueueHandle 등록 + 5 신규 handler)
└── src/
    ├── lib/
    │   ├── stores.ts      (queueStore + isProcessing + toastStore + navigateTo 시그니처 확장)
    │   ├── queue.ts       (Phase 1 신규)
    │   ├── commands.ts    (확장 — 5 wrappers)
    │   ├── types.ts       (확장 — QueueItem 등 8 타입)
    │   └── errorMessages.ts (이미 setup-page에서 도입)
    ├── components/
    │   ├── queue/         (Phase 1 신규)
    │   │   ├── DropZone.svelte
    │   │   ├── UrlInput.svelte
    │   │   ├── FileCard.svelte
    │   │   └── EmptyState.svelte
    │   └── common/
    │       └── Toast.svelte  (Phase 1 신규)
    └── pages/
        └── QueuePage.svelte  (Phase 1 본구현, ~300줄)
```

### 11.2 Implementation Order

1. **Phase 1 시작 — types + stores + queue.ts** (Foundation): types.ts에 QueueItem 등 정의 → stores.ts queueStore + Tauri Store sync + navigateTo 확장 → queue.ts normalizeUrl/classifyFile/isDuplicate
2. **Phase 1 — Toast 컴포넌트** (공통 인프라): Toast.svelte + toastStore 자체 구현 (~50 lines)
3. **Phase 1 — UI primitives**: DropZone → UrlInput → FileCard → EmptyState 순
4. **Phase 1 — QueuePage 조립**: 컴포넌트 합치기 + 단축키 listener + 액션 바 (분리 시작 disabled)
5. **Phase 1 단독 검증** (`pnpm tauri dev`): URL/파일 추가 + 다중 선택 + 삭제 + 앱 재시작 hydrate
6. **Phase 2 시작 — common 확장**: §1 queue_tmp_dir 추가 + §5 Error Translation (setup translate_error 이전, 호출부 수정)
7. **Phase 2 — video.rs**: fetch_video_metadata (ffprobe JSON) → extract_audio (ffmpeg + Channel + cancel hook)
8. **Phase 2 — frontend 통합**: commands.ts wrapper + FileCard에 진행률 emit → status 갱신
9. **Phase 2 단독 검증**: 영상 .mp4 드롭 → 추출 진행 → ready-to-separate
10. **Phase 3 시작 — youtube.rs**: fetch_youtube_metadata + download_youtube + friendly error 매핑
11. **Phase 3 — QueueHandle + cancel_queue_item**: lib.rs State 등록 + cancel command + Windows tree kill
12. **Phase 3 — ProcessPage 라우팅**: navigateTo("process", { ids, model })  
13. **Phase 3 단독 검증**: URL 다운로드 → ProcessPage placeholder 진입, [✕] cancel 좀비 0
14. **L1/L2/L3 테스트**: fixtures 작성 + Rust + Playwright 실행

### 11.3 Session Guide

#### Module Map

| Module | Scope Key | Description | Estimated Turns |
|---|---|---|:-:|
| Phase 1 — UI Shell + queueStore | `phase-1` | types + stores (queueStore + isProcessing + toastStore + navigateTo 확장) + queue.ts (normalize/classify/dedupe/add/remove) + Toast.svelte + DropZone/UrlInput/FileCard/EmptyState 4 components + QueuePage 조립 + 단축키 + ModelSelector slot reserve | 30~40 |
| Phase 2 — Local File Pipeline | `phase-2` | common §1 queue_tmp_dir + common §5 Error Translation (setup translate_error 이전 + 호출부 수정) + video.rs (fetch_video_metadata + extract_audio + cancel hook) + commands.ts wrapper + FileCard 진행률 emit | 30~40 |
| Phase 3 — YouTube + Cancel + Routing | `phase-3` | youtube.rs (fetch_youtube_metadata + download_youtube + friendly error) + QueueHandle State + cancel_queue_item + lib.rs handler 등록 + URL 정규화 통합 + ProcessPage 라우팅 (placeholder) | 25~35 |

#### Phase 의존성 그래프

```
Phase 1 (UI Shell + queueStore + Foundation TS)
  │
  │ exports: queueStore + QueueItem types + queue.ts API + Toast + navigateTo(payload?)
  │
  ├──────────────┬─────────────┐
  │              │             │
  ▼              ▼             │
Phase 2          Phase 3       │
(video.rs +      (youtube.rs + │
common §1+§5)    QueueHandle)  │
  │              │             │
  │ uses:        │ uses:       │
  │ queueStore   │ queueStore  │
  │ FileCard     │ FileCard    │
  │              │ + Phase 2의 │
  │              │ extract_audio (체이닝) │
  │              │             │
  │ exports:     │ exports:    │
  │ extractAudio │ downloadYt  │
  │ + ffprobe    │ + cancel    │
  │              │ + nav route │
  └──────┬───────┘             │
         ▼                     │
  후속 피처 (ProcessPage — 별도 PDCA 사이클)
```

#### Phase 1 Interface Contract

```typescript
// src/lib/stores.ts
export const queueStore: Writable<QueueItem[]>;
export const isProcessing: Readable<boolean>;  // queueStore.derived
export const toastStore: Writable<Toast[]>;
export function navigateTo(page: PageName, payload?: NavigatePayload): void;

// src/lib/queue.ts
export function normalizeUrl(raw: string): string | null;  // null = invalid
export function classifyFile(path: string): "audio" | "video" | "unknown";
export function isDuplicate(item: QueueItem, queue: QueueItem[]): boolean;
export function addToQueue(item: QueueItem): void;        // 중복 체크 + toast 포함
export function removeFromQueue(id: string): Promise<void>; // 처리 중이면 cancel
```

#### Phase 2 추가분

```rust
// common.rs
pub fn queue_tmp_dir(app: &AppHandle) -> Result<PathBuf, String>;
pub fn translate_error(raw: &str, ctx: ErrorContext) -> String;
pub enum ErrorContext { Setup, YoutubeDownload, VideoExtract, FetchMetadata }

// video.rs
#[tauri::command] pub async fn fetch_video_metadata(...) -> Result<VideoMetadata, String>;
#[tauri::command] pub async fn extract_audio(...) -> Result<String, String>;
```

#### Phase 3 추가분

```rust
// youtube.rs
#[tauri::command] pub async fn fetch_youtube_metadata(...) -> Result<YoutubeMetadata, String>;
#[tauri::command] pub async fn download_youtube(...) -> Result<String, String>;

// lib.rs
pub struct QueueHandle(pub Mutex<HashMap<String, u32>>);
#[tauri::command] pub async fn cancel_queue_item(...) -> Result<(), String>;
// .manage(QueueHandle::default()) + 5 신규 handler 등록
```

#### Phase 단독 완료 조건

**Phase 1**:
- `pnpm tauri dev` → QueuePage 정상 표시
- URL/파일 추가 → 1초 내 placeholder 카드
- 다중 선택 + Delete = 삭제, Ctrl+A = 전체 선택, Ctrl+O = 다이얼로그
- 앱 재시작 → pending 큐 복원 (mismatch 자동 제거)
- [▶ 분리 시작] disabled + tooltip
- Toast 동작 (중복 안내 등)

**Phase 2**:
- 영상 드롭 → ffprobe metadata 표시 → ffmpeg 추출 진행률 → ready-to-separate
- corrupt video → "이 파일을 읽을 수 없어요" 토스트
- 추출 도중 [✕] cancel → tmp 파일 정리

**Phase 3**:
- URL → metadata 5초 내 갱신 → 다운로드 → (영상이면) 추출 체이닝 → ready-to-separate
- 비공개 URL → "이 영상은 비공개이거나..." 토스트
- [▶ 분리 시작] → ProcessPage placeholder 진입 (실제 ProcessPage는 별도 피처)
- 다운로드 도중 [✕] cancel → 좀비 0 (작업관리자 확인)

#### Recommended Session Plan

| Session | Phase | Scope | Turns |
|---|---|---|:-:|
| 1 | Plan + Design | 전체 | 이미 완료 |
| 2 | Do Phase 1 | `--scope phase-1` | 30~40 |
| 3 | Do Phase 2 | `--scope phase-2` | 30~40 |
| 4 | Do Phase 3 | `--scope phase-3` | 25~35 |
| 5 | Check + Report | 전체 | 25~30 |

#### SC ↔ Phase 매핑

| Phase | Addressed SCs |
|---|---|
| Phase 1 | SC-1, SC-2 (file 부분), SC-3, SC-7, SC-8, SC-12, SC-13, SC-18, SC-20 (시각화 placeholder), SC-21 (model grep) |
| Phase 2 | SC-4 (Phase 3와 함께), SC-9, SC-15, SC-20 (extracting), SC-2 (file metadata 갱신) |
| Phase 3 | SC-5, SC-6, SC-10, SC-11, SC-14, SC-16, SC-17, SC-19, SC-20 (downloading) |

---

## Version History

| Version | Date | Changes | Author |
|---|---|---|---|
| 0.1 | 2026-04-29 | Initial draft. Option C (Pragmatic) 사용자 승인. Plan v0.6 완전 흡수. 11 sections, Module Map 3 modules + 단독 완료 조건 + Phase 의존성 그래프 + SC↔Phase 매핑 + Interface Contract per phase. | rhino-ty |
