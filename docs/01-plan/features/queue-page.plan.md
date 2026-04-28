# queue-page Planning Document

> **Summary**: 사용자 메인 허브. YouTube URL 또는 로컬 파일(오디오/영상) 입력 → 큐에 적재 → 다중 선택 + [▶ 분리 시작] → ProcessPage 라우팅. setup-page Foundation API (common::*) 재사용.
>
> **Project**: MR Extractor
> **Version**: 0.1
> **Author**: rhino-ty
> **Date**: 2026-04-29
> **Status**: Draft v0.1 — Checkpoint 1+2 완료, 6 Key Decisions 확정

---

## Executive Summary

| Perspective | Content |
|---|---|
| **Problem** | setup-page 후 사용자는 "무엇을 처리할지" 입력해야 함. 두 입력 경로(YouTube URL, 로컬 파일)가 분리되어 있고 영상 파일은 사전 오디오 추출이 필요. 큐 관리 + 다중 선택 + 처리 시작 흐름이 없으면 사용자는 한 번에 한 개씩만 처리 가능. |
| **Solution** | QueuePage = DropZone (파일 드래그) + UrlInput (URL 붙여넣기) + 큐 리스트 (FileCard×N) + 다중 선택 액션 바. 영상은 `video.rs::extract_audio`로 자동 추출, URL은 `youtube.rs::download_youtube`로 다운로드 후 ProcessPage 라우팅. Tauri Store로 pending 큐 영속화. |
| **Function/UX Effect** | 사용자: URL 붙여넣기 또는 파일 드래그 → 큐에 즉시 추가 → 여러 항목 선택 → [▶ 분리 시작] → ProcessPage 자동 진입. 앱 재시작해도 pending 큐 복원. 처리 중에도 큐 적재 가능. |
| **Core Value** | "유튜브 URL → MR 한 방에" 킬러 피처의 사용자 진입점 + 배치 처리 게이트웨이. setup-page Foundation API (common::sidecar_dir / app_data_dir) 즉시 재사용 검증. |

---

## Context Anchor

| Key | Value |
|---|---|
| **WHY** | 사용자는 URL 1개씩 또는 파일 1개씩 처리하는 워크플로우를 견디지 못함. 큐 + 배치 처리 + 영속화로 "여러 노래 한꺼번에 작업 후 자리 비움" 시나리오 지원. |
| **WHO** | setup-page 통과한 사용자 전원. URL 음원을 즐기는 일반인 + 로컬 음원 라이브러리 가진 음악 애호가. |
| **RISK** | ① yt-dlp stdout 포맷 변경 → 진행률 파싱 깨짐. ② 영상 파일 corrupt → ffmpeg hang. ③ 큐 영속화 동시 쓰기 race. ④ 임시 파일 공간 누수 (분리 실패 후 cleanup 안 됨). ⑤ 큐 영속화 vs 사용자가 옮긴 파일 경로 mismatch. |
| **SUCCESS** | URL 또는 파일 입력 → 1초 이내 큐 추가. 다중 선택 → [▶ 분리 시작] → ProcessPage. 앱 재시작 시 pending 큐 복원. 처리 중 큐 추가 가능. |
| **SCOPE** | Phase 1: UI shell + DropZone + UrlInput + 큐 stores + 다중 선택. Phase 2: video.rs (영상→오디오 추출). Phase 3: youtube.rs (yt-dlp 다운로드) + ProcessPage 라우팅. |

---

## 1. Overview

### 1.1 Purpose

setup-page 통과 후 사용자가 가장 많은 시간을 보낼 화면. URL/파일 입력을 받고 큐로 관리하며 분리 처리를 시작하는 "허브" 역할.

### 1.2 Background

setup-page에서 sidecar 바이너리 (ffmpeg/ffprobe/yt-dlp) + Embedded Python venv + demucs + 모델 모두 확보 완료. queue-page는 이 자산을 처음으로 **활용**하는 피처.

| 자원 | 출처 | queue-page에서 사용처 |
|---|---|---|
| ffmpeg sidecar | setup-page | 영상→오디오 추출 (video.rs) |
| ffprobe sidecar | setup-page | 영상 길이 사전 확인 (진행률 계산) |
| yt-dlp sidecar | setup-page | YouTube 다운로드 (youtube.rs) |
| `common::sidecar_dir` | setup-page Foundation | sidecar 경로 resolve (dev/prod fallback) |
| `common::app_data_dir` | setup-page Foundation | %APPDATA%/com.rhinoty.mr-extractor/queue-tmp/ |
| Tauri Store plugin | 이미 capabilities 등록 | pending 큐 영속화 |

### 1.3 Related Documents

- **선행 피처**: [setup-page.report.md](../../04-report/setup-page.report.md) — Foundation API 14종 export
- **참조 스펙**:
  - [docs/references/COMMANDS.md](../../references/COMMANDS.md) — youtube.rs / video.rs 시그니처
  - [docs/references/UI.md](../../references/UI.md) — 다중 선택 UI 규칙 (Ctrl+클릭, Shift+클릭)
  - [docs/references/UX_BEHAVIORS.md](../../references/UX_BEHAVIORS.md) — 중복 처리, 종료 안전 처리, URL 정규화
  - [docs/references/FILE_FORMATS.md](../../references/FILE_FORMATS.md) — DropZone 확장자 분류
- **로드맵**: [docs/ROADMAP.md](../../ROADMAP.md) v1 — `QueuePage` + `youtube.rs` + `video.rs`

---

## 2. Scope

### 2.1 In Scope

#### Phase 1 — UI Shell (Frontend only)
- [ ] `src/pages/QueuePage.svelte` — 라우팅 진입점 (Phase 1 UI shell)
- [ ] `src/components/queue/DropZone.svelte` — 파일 드래그&드롭 (확장자 필터)
- [ ] `src/components/queue/UrlInput.svelte` — YouTube URL 입력 (정규화 + Enter)
- [ ] `src/components/queue/FileCard.svelte` — 큐 항목 1개 (아이콘 + 라벨 + 상태 + 진행률)
- [ ] `src/components/queue/EmptyState.svelte` — 빈 큐 일러스트 + 안내
- [ ] `src/lib/stores.ts` 확장 — `queueStore` (Svelte writable) + Tauri Store sync
- [ ] `src/lib/queue.ts` (신규) — URL 정규화, 확장자 분류, 중복 체크 헬퍼
- [ ] 다중 선택 (Ctrl+클릭, Shift+클릭, 전체 선택)
- [ ] 하단 액션 바: [🗑 삭제 (N)] / [▶ 분리 시작 (N)]

#### Phase 2 — Local File Pipeline
- [ ] `src-tauri/src/commands/video.rs::extract_audio` 본구현
  - ffprobe로 총 재생시간 사전 확인 → 진행률 계산
  - ffmpeg `-i video -vn -acodec pcm_s16le -ar 44100 -ac 2 -y out.wav`
  - stderr `time=HH:MM:SS` 파싱 → Channel emit
  - 임시 파일 출력 위치: `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{id}.wav`
- [ ] FileCard에 영상→오디오 추출 단계 시각화 (Plan §10.2 결정)
- [ ] 추출 완료 후 큐 항목 상태 → `ready-to-separate`

#### Phase 3 — YouTube Pipeline + ProcessPage Routing
- [ ] `src-tauri/src/commands/youtube.rs::download_youtube` 본구현
  - yt-dlp `--no-playlist` (단일 URL만, FR-15)
  - stdout 진행률 파싱 (yt-dlp `[download]` 라인)
  - 출력: `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{id}.{ext}`
  - 영상이면 즉시 extract_audio 체이닝
- [ ] `src/lib/queue.ts::normalizeUrl` — youtube/youtu.be → `youtube.com/watch?v={id}` 통일 (ref UX_BEHAVIORS.md)
- [ ] [▶ 분리 시작] 클릭 → 선택 큐 항목 IDs를 ProcessPage에 전달 → `navigateTo("process")` (FR-05)
- [ ] 처리 중에도 큐 추가 허용 (FR-09)

### 2.2 Out of Scope

- `separate.rs` (demucs) → **ProcessPage** 책임
- `export.rs` → **PlayerPage** 책임
- 모델 선택 (htdemucs/htdemucs_ft/htdemucs_6s) → **v1.1 ModelSelector**
- 처리 히스토리 → **v1.1 HistoryPage**
- 중복 감지 모달 (선택지 다이얼로그) → **v1.1**, queue-page는 "건너뜀" 기본만 (FR-11)
- yt-dlp 플레이리스트 자동 펼침 → **v1.1+** (`--no-playlist` 강제)
- 다중 영상 URL 일괄 입력 → **v1.1+** (싱글 URL만)
- 임시 파일 정리 (앱 uninstall 시) → **v1.2 app-lifecycle** (UX_BEHAVIORS.md "앱 종료 안전 처리"와 함께)
- BPM 감지 / EQ → **v2+**

---

## 3. Requirements

### 3.1 Functional Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| FR-01 | UrlInput에 URL 입력 + Enter → 정규화 + 중복 체크 → 1초 내 큐에 추가 | High | Pending |
| FR-02 | DropZone에 파일 드래그 → 확장자 분류 (audio/video/unknown) + 중복 체크 → 1초 내 큐 추가 | High | Pending |
| FR-03 | 큐 항목 클릭 = 선택 토글. Ctrl+클릭 = 개별 추가/제거. Shift+클릭 = 범위 선택. Cmd/Ctrl+A = 전체 선택 | High | Pending |
| FR-04 | [🗑 삭제 (N)] 버튼 = 선택된 항목 일괄 삭제. 처리 중인 항목은 cancel + 삭제 | High | Pending |
| FR-05 | [▶ 분리 시작 (N)] 버튼 = 선택 항목 처리 큐 진입 + ProcessPage 라우팅 (URL 파라미터로 IDs 전달) | High | Pending |
| FR-06 | 영상 확장자(mp4/mkv/mov/avi/webm/wmv/flv/ts/m2ts) 항목은 video.rs::extract_audio 자동 호출 → wav 변환 후 ready-to-separate. FileCard에 "(영상에서 추출)" 표시 | High | Pending |
| FR-07 | YouTube URL 항목은 youtube.rs::download_youtube 자동 호출 (`--no-playlist` 강제). 영상이 다운로드되면 FR-06 체이닝 | High | Pending |
| FR-08 | 큐 상태(`pending` 항목만) Tauri Store에 영속화. 앱 재시작 시 hydrate. 단, **파일이 없으면 자동 제거** (mismatch 감지) | High | Pending |
| FR-09 | 처리 중에도 DropZone/UrlInput 활성. 새 항목은 `pending` 상태로 큐 끝에 적재. 사용자 액션 차단 없음 | Medium | Pending |
| FR-10 | 임시 파일 출력: `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{id}.{ext}` ({id}는 큐 항목 UUID, {ext}는 wav 또는 원본 video ext) | High | Pending |
| FR-11 | 중복 감지 기본값 = **건너뜀** (URL 정규화 후 + 파일 절대경로 기준). 토스트로 "이미 큐에 있어요" 안내. 모달 X | Medium | Pending |
| FR-12 | 빈 큐 상태: EmptyState 일러스트 + DropZone 안내 ("URL을 붙여넣거나 파일을 끌어다 놓으세요"). FileCard 영역 hide | Medium | Pending |
| FR-13 | 처리 중 항목 시각화: ⏳ 아이콘 + 진행률 바 (% + step 텍스트, "다운로드 중..." / "오디오 추출 중...") | Medium | Pending |
| FR-14 | URL 정규화 (ref UX_BEHAVIORS.md): youtube.com / youtu.be / m.youtube.com / music.youtube.com → 모두 `youtube.com/watch?v={id}` 통일 | High | Pending |
| FR-15 | yt-dlp는 항상 `--no-playlist` 옵션 강제. 플레이리스트 URL 입력 시에도 첫 영상만 추가 | High | Pending |

### 3.2 Non-Functional Requirements

| Category | Criteria | Measurement |
|---|---|---|
| Performance | URL 입력/파일 드래그 → 큐 추가 < 1초 | console.time |
| Performance | 영상→오디오 추출 진행률 < 2초 단위 갱신 | Channel emit 빈도 로그 |
| Performance | 1000개 큐 항목 시 다중 선택 응답 < 100ms | 가상 스크롤 미적용 (1000 미만 가정), 필요 시 Phase 4+ |
| Reliability | 큐 영속화 충돌 방지: write 큐 (debounce 500ms) | 동시 추가 5건 stress test |
| Reliability | 처리 실패 시 임시 파일 cleanup (다음 앱 시작 시 orphan 정리) | startup 시 queue-tmp/ scan |
| UX | 다크 테마 전용 (`--color-*` CSS 변수 준수) | 코드 리뷰 |
| Error Clarity | 다운로드 실패 시 한국어 메시지 + 원본 stderr | translate_error 패턴 setup-page와 동일 |
| Tech Jargon | UI 본문에 yt-dlp / ffmpeg / demucs / pip 노출 0건 | grep 검증 |
| Storage | 임시 파일 누수 < 1GB (정상 운영 시) | startup orphan cleanup |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] **SC-1**: URL 입력 후 1초 이내 큐에 카드 등장 (FR-01)
- [ ] **SC-2**: 파일 드래그 후 1초 이내 큐에 카드 등장 (FR-02)
- [ ] **SC-3**: 다중 선택 (Ctrl+클릭 3회 + Shift+클릭) → [🗑 삭제 (N)] → 정확히 N개 삭제 (FR-03/04)
- [ ] **SC-4**: 영상 파일 (.mp4) 드래그 → "(영상에서 추출)" 표시 + 진행률 → ready-to-separate 상태 → [▶ 분리 시작] → ProcessPage 진입 (FR-06)
- [ ] **SC-5**: YouTube URL (다양한 형식: youtu.be / m.youtube.com / 재생목록 URL) → 정규화 후 동일 v=ID → 다운로드 → 첫 영상만 추가 (FR-07/14/15)
- [ ] **SC-6**: 처리 중 (FileCard ⏳ 표시) URL/파일 추가 → 큐 끝에 pending 적재 (FR-09)
- [ ] **SC-7**: pending 항목 3개 있는 상태에서 앱 재시작 → 큐 복원 (단, 파일 사라진 항목은 자동 제거) (FR-08)
- [ ] **SC-8**: 같은 URL 두 번 입력 → 두 번째는 토스트 "이미 큐에 있어요" + 큐 그대로 (FR-11)
- [ ] **SC-9**: 영상 추출 후 임시 파일 정확히 `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{uuid}.wav` 위치 (FR-10)
- [ ] **SC-10**: `pnpm tauri build` 성공, `cargo check` 0 warnings, `pnpm check` 0 errors
- [ ] **SC-11**: UI 본문 grep — "yt-dlp" / "ffmpeg" / "demucs" / "pip" 0건
- [ ] **SC-12**: 빈 큐 상태에서 EmptyState 일러스트 + 안내 문구 표시 (FR-12)
- [ ] **SC-13**: 같은 절대경로 파일 두 번 드래그 → 두 번째는 건너뜀 (FR-11)

### 4.2 Quality Criteria

- [ ] Gap Analysis Match Rate ≥ 90%
- [ ] 한국어 라벨 전수 검수 (UI.md UX 규칙 준수)
- [ ] setup-page Foundation API 재사용 검증 (common::sidecar_dir / app_data_dir)
- [ ] capabilities scope 추가 없이 기존 sidecar/fs/store 권한만으로 동작

---

## 5. Risks and Mitigation

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| yt-dlp stdout 포맷 변경 → 진행률 파싱 깨짐 | High | Medium | `[download]` 라인 정규식 + fallback (% 추출 실패 시 indeterminate spinner). yt-dlp 버전 고정 권장 (현재 sidecar 2026.03.17) |
| 영상 파일 corrupt → ffmpeg hang | Medium | Low | ffprobe 사전 검증 (재생시간 = 0 또는 stderr error → 즉시 실패). subprocess timeout 30분 |
| 큐 영속화 동시 쓰기 race | Medium | Low | Tauri Store write debounce 500ms + atomic 단일 store key |
| 임시 파일 공간 누수 | Medium | Medium | startup 시 queue-tmp/ scan → 큐에 없는 orphan 자동 삭제 (24h grace 후) |
| 큐 영속화 vs 파일 이동 mismatch | Medium | Medium | hydrate 시 각 파일 `exists()` 체크 → false면 큐에서 자동 제거. 사용자에게 토스트 안내 |
| YouTube URL이 비공개/지역 차단 | Medium | Medium | yt-dlp stderr 파싱 → 한국어 친절 메시지 매핑 ("이 영상은 비공개이거나 접근할 수 없어요") |
| 매우 긴 영상 (3시간+) → 추출 시간 폭증 | Low | Low | 사전 ffprobe로 길이 확인 → 30분+ 시 사용자 컨펌 다이얼로그 (선택, v1.1로 미룰 수 있음) |
| 1000개+ 큐 항목 시 렌더링 lag | Low | Low | virtual scrolling 미적용 (Phase 1 가정 < 100). 1000+ 도달 시 v1.1 fix |
| 사용자가 처리 중 항목 강제 삭제 | Medium | Medium | 처리 중 항목 삭제 시 subprocess cancel → cleanup → 큐 제거. (cancel_install 패턴 재사용) |

---

## 6. Impact Analysis

### 6.1 Changed Resources

| Resource | Type | Change |
|---|---|---|
| `src/pages/QueuePage.svelte` | Svelte Page | 빈 placeholder → DropZone + UrlInput + 큐 리스트 + 액션 바 |
| `src/components/queue/` (신규 디렉토리) | Components | DropZone / UrlInput / FileCard / EmptyState 4종 |
| `src/lib/stores.ts` | TS (확장) | `queueStore` writable + Tauri Store sync hooks |
| `src/lib/queue.ts` | TS (신규) | normalizeUrl / classifyFile / dedupCheck 헬퍼 |
| `src/lib/types.ts` | TS (확장) | `QueueItem`, `QueueStatus`, `ExtractProgress`, `DownloadProgress` |
| `src/lib/commands.ts` | TS (확장) | `extractAudio` / `downloadYoutube` invoke wrapper (Channel) |
| `src-tauri/src/commands/video.rs` | Rust | placeholder → ffprobe + ffmpeg subprocess + Channel + 임시 파일 출력 |
| `src-tauri/src/commands/youtube.rs` | Rust | placeholder → yt-dlp subprocess + 진행률 파싱 + Channel |
| `src-tauri/src/commands/common.rs` | Rust (확장) | `queue_tmp_dir(app)` 헬퍼 추가 |
| `src-tauri/src/lib.rs` | Rust | (no change — extract_audio / download_youtube 이미 등록됨) |
| `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/` | Runtime Asset | 신규 디렉토리. 임시 파일 영역. **uninstall 시 삭제 v1.2 app-lifecycle 이관** |
| `tauri-plugin-store` 사용 | Library | 이미 capabilities/lib.rs 등록됨, 추가 작업 X |

### 6.2 Future Consumers

queue-page 완료 후 의존하게 될 후속 피처:

| Future Feature | Operation | Reuses |
|---|---|---|
| **ProcessPage** (다음 피처) | navigateTo + 큐 IDs | queueStore + QueueItem type |
| **PlayerPage** | 분리 결과 → 재생 | queue-tmp 출력 wav (separate.rs가 입력으로 사용) |
| **v1.1 HistoryPage** | 완료된 큐 항목 → 히스토리 적재 | QueueItem.status='done' 트리거 |
| **v1.2 SettingsPage** | "임시 파일 정리" 버튼 | queue_tmp_dir 경로 |
| **v1.2 app-lifecycle** | uninstall 시 cleanup | %APPDATA% 통째 삭제 |

### 6.3 Verification

- [ ] `pnpm tauri dev`로 URL 입력 → 큐 추가 동작 확인
- [ ] 영상 파일 드래그 → 추출 → ready-to-separate 도달 확인
- [ ] 앱 재시작 → 큐 복원 확인
- [ ] 임시 파일 위치 정확성 확인 (`%APPDATA%/.../queue-tmp/`)

---

## 7. Architecture Considerations

### 7.1 Project Level

`Dynamic` 유지 (setup-page에서 확정). 변경 없음.

### 7.2 Key Architectural Decisions (Checkpoint 1+2 확정)

| # | Decision | Options | 선택 | 이유 |
|---|---|---|---|---|
| K1 | **큐 상태 영속화** | (A) 휘발 / (B) Tauri Store / (C) 하이브리드 | **(B) Tauri Store** | 앱 재시작 시 pending 큐 복원으로 사용자 작업 손실 방지. tauri-plugin-store 이미 등록됨. 단, 영속 대상은 `pending` 상태만 — in-progress/done/error는 휘발 (단순화) |
| K2 | **분리 시작 버튼** | (A) 전역 / (B) 항목별 / (C) 둘 다 | **(A) 전역** | 다중 선택 UI와 일관. 하단 액션 바 [▶ 분리 시작 (N)] 하나. UI 단순. |
| K3 | **중복 처리** | (A) 건너뜀 / (B) 모달 / (C) 무시 | **(A) 건너뜀 (기본값)** | ref UX_BEHAVIORS.md와 일치. 토스트로 안내만. 모달 다이얼로그는 v1.1로 미룸 |
| K4 | **다중 URL 지원** | (A) 싱글 / (B) 줄바꿈 / (C) 플레이리스트 | **(A) 싱글 URL만** | MVP 스코프. yt-dlp `--no-playlist` 강제. v1.1로 확장 가능 |
| K5 | **임시 파일 위치** | (A) %APPDATA%/queue-tmp/ / (B) %TEMP% / (C) ~/Desktop/tmp | **(A) %APPDATA%** | common::app_data_dir과 일관. 우리가 제어 가능. uninstall 삭제는 v1.2 app-lifecycle 이관 (사용자 승인) |
| K6 | **처리 중 큐 추가** | (A) 가능 / (B) 차단 | **(A) 가능** | UX 우수. 큐 상태 (`pending` / `in-progress` / `done` / `error`) 명확히 구분 |
| K7 | (Inherited from setup-page) | Architecture: Option C | **Option C** | common.rs 재사용, video.rs / youtube.rs 단일 파일씩, 후속 피처 동일 패턴 |

### 7.3 Data Model

```typescript
// src/lib/types.ts 추가분
export type QueueItemSource = "url" | "audio-file" | "video-file";
export type QueueItemStatus = "pending" | "downloading" | "extracting" | "ready-to-separate" | "in-progress" | "done" | "error";

export interface QueueItem {
  id: string;            // UUID v4
  source: QueueItemSource;
  label: string;         // 표시용: "소란 - 사랑한 마음엔" (URL이면 yt-dlp metadata, 파일이면 basename)
  origin: string;        // URL 또는 절대경로
  tmpPath?: string;      // 다운로드/추출 후 wav 경로
  progress: number;      // 0~100
  step?: string;         // "다운로드 중..." / "오디오 추출 중..." 등
  status: QueueItemStatus;
  errorDetail?: string;  // status='error' 시
  addedAt: string;       // ISO 8601
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
```

```rust
// src-tauri/src/commands/youtube.rs / video.rs
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub item_id: String,
    pub step: String,
    pub percent: u32,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractProgress {
    pub item_id: String,
    pub percent: u32,
}
```

### 7.4 Persistence Schema (Tauri Store)

```json
// %APPDATA%/com.rhinoty.mr-extractor/queue-store.json (tauri-plugin-store)
{
  "version": 1,
  "queue": [
    { "id": "...", "source": "url", "origin": "https://youtube.com/watch?v=abc", "label": "...", "status": "pending", "addedAt": "2026-04-29T..." },
    ...
  ]
}
```

영속화 정책:
- `status === 'pending'`인 항목만 저장
- write debounce 500ms (NFR Reliability)
- hydrate 시 각 항목의 `origin` 검증 (URL은 그대로, 파일은 `exists()` 체크)
- mismatch 항목은 자동 제거 + 토스트 안내

---

## 8. Convention Prerequisites

| Category | Current | To Define / Verify | Priority |
|---|---|---|---|
| **사용자 표시 문자열** | ref COMMANDS.md | "다운로드 중..." / "오디오 추출 중..." 한국어 별칭 매핑 | High |
| **확장자 분류** | ref FILE_FORMATS.md | AUDIO_EXTS / VIDEO_EXTS Set 그대로 사용 | High |
| **URL 정규화 규칙** | ref UX_BEHAVIORS.md | youtube.com / youtu.be / music.youtube.com 모두 v= 기준 통일 | High |
| **DownloadProgress / ExtractProgress 스키마** | missing | Channel payload 위 §7.3 정의 따름 | High |
| **임시 파일 명명 규칙** | missing | `{id}.wav` (오디오/추출 후) 또는 `{id}.{원본 ext}` (다운로드 직후) | Medium |
| **큐 영속화 키** | missing | tauri-plugin-store 키 = `"queue"` (단일 키, JSON 배열) | Medium |
| **에러 메시지 매핑** | setup-page와 동일 | translate_error 패턴 + youtube/ffmpeg 특화 (비공개 영상 / 지역 차단 / 코덱 미지원) | Medium |
| **다중 선택 UX** | ref UI.md | Ctrl/Shift+클릭 + bg-accent/20 하이라이트 | High |
| **토스트** | missing | `tauri-plugin-notification` 이미 등록됨, OS 알림 사용 OR Svelte 자체 토스트 (결정 필요 → Design 단계) | Medium |

---

## 9. Next Steps

1. [x] Checkpoint 1+2 완료 — 6 Key Decisions 확정
2. [ ] `/pdca design queue-page` → 3 architecture options 비교 후 선택 (Option C 권장 — setup-page 패턴 답습)
3. [ ] Design 확정 후 `/pdca do queue-page --scope phase-1` (UI shell only)
4. [ ] Phase 2 (video.rs), Phase 3 (youtube.rs + ProcessPage 라우팅) 세션 분할

### 9.1 Design 단계 필수 포함 체크리스트

- [ ] **Phase별 Interface Contract** — Phase 1 export, Phase 2/3 추가분
- [ ] **Phase 간 의존성 그래프** — Phase 1 stores → Phase 2 + 3 활용
- [ ] **SC ↔ Phase 매핑** — SC-1~13 각 phase 배분
- [ ] **중간 상태 무결성** — Phase 1 단독 완료 시 앱 정상 동작 (분리 시작은 placeholder)
- [ ] **Toast 라이브러리 결정** — Svelte 자체 vs OS notification (Plan §8 후속)
- [ ] **virtual scrolling 도입 시점** — Phase 1에서 미적용, v1.1+에서 결정
- [ ] **uninstall cleanup 책임 분리** — v1.2 app-lifecycle 명시
- [ ] **subprocess cancel 패턴** — setup::cancel_install + InstallHandle 재사용 가능 여부 검토 (queue-page용 별도 handle 필요할지)

### 9.2 후속 피처 연관

| 후속 피처 | 의존 | 인터페이스 |
|---|---|---|
| **ProcessPage** | queueStore + QueueItem | navigateTo("process", { ids }) |
| **PlayerPage** | tmpPath (wav) | separate 결과 + queue-tmp 정리 |
| **v1.1 HistoryPage** | QueueItem 'done' 항목 | queueStore subscribe |
| **v1.2 SettingsPage** | queue_tmp_dir | "임시 파일 정리" 버튼 |
| **v1.2 app-lifecycle** | %APPDATA%/queue-tmp/ | uninstall NSIS 스크립트 |

---

## 10. Key Decisions (Checkpoint 결과)

### 10.1 결정 요약 테이블

| # | 결정 항목 | 선택 | 사용자 코멘트 |
|---|---|---|---|
| K1 | 큐 상태 영속화 | **Tauri Store** | "영속화가 좋긴한데" (스코프 우려) → pending만 영속화, 휘발성 단순화로 절충 |
| K2 | 분리 시작 버튼 위치 | **전역** | (Recommended 그대로) |
| K3 | 중복 처리 기본값 | **건너뜀** | (Recommended 그대로) |
| K4 | 다중 URL 지원 | **싱글 URL만** | (Recommended 그대로) |
| K5 | 임시 파일 위치 | **%APPDATA%/queue-tmp/** | "앱 삭제 시 얘도 삭제 되게끔 해야겠지?" → v1.2 app-lifecycle 이관 명시 |
| K6 | 처리 중 큐 추가 | **가능** | (Recommended 그대로) |

### 10.2 파생 결정 (Design에서 확정)

- Toast UI 구현 방식 (Svelte 자체 컴포넌트 vs `tauri-plugin-notification`)
- subprocess cancel 패턴 (setup::cancel_install 재사용 vs queue-specific handle)
- queue-tmp orphan cleanup 시점 (앱 시작 시 vs 사용자 명시 액션 vs 양쪽)
- FileCard 미리보기 (썸네일? 파형 mini? — v1.1?)

---

## Version History

| Version | Date | Changes | Author |
|---|---|---|---|
| 0.1 | 2026-04-29 | Initial draft. Checkpoint 1+2 완료 (6 Key Decisions). Phase 1/2/3 분할. setup-page Foundation API 재사용 명시. SC 13건 정의. v1.2 app-lifecycle (uninstall cleanup) 이관 명시. | rhino-ty |
