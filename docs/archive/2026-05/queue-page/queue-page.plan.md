# queue-page Planning Document

> **Summary**: 사용자 메인 허브. YouTube URL 또는 로컬 파일(오디오/영상) 입력 → 큐에 적재 → 다중 선택 + [▶ 분리 시작] → ProcessPage 라우팅. setup-page Foundation API (common::*) 재사용.
>
> **Project**: MR Extractor
> **Version**: 0.6
> **Author**: rhino-ty
> **Date**: 2026-04-29
> **Status**: Confirmed v0.6 — Iteration 5 stability check fix (Phase 1 verification 모순 / label 정합성 / SC 매핑 / 6건). Plan 안정화 → Design 진입 가능.

---

## Executive Summary

| Perspective | Content |
|---|---|
| **Problem** | setup-page 후 사용자는 "무엇을 처리할지" 입력해야 함. 두 입력 경로(YouTube URL, 로컬 파일)가 분리되어 있고 영상 파일은 사전 오디오 추출이 필요. 큐 관리 + 다중 선택 + 처리 시작 + 개별 cancel 흐름이 없으면 사용자는 한 번에 한 개씩만 처리 가능. |
| **Solution** | QueuePage = DropZone + UrlInput + 큐 리스트 (FileCard×N, 라벨 + 길이) + 다중 선택 액션 바 + 자체 Toast. 영상은 `video.rs::extract_audio`, URL은 `youtube.rs::download_youtube` → ProcessPage 라우팅. 메타데이터는 placeholder 카드 즉시 등장 + 백그라운드 fetch 패턴. Tauri Store pending 영속화 + 단축키 (Delete/Ctrl+A/Ctrl+O/Escape). |
| **Function/UX Effect** | URL/파일 입력 → 1초 내 placeholder 카드 + 5초 내 metadata 갱신 → 다중 선택 → [▶ 분리 시작] → ProcessPage. 처리 중에도 추가 가능, 개별 cancel 가능, 앱 재시작해도 pending 복원. 모델은 `htdemucs_ft` 고정 (v1.1 ModelSelector 슬롯 reserve). |
| **Core Value** | "유튜브 URL → MR 한 방에" 킬러 피처의 사용자 진입점 + 배치 처리 게이트웨이 + setup-page Foundation API 14종 첫 재사용 검증. v1.1 (ModelSelector / HistoryPage) + v1.2 (SettingsPage / app-lifecycle) 인터페이스 (`addToQueue` / `isProcessing` / `queue_tmp_dir`)도 동시 노출. |

---

## Context Anchor

| Key | Value |
|---|---|
| **WHY** | 사용자는 URL 1개씩 또는 파일 1개씩 처리하는 워크플로우를 견디지 못함. 큐 + 배치 처리 + 영속화로 "여러 노래 한꺼번에 작업 후 자리 비움" 시나리오 지원. |
| **WHO** | setup-page 통과한 사용자 전원. URL 음원을 즐기는 일반인 + 로컬 음원 라이브러리 가진 음악 애호가. |
| **RISK** | ① yt-dlp stdout 포맷 변경 → 진행률 파싱 깨짐. ② 영상 파일 corrupt → ffmpeg hang. ③ 큐 영속화 동시 쓰기 race. ④ 임시 파일 공간 누수. ⑤ 큐 영속화 vs 사용자 옮긴 파일 경로 mismatch. ⑥ YouTube 비공개/지역 차단/코덱 미지원. ⑦ 매우 긴 영상 추출 시간 폭증. ⑧ 1000+ 큐 시 렌더링 lag. ⑨ 처리 중 강제 삭제 시 좀비 프로세스. ⑩ metadata 추출 시간이 "1초 추가" 제약 위반 가능. ⑪ yt-dlp 출력 위치 미강제 시 작업 디렉토리 오염. ⑫ ModelSelector 슬롯 미reserve 시 v1.1 도입 리팩터 비용. |
| **SUCCESS** | URL/파일 입력 → 1초 내 placeholder 카드 + 5초 내 metadata. 다중 선택 → [▶ 분리 시작] → ProcessPage. 앱 재시작 시 pending 복원. 처리 중 추가 + 개별 cancel + 좀비 0. 단축키 통합. |
| **SCOPE** | Phase 1 (~3h): UI shell + Toast + DropZone + UrlInput + 큐 stores + 다중 선택 + 단축키. Phase 2 (~3h): video.rs + ffprobe metadata + corrupt 검증. Phase 3 (~2h): youtube.rs + URL 정규화 + yt-dlp metadata + 친절 에러 + 개별 cancel + ProcessPage 라우팅. **합계 ~8h** (setup-page 동일). |

---

## 1. Overview

### 1.1 Purpose

setup-page 통과 후 사용자가 가장 많은 시간을 보낼 화면. URL/파일 입력을 받고 큐로 관리하며 분리 처리를 시작하는 "허브" 역할.

### 1.2 Background

setup-page에서 sidecar 바이너리 (ffmpeg/ffprobe/yt-dlp) + Embedded Python venv + demucs + 모델 모두 확보 완료. queue-page는 이 자산을 처음으로 **활용**하는 피처.

| 자원 | 출처 | queue-page에서 사용처 |
|---|---|---|
| ffmpeg sidecar | setup-page | 영상→오디오 추출 (video.rs) |
| ffprobe sidecar | setup-page | 영상 길이 사전 확인 (진행률 계산) + corrupt 검증 |
| yt-dlp sidecar | setup-page | YouTube 다운로드 + metadata fetch (youtube.rs) |
| `common::sidecar_dir` | setup-page Foundation | sidecar 경로 resolve (dev/prod fallback) |
| `common::app_data_dir` | setup-page Foundation | `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/` 위치 단일 출처 |
| `common::dir_size` | setup-page Foundation | queue-tmp 누수 감지 (24h orphan cleanup, NFR Storage) |
| `common::available_space_mb` | setup-page Foundation | Phase 2/3 다운로드 직전 디스크 여유 체크 (재사용) |
| `common::dev_log` | setup-page Foundation | dev 빌드 진단 로그 (setup.log 공유, prefix `queue:` 사용 — Convention §8) |
| `translate_error` 패턴 | setup-page (translate_error fn) | Phase 2/3 친절 에러 매핑 — `common.rs::§ 5 Error Translation`로 이전 후 ErrorContext enum 분기 (Convention §8) |
| Tauri Store plugin | 이미 capabilities 등록 | pending 큐 영속화 (`queue-store.json`, history.json과 별도 파일) |
| `tauri-plugin-dialog` | 이미 capabilities 등록 | Ctrl+O 파일 다이얼로그 (FR-21) |
| `tauri-plugin-process` `exit` | setup-page에서 사용 (close button) | 미사용 (queue-page는 프로세스 종료 책임 없음) |

### 1.3 Related Documents

- **선행 피처**: [setup-page.report.md](../../04-report/setup-page.report.md) — Foundation API 14종 export
- **참조 스펙**:
  - [docs/references/COMMANDS.md](../../references/COMMANDS.md) — youtube.rs / video.rs 시그니처. **DownloadProgress / ExtractProgress 페이로드 동기화 책임**: Phase 3 종료 시 본 Plan의 §7.3 Data Model 정의를 ref COMMANDS.md에 반영 (별도 docs commit) — Iteration 1 fix C4
  - [docs/references/UI.md](../../references/UI.md) — 다중 선택 UI 규칙 (Ctrl+클릭, Shift+클릭)
  - [docs/references/UX_BEHAVIORS.md](../../references/UX_BEHAVIORS.md) — 중복 처리 / URL 정규화 / 앱 종료 안전 처리 (`is_processing()`)
  - [docs/references/FILE_FORMATS.md](../../references/FILE_FORMATS.md) — DropZone 확장자 분류 (AUDIO_EXTS / VIDEO_EXTS)
  - [docs/references/SHORTCUTS.md](../../references/SHORTCUTS.md) — 큐/전역 단축키 (Delete / Ctrl+A / Ctrl+O / Escape)
  - [docs/references/MODEL_SELECTOR.md](../../references/MODEL_SELECTOR.md) — QueuePage 설정 섹션 위치 명시 (v1.1 슬롯 reserve)
  - [docs/references/HISTORY.md](../../references/HISTORY.md) — `source_type` 명명 통일 (queue ↔ history 일관성), "재처리 → 큐 추가" 인터페이스
- **로드맵**: [docs/ROADMAP.md](../../ROADMAP.md) v1 — `QueuePage` + `youtube.rs` + `video.rs`

---

## 2. Scope

### 2.1 In Scope

#### Phase 1 — UI Shell (Frontend only, ~3h)
- [ ] `src/pages/QueuePage.svelte` — 라우팅 진입점 + 단축키 listener (FR-21)
- [ ] `src/components/queue/DropZone.svelte` — 파일 드래그&드롭 (확장자 필터 + drag-over visual + Ctrl+O 파일 다이얼로그 fallback)
- [ ] `src/components/queue/UrlInput.svelte` — YouTube URL 입력 (정규화 + Enter + invalid URL inline error)
- [ ] `src/components/queue/FileCard.svelte` — 큐 항목 1개 (아이콘 + 라벨 + 길이 + 상태 + 진행률 + 처리 중 [✕] cancel 버튼)
- [ ] `src/components/queue/EmptyState.svelte` — 빈 큐 일러스트 + 안내
- [ ] `src/components/common/Toast.svelte` (신규, Convention §8) — 자체 구현, FR-11 안내 + Phase 2/3 에러 표시 공유
- [ ] `src/lib/stores.ts` 확장 — `queueStore` (Svelte writable) + Tauri Store sync (debounce 500ms, FR-08) + `isProcessing` derived (FR-20)
- [ ] `src/lib/queue.ts` (신규) — `normalizeUrl` / `classifyFile` / `isDuplicate` / `addToQueue` / `removeFromQueue` 헬퍼 (FR-14, v1.1 HistoryPage가 `addToQueue` 재사용)
- [ ] 다중 선택 (Ctrl+클릭, Shift+클릭, 전체 선택 Ctrl+A) — FR-03
- [ ] 하단 액션 바: [🗑 삭제 (N)] / [▶ 분리 시작 (N)] — Phase 2/3 도입 전엔 후자 placeholder
- [ ] 단축키 통합 (Delete / Ctrl+A / Ctrl+O / Escape) — FR-21
- [ ] ModelSelector 슬롯 영역 reserve — Design 단계에서 위치 결정 (상단 헤더 / 액션 바 옆 / DropZone 위 중 1개, §10.2 파생). 빈 div + comment "v1.1 ModelSelector 슬롯"
- [ ] [▶ 분리 시작] 버튼 = **disabled + tooltip** "Phase 2/3에서 사용 가능해요" (Phase 2/3 도입 전까지). alert 사용 X (Phase 1 단독 완료 조건과 일관)

#### Phase 2 — Local File Pipeline (~4h)
- [ ] `src-tauri/src/commands/video.rs::extract_audio` 본구현
  - ffprobe **단일 호출** (`ffprobe -v quiet -print_format json -show_format <file>`) → JSON에서 duration 추출 (FR-17 metadata + 진행률 계산 동시) + corrupt 검증 (duration=0 또는 stderr error → 즉시 실패)
  - ffmpeg `-i video -vn -acodec pcm_s16le -ar 44100 -ac 2 -y "{queue_tmp}/{id}.wav"`
  - stderr `time=HH:MM:SS` 파싱 → Channel emit (2초 단위)
  - **ffmpeg subprocess timeout 30분** (corrupt video 보호용, Risk 2). ※ Phase 3 yt-dlp 다운로드는 timeout 없음 (NFR Limits) — 두 timeout 정책 분리
- [ ] **ffprobe metadata 사전 추출** (FR-17): 길이 + duration_sec → FileCard 표시
- [ ] FileCard에 영상→오디오 추출 단계 시각화 (Plan §10.2 결정)
- [ ] 추출 완료 후 큐 항목 상태 → `ready-to-separate`
- [ ] 친절 에러: corrupt video → "이 파일을 읽을 수 없어요" (SC-15)

#### Phase 3 — YouTube Pipeline + Cancel + Routing (~2h)
- [ ] `src-tauri/src/commands/youtube.rs::download_youtube` 본구현
  - yt-dlp `--output "{queue_tmp}/{id}.%(ext)s" --no-playlist --no-mtime --no-warnings` (FR-15/16)
  - stdout 진행률 파싱 (yt-dlp `[download]` 라인 정규식 + indeterminate fallback)
  - 영상이면 즉시 extract_audio 체이닝 (FR-07)
- [ ] **yt-dlp metadata 사전 추출** (FR-17): `--skip-download --print "%(title)s\n%(duration)s"` (~3~5초, 백그라운드 fetch)
- [ ] `src/lib/queue.ts::normalizeUrl` — youtube/youtu.be/m.youtube.com/music.youtube.com → `youtube.com/watch?v={id}` (FR-14)
- [ ] **개별 cancel** (FR-18): `QueueHandle: Mutex<HashMap<String, u32>>` State + `cancel_queue_item(id)` Tauri command + Windows `taskkill /F /T /PID`
- [ ] 친절 에러 매핑 확장 (setup::translate_error 패턴): "비공개 영상" / "지역 차단" / "코덱 미지원" (SC-14)
- [ ] [▶ 분리 시작] 클릭 → 선택 큐 항목 IDs를 ProcessPage에 전달 → `navigateTo("process", { ids, model: "htdemucs_ft" })` (FR-05/19)
- [ ] 처리 중에도 큐 추가 허용 (FR-09)

### 2.2 Out of Scope

- `separate.rs` (demucs) → **ProcessPage** 책임
- `export.rs` → **PlayerPage** 책임
- **ModelSelector 본구현** → **v1.1**. queue-page Phase 1~3은 모델 = `htdemucs_ft` **고정값** 사용 (FR-19). 단, ref MODEL_SELECTOR.md가 "QueuePage 설정 섹션" 위치를 명시하므로 v1.1 도입 시 queue-page 레이아웃에 슬롯만 미리 확보 (Design 단계에서 영역 reserve)
- 처리 히스토리 → **v1.1 HistoryPage**. queue-page의 "재처리" 진입점(HISTORY 항목 → 큐 추가)은 v1.1에서 외부 인터페이스로 노출 — `addToQueue(item: QueueItem)` 함수로 export
- 중복 감지 모달 (선택지 다이얼로그) → **v1.1**, queue-page는 "건너뜀" 기본만 (FR-11)
- yt-dlp 플레이리스트 자동 펼침 → **v1.1+** (`--no-playlist` 강제)
- 다중 영상 URL 일괄 입력 → **v1.1+** (싱글 URL만)
- 임시 파일 정리 (앱 uninstall 시) → **v1.2 app-lifecycle** (UX_BEHAVIORS.md "앱 종료 안전 처리"와 함께)
- **앱 종료 안전 다이얼로그 (CloseRequested 인터셉트)** → **v1.1 또는 별도 피처**. queue-page는 `is_processing()` 데이터의 source만 제공 (queueStore에서 in-progress 항목 수 export). 다이얼로그 UI 자체는 별도 책임 (FR-20)
- BPM 감지 / EQ → **v2+**

---

## 3. Requirements

### 3.1 Functional Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| FR-01 | UrlInput에 URL 입력 + Enter → 정규화 + 중복 체크 → 1초 내 큐에 추가 | High | Pending |
| FR-02 | DropZone에 파일 드래그 → 확장자 분류 (audio/video/unknown) + 중복 체크 → 1초 내 큐 추가. **다중 파일 동시 드래그 시 N개 카드 동시 등장** (권장 §10.2 결정) | High | Pending |
| FR-03 | 큐 항목 클릭 = 선택 토글 (체크박스 미사용, **`bg-accent/20` 하이라이트만** — §10.2 결정으로 HISTORY 체크박스 패턴과 분리). Ctrl+클릭 = 개별 추가/제거. Shift+클릭 = 범위 선택. Ctrl+A = 전체 선택 (Windows 전용이므로 Cmd 표기 불필요) | High | Pending |
| FR-04 | [🗑 삭제 (N)] 버튼 = 선택된 항목 일괄 삭제. 처리 중인 항목은 **FR-18과 동일한 cancel 메커니즘 호출** 후 큐 제거 (`removeFromQueue(id)` 내부에서 status 검사 → cancel_queue_item invoke) | High | Pending |
| FR-05 | [▶ 분리 시작 (N)] 버튼 = 선택 항목 처리 큐 진입 + ProcessPage 라우팅 (URL 파라미터로 IDs 전달) | High | Pending |
| FR-06 | 영상 확장자(mp4/mkv/mov/avi/webm/wmv/flv/ts/m2ts) 항목은 video.rs::extract_audio 자동 호출 → wav 변환 후 ready-to-separate. FileCard에 "(영상에서 추출)" 표시 | High | Pending |
| FR-07 | YouTube URL 항목은 youtube.rs::download_youtube 자동 호출 (`--no-playlist` 강제). 영상이 다운로드되면 FR-06 체이닝 | High | Pending |
| FR-08 | 큐 상태(`pending` 항목만) Tauri Store에 영속화. 앱 재시작 시 hydrate. **파일 항목**(`sourceType === "file"\|"video"`)은 hydrate 시 `exists()` 체크 후 false면 자동 제거 + 토스트 안내. **URL 항목**(`sourceType === "youtube"`)은 exists 체크 X (네트워크 영속성 가정) | High | Pending |
| FR-09 | 처리 중에도 DropZone/UrlInput 활성. 새 항목은 `pending` 상태로 큐 끝에 적재. 사용자 액션 차단 없음 | Medium | Pending |
| FR-10 | 임시 파일 출력: `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{id}.{ext}` ({id}는 큐 항목 UUID, {ext}는 wav 또는 원본 video ext) | High | Pending |
| FR-11 | 중복 감지 기본값 = **건너뜀** (URL 정규화 후 + 파일 절대경로 기준). 토스트로 "이미 큐에 있어요" 안내. 모달 X | Medium | Pending |
| FR-12 | 빈 큐 상태: EmptyState 일러스트 + DropZone 안내 ("URL을 붙여넣거나 파일을 끌어다 놓으세요"). FileCard 영역 hide | Medium | Pending |
| FR-13 | 처리 중 항목 시각화: ⏳ 아이콘 + 진행률 바 (% + step 텍스트, "다운로드 중..." / "오디오 추출 중...") | Medium | Pending |
| FR-14 | URL 정규화 (ref UX_BEHAVIORS.md): youtube.com / youtu.be / m.youtube.com / music.youtube.com → 모두 `youtube.com/watch?v={id}` 통일 | High | Pending |
| FR-15 | yt-dlp는 항상 `--no-playlist` 옵션 강제. 플레이리스트 URL 입력 시에도 첫 영상만 추가 | High | Pending |
| FR-16 | yt-dlp 출력 인자 `--output {queue_tmp_dir}/{id}.%(ext)s --no-playlist --no-mtime`. 다운로드 위치를 우리가 제어 (FR-10과 일치). yt-dlp 기본 경로(작업 디렉토리) 사용 금지 | High | Pending |
| FR-17 | 메타데이터 표시: URL은 yt-dlp `--skip-download --print "%(title)s\n%(duration)s"`로 사전 추출 (~2~5초). 파일은 ffprobe로 길이 추출 (~수백ms). FileCard에 라벨 + 길이 표시. 추출 실패 시 fallback (URL → ID, 파일 → basename) | Medium | Pending |
| FR-18 | 개별 cancel: 처리 중 (downloading/extracting) 항목의 [✕] 버튼 → subprocess 트리 kill + 큐 항목 제거 + 임시 파일 cleanup. setup::cancel_install의 InstallHandle 패턴 응용 | Medium | Pending |
| FR-19 | 모델 = `htdemucs_ft` **고정**. ProcessPage로 라우팅 시 model 파라미터 전달. v1.1 ModelSelector 도입 전까지 사용자 선택 UI 없음 | High | Pending |
| FR-20 | `queueStore.derived('isProcessing')` **frontend** export — 임의 페이지/Svelte 컴포넌트가 "처리 중인지" 판정 가능. UX_BEHAVIORS.md `is_processing()` **Rust 함수**는 별도 피처(v1.1 앱 종료 안전 다이얼로그)에서 구현 — 큐 데이터를 IPC로 frontend ↔ Rust 양방향 sync 또는 Tauri event emit. queue-page는 frontend store 노출까지만 책임 | Medium | Pending |
| FR-21 | 단축키 통합 (ref SHORTCUTS.md §전역 + 큐): `Delete` = 선택 삭제 (queue-page active 시), `Ctrl+A` = 전체 선택 (queue-page active 시), `Ctrl+O` = 파일 다이얼로그 (**전역 단축키 — 다른 페이지 active 시에도 동작, 자동으로 navigateTo("queue") 후 다이얼로그**), `Escape` = 모달/도움말 닫기 (Phase 1에는 모달 없음, Phase 3 [상세] 토글 닫기) | Medium | Pending |

### 3.2 Non-Functional Requirements

| Category | Criteria | Measurement |
|---|---|---|
| Performance | URL 입력/파일 드래그 → **placeholder 카드 등장** < 1초 (metadata는 SC-19에서 별도 측정) | console.time, FR-01/02 |
| Performance | 영상→오디오 추출 진행률 < 2초 단위 갱신 | Channel emit 빈도 로그 |
| Performance | 1000개 큐 항목 시 다중 선택 응답 < 100ms | 가상 스크롤 미적용 (1000 미만 가정), 필요 시 v1.1+ |
| Performance | 개별 cancel 클릭 → 큐 카드 제거 < 500ms (subprocess kill은 비동기 OK) | FR-18 |
| Performance | yt-dlp metadata fetch (`--skip-download`) < 5초. 실패 시 fallback 라벨 (FR-17) | console.time |
| Reliability | 큐 영속화 충돌 방지: write 큐 debounce 500ms | 동시 추가 5건 stress test |
| Reliability | 처리 실패 시 임시 파일 cleanup (다음 앱 시작 시 orphan 정리) | startup 시 queue-tmp/ scan, 24h grace |
| Reliability | invalid URL 입력 시 즉시 inline error, 큐 추가 X | UrlInput 검증 |
| Concurrency | queue-page는 큐 적재 + metadata fetch까지만 책임. 동시 처리 항목 수 제한은 ProcessPage 책임 (GPU memory 제약, ref CLAUDE.md) | Phase 3 라우팅 시 인자 분리 |
| Memory | 1000 큐 항목 메모리 ≲ 5MB (Tauri Store JSON ≲ 50KB) | 추정 |
| Storage | 임시 파일 누수 < 1GB (정상 운영 시) | startup orphan cleanup |
| Storage | 큐 영속화 파일 (queue-store.json) 크기 < 100KB (1000 항목 가정) | 측정 |
| UX | 다크 테마 전용 (`--color-*` CSS 변수 준수) | 코드 리뷰 |
| UX | DropZone drag-over 시각 피드백 < 100ms | FR-02 |
| Error Clarity | 다운로드 실패 시 한국어 메시지 + 원본 stderr [상세] 토글 | translate_error 패턴 setup-page와 동일 |
| Tech Jargon | UI 본문에 yt-dlp / ffmpeg / demucs / pip / torch 노출 0건 | grep 검증, SC-11 |
| Limits | 큐 hard limit = **없음** (사용자 디스크 책임). 1000+ 도달 시 가상 스크롤 v1.1+에서 도입 권장 | 운영 가이드 |
| Limits | 다운로드 timeout = **없음** (yt-dlp 자체 timeout만 의존, 매우 긴 영상 허용). 대신 사용자가 [✕] cancel로 제어 (FR-18) | Risk 7 |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] **SC-1**: URL 입력 후 1초 이내 큐에 **placeholder 카드** 등장 (라벨 = URL 자체, 길이 미상) (FR-01)
- [ ] **SC-2**: 파일 드래그 후 1초 이내 큐에 카드 등장. 오디오 파일은 ffprobe 길이 추출 후 즉시 라벨 갱신 (~수백 ms) (FR-02)
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
- [ ] **SC-14**: yt-dlp 실패(비공개/지역차단/코덱) → 한국어 친절 메시지 + raw stderr [상세] 토글 (FR-07 + Risk 6)
- [ ] **SC-15**: corrupt video 드래그 (ffprobe 길이=0 또는 stderr error) → 즉시 실패 + "이 파일을 읽을 수 없어요" 안내 (FR-06 + Risk 2)
- [ ] **SC-16**: yt-dlp 출력이 정확히 `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{uuid}.{ext}` 패턴 — 작업 디렉토리에 파일 생성 0건 (FR-16)
- [ ] **SC-17**: 처리 중 항목의 [✕] 클릭 → subprocess 종료 + 좀비 프로세스 0 + 임시 파일 cleanup 확인 (FR-18)
- [ ] **SC-18**: queue-page active 시 `Delete` 키 = **선택된 N개 항목 일괄 삭제** (FR-04와 동일 결과), `Ctrl+A` = 전체 선택. 임의 페이지에서 `Ctrl+O` = navigateTo("queue") + 파일 다이얼로그 (FR-21)
- [ ] **SC-19**: URL 큐 추가 후 백그라운드 yt-dlp metadata fetch 5초 이내 완료 → FileCard 라벨 "다운로드 준비 중..." → "{영상 제목} (3:42)"로 갱신. 5초 초과 시 fallback 라벨 (URL 자체) 유지 (FR-17)
- [ ] **SC-20**: 처리 중 항목 (status `downloading` 또는 `extracting`) FileCard에 ⏳ 아이콘 + 진행률 바 + step 텍스트 표시 (FR-13)
- [ ] **SC-21**: 코드 grep — `separate.rs` / ProcessPage 진입 인자에 `model` = "htdemucs_ft" 리터럴만 존재 (모델 선택 UI 없음, FR-19)

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
| 사용자가 처리 중 항목 강제 삭제 | Medium | Medium | 처리 중 항목 삭제 시 subprocess cancel → cleanup → 큐 제거. (cancel_install 패턴 재사용, FR-18) |
| metadata 추출 시간 (yt-dlp `--skip-download` ~3~5초)이 FR-01 "1초 추가" 제약 위반 | Medium | High | 2-step 패턴: (1) 즉시 큐에 placeholder 카드 등장 (URL 자체를 라벨로) + (2) 백그라운드 metadata fetch → 라벨 갱신 (FR-17, SC-19). 1초 제약은 카드 등장까지만 적용 |
| yt-dlp 출력 위치 미강제 → 작업 디렉토리에 파일 생성됨 | High | Low | `--output {queue_tmp_dir}/{id}.%(ext)s` 인자 강제 (FR-16). cwd 의존하지 않음. SC-16에서 검증 |
| ModelSelector 슬롯 없이 Phase 1 UI 만들면 v1.1 도입 시 큰 리팩터 | Low | Medium | Design §UI 영역에 "ModelSelector 슬롯" placeholder 미리 reserve. 본구현은 v1.1로 미루지만 div 영역만 잡아둠 (Plan FR-19 + §2.2 명시) |

---

## 6. Impact Analysis

### 6.1 Changed Resources

| Resource | Type | Change |
|---|---|---|
| `src/pages/QueuePage.svelte` | Svelte Page | 빈 placeholder → DropZone + UrlInput + 큐 리스트 + 액션 바 + 단축키 listener + ModelSelector 슬롯 (reserve) |
| `src/components/queue/` (신규 디렉토리) | Components | DropZone / UrlInput / FileCard / EmptyState 4종 |
| `src/components/common/Toast.svelte` | Component (신규) | Svelte 5 runes 기반 자체 토스트. FR-11 안내 + Phase 2/3 에러 노출 공유. 다른 페이지에서도 재사용 |
| `src/lib/stores.ts` | TS (확장) | `queueStore` writable + Tauri Store sync (debounce 500ms) + `isProcessing` derived (FR-20) + `toastStore`. **`navigateTo` 시그니처 확장**: 기존 `navigateTo(page)` → `navigateTo(page, payload?)`. 페이지별 페이로드 전달 (queue → process: `{ ids: string[], model: "htdemucs_ft" }`). setup-page navigateTo("queue") 호출은 그대로 동작 (payload optional) |
| `src/lib/queue.ts` | TS (신규) | `normalizeUrl` / `classifyFile` / `isDuplicate` / `addToQueue` / `removeFromQueue` 헬퍼. v1.1 HistoryPage가 `addToQueue` 재사용 |
| `src/lib/types.ts` | TS (확장) | `QueueItem`, `QueueItemStatus` (8 variants), `QueueSourceType` (HISTORY 통일), `ExtractProgress`, `DownloadProgress` |
| `src/lib/commands.ts` | TS (확장) | `extractAudio` / `downloadYoutube` / `cancelQueueItem` / `fetchYoutubeMetadata` invoke wrappers (Channel) |
| `src-tauri/src/commands/video.rs` | Rust | placeholder → ffprobe metadata + ffmpeg subprocess + Channel + 임시 파일 출력 + corrupt 검증 |
| `src-tauri/src/commands/youtube.rs` | Rust | placeholder → yt-dlp subprocess (download + `--skip-download` metadata) + 진행률 파싱 + Channel + friendly 에러 매핑 |
| `src-tauri/src/commands/common.rs` | Rust (확장) | **`queue_tmp_dir(app)` 헬퍼** 추가 (= app_data_dir/queue-tmp/) |
| `src-tauri/src/lib.rs` | Rust (확장) | **`QueueHandle: Mutex<HashMap<String, u32>>` State 등록** (FR-18) + `cancel_queue_item` Tauri command 추가 + `extract_audio` / `download_youtube` / `fetch_youtube_metadata` handler 등록 |
| `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/` | Runtime Asset | 신규 디렉토리. 임시 파일 영역. **uninstall 시 삭제 v1.2 app-lifecycle 이관** |
| `%APPDATA%/com.rhinoty.mr-extractor/queue-store.json` | Runtime Asset | tauri-plugin-store 영속화 파일. version 1 schema, 1000 항목 가정 < 100KB |
| `tauri-plugin-store` 사용 | Library | 이미 capabilities/lib.rs 등록됨, 추가 작업 X |
| `tauri-plugin-dialog` 사용 (Ctrl+O) | Library | `dialog:allow-open` setup-page에서 등록됨 (capabilities/default.json:38), 추가 작업 X |

### 6.2 Future Consumers

queue-page 완료 후 의존하게 될 후속 피처:

| Future Feature | Operation | Reuses (Plan §6.1 export) |
|---|---|---|
| **ProcessPage** (다음 피처) | navigateTo + 큐 IDs + model | `queueStore` 구독 + `QueueItem` type + `cancelQueueItem` (재처리 시) |
| **PlayerPage** | 분리 결과 → 재생 | queue-tmp 출력 wav (separate.rs가 입력으로 사용) |
| **v1.1 ModelSelector** | QueuePage 설정 영역 슬롯 채우기 | queue-page reserve 영역 (Phase 1에서 빈 div) + `setModelForQueueItem(id, model)` 인터페이스 (queue.ts 추가) |
| **v1.1 HistoryPage** | 완료된 큐 항목 → 히스토리 적재 / "재처리" → 큐 추가 | `QueueItem.status='done'` 트리거 + `addToQueue(item)` 함수 (queue.ts export) |
| **v1.1 앱 종료 안전 다이얼로그** | CloseRequested 시 처리 중 여부 확인 | `queueStore.isProcessing` derived (FR-20) |
| **v1.2 SettingsPage** | "임시 파일 정리" 버튼 + 저장 공간 표시 | `common::queue_tmp_dir` + `common::dir_size` (setup-page Foundation) |
| **v1.2 app-lifecycle** | uninstall 시 cleanup | `%APPDATA%/com.rhinoty.mr-extractor/` 통째 삭제 (queue-tmp + queue-store.json 포함) |

### 6.3 Verification (Phase별)

**Phase 1**:
- [ ] `pnpm tauri dev` → URL/파일 추가 → placeholder 카드 등장 (1초 이내)
- [ ] 다중 선택 (Ctrl+클릭, Shift+클릭, Ctrl+A) + Delete 키 정상
- [ ] 앱 재시작 → pending 큐 복원, 사라진 파일 자동 제거 확인
- [ ] 중복 입력 시 토스트 안내, 큐 그대로 (FR-11)
- [ ] [▶ 분리 시작] 버튼이 **disabled 상태** + hover 시 tooltip "Phase 2/3에서 사용 가능해요" (FR-05 / Phase 1 In Scope §2.1과 일관, alert 호출 X)

**Phase 2**:
- [ ] 영상 파일 (.mp4) 드래그 → ffprobe metadata 추출 (길이 표시) → ffmpeg 추출 진행률 → ready-to-separate 도달
- [ ] corrupt video 드래그 → 즉시 실패 + "이 파일을 읽을 수 없어요" 토스트
- [ ] 임시 파일 위치 정확성 (`%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{uuid}.wav`)
- [ ] 추출 도중 [✕] cancel → subprocess 종료 + 임시 파일 cleanup

**Phase 3**:
- [ ] YouTube URL (다양한 형식) → 정규화 → metadata fetch (제목 + 길이) → 다운로드 진행률 → 영상이면 Phase 2 체이닝
- [ ] 비공개/지역차단 영상 → 한국어 친절 메시지 + [상세] 토글
- [ ] [▶ 분리 시작] → ProcessPage 라우팅 (placeholder OK, 실제 ProcessPage는 다음 피처)
- [ ] 다운로드 도중 [✕] cancel → tree kill + 좀비 0 (작업관리자 확인)

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
| K8 | **subprocess Handle 일반화** | (A) setup InstallHandle 그대로 + queue QueueHandle 별도 / (B) common::ProcessHandle로 일반화 후 두 use case 통합 | **Design 단계 결정** | (A)는 단순하나 중복. (B)는 일반성 크나 setup-page 리팩터 필요. Phase 3 진입 직전 Design 평가 권장 — Plan §10.2 파생 결정 |

### 7.3 Data Model

> **HISTORY.md 일관성 (Iteration 1 fix C1)**: `source_type`은 HISTORY JSON의 `"youtube" | "file" | "video"` 명명을 그대로 따름. queue-page는 "file" 단일 라벨 대신 audio/video 분리해서 내부적으로만 구분 (FileCard 아이콘 차이) — 외부 노출 source 필드는 HISTORY와 동일.

```typescript
// src/lib/types.ts 추가분 — HISTORY.md JSON 스키마와 source_type 일치
export type QueueSourceType = "youtube" | "file" | "video";  // HISTORY.md 호환
export type QueueItemStatus =
  | "pending"            // 큐에 추가됨, 처리 대기
  | "fetching-metadata"  // yt-dlp --skip-download 또는 ffprobe 실행 중 (FR-17)
  | "downloading"        // yt-dlp 다운로드 중 (URL only)
  | "extracting"         // ffmpeg 오디오 추출 중 (영상 only)
  | "ready-to-separate"  // 추출/다운로드 완료, ProcessPage 진입 가능
  | "in-progress"        // ProcessPage가 separate.rs 실행 중 (queue-page는 read-only)
  | "done"               // 완료 (HISTORY로 이관됨)
  | "error";             // 실패

export interface QueueItem {
  id: string;            // crypto.randomUUID() (UUID v4)
  sourceType: QueueSourceType;
  source: string;        // URL 또는 절대경로 (HISTORY.md `source` 필드와 동일 명명)
  label: string;         // 항상 사용자 표시용. 추가 직후 placeholder ("다운로드 준비 중..." for URL / basename for file) → metadata 추출 후 실제 라벨 ("{title} ({mm:ss})" for URL / "{basename} ({mm:ss})" for file)로 갱신 (SC-19와 일관)
  durationSec?: number;  // 영상/오디오 길이 (FR-17 metadata 결과). 미상 시 undefined
  tmpPath?: string;      // 다운로드/추출 후 wav 경로 (status='ready-to-separate' 이후)
  progress: number;      // 0~100
  step?: string;         // "다운로드 중..." / "오디오 추출 중..." 등 (FR-13)
  status: QueueItemStatus;
  errorDetail?: string;  // status='error' 시
  addedAt: string;       // ISO 8601. 정렬 기준: 큐 리스트 표시 순서 (오래된 → 최신, top-down)
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
    { "id": "...", "sourceType": "youtube", "source": "https://youtube.com/watch?v=abc", "label": "...", "durationSec": 234, "status": "pending", "addedAt": "2026-04-29T..." },
    ...
  ]
}
```

**영속화 정책**:
- `status === 'pending'`인 항목만 저장 (in-progress / done / error는 휘발)
- write debounce 500ms (NFR Reliability)
- hydrate 시 각 항목의 `source` 검증 (URL은 그대로, 파일은 `exists()` 체크)
- mismatch 항목은 자동 제거 + 토스트 안내

**Schema Migration 정책**:
- `version` 필드 필수. v1 → v2 미래 변경 시 hydrate 시점에 미들웨어 함수 (`migrateQueueStore`) 거치도록
- 알 수 없는 version (down-migration) 시 큐 전체 폐기 + 사용자에게 경고 토스트 ("저장된 큐를 불러올 수 없어요")
- 향후 schema 변경 시 v0.x → v1 → v2 등 단계별 migration 작성. v1.1 ModelSelector 도입 시 `model` 필드 추가될 가능성 높음

---

## 8. Convention Prerequisites

| Category | Current | To Define / Verify | Priority |
|---|---|---|---|
| **사용자 표시 문자열** | ref COMMANDS.md | "다운로드 중..." / "오디오 추출 중..." 한국어 별칭 매핑 | High |
| **확장자 분류** | ref FILE_FORMATS.md | AUDIO_EXTS / VIDEO_EXTS Set 그대로 사용 (`src/lib/queue.ts::classifyFile` 위치) | High |
| **URL 정규화 규칙** | ref UX_BEHAVIORS.md | youtube.com / youtu.be / m.youtube.com / music.youtube.com 모두 `v=` 기준 통일. 위치 `src/lib/queue.ts::normalizeUrl` | High |
| **DownloadProgress / ExtractProgress 스키마** | missing | Channel payload 위 §7.3 정의 따름. ref COMMANDS.md 동기화 필요 (Iteration 1 fix C4) | High |
| **임시 파일 명명 규칙** | missing | yt-dlp 다운로드: `{id}.%(ext)s` (yt-dlp가 ext 결정). ffmpeg 추출 후: `{id}.wav` (44100Hz stereo pcm_s16le 강제). 모두 `common::queue_tmp_dir(app)` 하위 | Medium |
| **큐 영속화 키** | missing | `tauri-plugin-store` 키 = `"queue"` (단일 키, JSON 배열). 영속 대상은 `status === "pending"` only | Medium |
| **에러 메시지 매핑** | setup-page와 동일 | `translate_error` 패턴 확장. youtube/ffmpeg 특화: "비공개 영상" / "지역 차단" / "코덱 미지원" / "파일을 읽을 수 없어요". **위치 결정 (§10.2 추가)**: `src-tauri/src/commands/common.rs`에 `§ 5 Error Translation` 섹션 신규 + `pub fn translate_error(raw: &str, ctx: ErrorContext)`. setup.rs 기존 함수도 이리 이전 (Phase 2 진입 전 리팩터, **breaking change 없음** — 시그니처 동일하지만 위치만 변경, setup의 호출부도 같이 수정). youtube/video는 ctx만 다르게 호출 | High |
| **dev_log 공유** | setup-page에서 정의 | queue-page도 같은 `setup.log`에 적재. 큐 관련 로그는 `prefix queue:` 사용 — 예: `dev_log(app, "queue:download_youtube({id}): start")`. 향후 페이지별 로그 분리 필요 시 v1.2+에서 결정 | Medium |
| **다중 선택 UX** | ref UI.md (체크박스 X) / ref HISTORY.md (체크박스 ☐/☑) — **상충** | **결정**: queue-page는 체크박스 **미사용**, `bg-accent/20` 하이라이트만. 사유: ① 처리 진행률 + 길이 + 라벨 등 정보가 풍부해 체크박스 추가 시 시각 노이즈, ② UI.md 명시와 일치, ③ HISTORY는 별도 피처로 자율 결정 | High |
| **단축키** | ref SHORTCUTS.md | `Delete` / `Ctrl+A` / `Ctrl+O` / `Escape` 처리 (FR-21). 페이지 로컬 `keydown` listener | Medium |
| **Toast UI** | missing | **결정 (§10.2 + §6.1 일관)**: (c) Svelte 5 runes 자체 구현 — `src/components/common/Toast.svelte` (~50 lines). 외부 의존 0. tauri-plugin-notification은 OS 알림 (장시간 처리 완료 알림 v1.1)에만 사용. Phase 1에서 본구현 | High |
| **UUID 생성** | missing | `crypto.randomUUID()` (브라우저 표준, Tauri webview 지원). 서버 측은 `uuid` crate 사용 안 함 — 모든 ID는 frontend에서 생성 후 Rust로 전달 | Medium |
| **DropZone visual feedback** | missing | drag-over 시 `border-accent` + `bg-accent/10`. 응답 < 100ms (NFR Performance) | Medium |
| **yt-dlp 인자** | missing | 항상 `--output "{tmp}/{id}.%(ext)s" --no-playlist --no-mtime --no-warnings` (FR-15/16/17). metadata 사전 fetch는 `--skip-download --print "%(title)s\n%(duration)s"` | High |
| **ffmpeg 인자** | ref CLAUDE.md | `ffmpeg -i {video} -vn -acodec pcm_s16le -ar 44100 -ac 2 -y "{tmp}/{id}.wav"`. 진행률은 stderr `time=HH:MM:SS` 파싱 (CLAUDE.md "Do Not"의 ffprobe 사전 확인 필수 준수) | High |

---

## 9. Next Steps

1. [x] Checkpoint 1+2 완료 — 6 Key Decisions 확정
2. [ ] `/pdca design queue-page` → 3 architecture options 비교 후 선택 (Option C 권장 — setup-page 패턴 답습)
3. [ ] Design 확정 후 `/pdca do queue-page --scope phase-1` (UI shell only)
4. [ ] Phase 2 (video.rs), Phase 3 (youtube.rs + ProcessPage 라우팅) 세션 분할

### 9.1 Design 단계 필수 포함 체크리스트

**Plan v0.6 시점에서 이미 결정됨 (✅)**:
- [x] **Toast UI**: Svelte 5 runes 자체 구현 (Convention §8, §10.2)
- [x] **HISTORY 체크박스 vs queue 하이라이트**: 하이라이트만 (Convention §8 다중 선택)
- [x] **uninstall cleanup 책임**: v1.2 app-lifecycle 이관 (§2.2)
- [x] **다중 파일 동시 드래그**: N개 동시 추가 (§10.2)
- [x] **EmptyState 형식**: 이모지 + 1줄 안내 (SVG는 v1.1+, §10.2)
- [x] **translate_error 위치**: common.rs §5 Error Translation (§10.2)
- [x] **dev_log 공유 정책**: setup.log + queue prefix (Convention §8)
- [x] **Ctrl+O 전역 동작**: navigateTo("queue") 후 다이얼로그 (FR-21)

**Design에서 결정 (Open)**:
- [ ] **Phase별 Interface Contract** — Phase 1 export 함수/타입, Phase 2/3 추가분
- [ ] **Phase 간 의존성 그래프** — Phase 1 stores → Phase 2 + 3 활용
- [ ] **SC ↔ Phase 매핑** — SC-1~21 (총 21건) 각 phase 배분
- [ ] **중간 상태 무결성** — Phase 1 단독 완료 시 앱 정상 동작 (분리 시작은 disabled+tooltip)
- [ ] **subprocess Handle 일반화** (K8) — InstallHandle 별도 vs ProcessHandle 통합. Phase 3 진입 전 평가
- [ ] **ModelSelector 슬롯 위치** (§10.2 파생) — 상단 헤더 / 액션 바 옆 / DropZone 위 중 1개
- [ ] **queue-tmp orphan cleanup 시점** (§10.2 파생) — 앱 시작 시 자동 (24h grace 권장) 확정
- [ ] **EmptyState 문구 final pick** (§10.2 후보 2종 중)
- [ ] **virtual scrolling 도입 시점** — Phase 1에서 미적용, v1.1+에서 결정 명시

### 9.2 후속 피처 연관

→ **§6.2 Future Consumers 표 참조** (7개 항목, 인터페이스 상세). v0.3에서 통합됨.
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

### 10.2 파생 결정 (Plan v0.4에서 확정 + Design 단계 추가 결정)

**Plan v0.4 확정**:
- ✅ **Toast UI**: Svelte 5 runes 자체 구현 (외부 의존 0, ~50 lines)
- ✅ **다중 선택**: 체크박스 미사용, `bg-accent/20` 하이라이트만 (HISTORY와 분리, UI.md 일치)
- ✅ **drag&drop 다중 파일**: N개 동시 추가 (단일 추가 보다 다중 선택 패턴과 일관)
- ✅ **translate_error 위치**: `common.rs::§ 5 Error Translation` 섹션 + `ErrorContext` enum (Setup / YoutubeDownload / VideoExtract). setup-page 기존 함수도 이리 이전 (Phase 2 진입 전 리팩터)
- ✅ **EmptyState 일러스트**: 이모지 + 한국어 안내 1줄. SVG는 v1.1+ (디자인 시스템 도입 시). 후보 문구 (Design에서 final pick): (a) "🎵 음원을 추가해보세요\n\n🔗 URL 또는 파일을 끌어다 놓으세요" / (b) "🎵 큐가 비었어요\n\nYouTube URL을 붙여넣거나 파일을 드래그하세요"
- ✅ **Ctrl+O 전역 처리**: 다른 페이지 active 시 자동 navigateTo("queue") 후 다이얼로그 (FR-21)

**Design에서 확정**:
- **subprocess cancel 패턴 (FR-18)**: setup::cancel_install + InstallHandle 재사용 vs queue-specific `QueueHandle: Mutex<HashMap<String, u32>>` (item id → child PID 매핑). **권장 후자** — 동시 여러 항목 처리 가능성 대비
- **queue-tmp orphan cleanup 시점**: 앱 시작 시 자동 (큐 hydrate 후 비교) vs 사용자 명시 액션 (v1.2 SettingsPage). **권장 전자** — 24h grace 후 orphan 자동 삭제
- **FileCard 미리보기**: 썸네일/파형 mini → v1.1. queue-page는 텍스트 라벨 + 길이만 (FR-17)
- **ModelSelector 슬롯 디자인**: queue-page UI 어디에 reserve할지 (상단 헤더 vs 액션 바 옆 vs DropZone 위) — Design 단계 결정
- **InstallHandle 리팩터링 (선택)**: setup-page InstallHandle을 common::ProcessHandle로 일반화하여 QueueHandle과 통합? Design에서 비용/효용 평가

### 10.3 Phase 시간 추정 (setup-page 비례 환산)

| Phase | 예상 시간 | 구현 범위 | 관련 FR / SC |
|---|---|---|---|
| **Phase 1 — UI Shell** | ~3h | DropZone + UrlInput + queueStore (Tauri Store sync) + 다중 선택 + 삭제 + EmptyState + Toast 컴포넌트 + 단축키 + ModelSelector 슬롯 reserve | FR-01~04, FR-08, FR-09, FR-11~14, FR-19~21, SC-1~3, SC-7, SC-8, SC-12, SC-13, SC-18 |
| **Phase 2 — Local File Pipeline** | ~3h | video.rs::extract_audio (ffprobe + ffmpeg + Channel ~1h) + ffprobe metadata 추출 (~30min) + 추출 진행률 UI (~30min) + corrupt 검증 + 친절 에러 + subprocess timeout (~1h 합산) | FR-06, FR-10, FR-13, FR-17, SC-4, SC-9, SC-15 |
| **Phase 3 — YouTube + Routing + Cancel** | ~2h | youtube.rs::download_youtube + URL 정규화 + yt-dlp metadata 추출 + ProcessPage 라우팅 + 친절 에러 매핑 + 개별 cancel (QueueHandle) | FR-05, FR-07, FR-15~18, SC-5, SC-14, SC-16, SC-17, SC-19 |

**합계 ~8h** (setup-page와 동일 규모)

### 10.4 Phase 의존성 그래프

```
Phase 1 (UI Shell + queueStore)
  │
  │ exports: queueStore + QueueItem 타입 + addToQueue/removeFromQueue 함수 + Toast
  │ exports: derived isProcessing (FR-20)
  │
  ├──────────────┬─────────────┐
  │              │             │
  ▼              ▼             │
Phase 2          Phase 3       │
(extract)        (download +   │
  │              routing)      │
  │              │             │
  │ uses:        │ uses:       │
  │ queueStore   │ queueStore  │
  │ + Channel    │ + Channel   │
  │ + tmpPath    │ + tmpPath   │
  │              │             │
  │ exports:     │ exports:    │
  │ extractAudio │ downloadYt  │
  │              │ + nav       │
  └──────┬───────┘             │
         ▼                     │
  후속 피처 (ProcessPage — 별도 PDCA 사이클)
  │
  └─ uses: queueStore + ready-to-separate 항목들 + cancelQueueItem 인터페이스
```

**Phase 1 단독 완료 조건**:
- `pnpm tauri dev` 실행 시 QueuePage 진입 OK
- URL/파일 추가 → 큐에 placeholder 카드 등장 (FR-17 metadata fetch는 Phase 2/3에서 활성)
- 다중 선택 + 삭제 동작
- 앱 재시작 시 pending 항목 복원
- [▶ 분리 시작] 버튼은 **disabled + tooltip "Phase 2/3에서 사용 가능해요"** (한국어, SC-11 영향 없음)

**Phase 2 단독 완료 조건**:
- 로컬 영상 파일 → 추출 진행률 표시 → ready-to-separate 도달
- 로컬 오디오 파일 → 추출 단계 skip → 즉시 ready-to-separate
- corrupt video 시 즉시 실패 + 친절 메시지

**Phase 3 단독 완료 조건**:
- YouTube URL → 다운로드 진행률 → 영상이면 Phase 2 체이닝 → ready-to-separate
- ProcessPage 라우팅 (`navigateTo("process", { ids })`) — 단, ProcessPage 자체는 별도 피처이므로 Phase 3 시점에는 placeholder 알림으로 OK (ids 전달만 검증)
- 개별 cancel 동작 + 좀비 프로세스 0

---

## Version History

| Version | Date | Changes | Author |
|---|---|---|---|
| 0.1 | 2026-04-29 | Initial draft. Checkpoint 1+2 완료 (6 Key Decisions). Phase 1/2/3 분할. setup-page Foundation API 재사용 명시. SC 13건 정의. v1.2 app-lifecycle (uninstall cleanup) 이관 명시. | rhino-ty |
| 0.2 | 2026-04-29 | Iteration 1 cross-ref fix (13건). **충돌 4건**: ① HISTORY source_type 명명 통일 (`youtube\|file\|video`), ② MODEL_SELECTOR queue-page 영역 + 본구현 v1.1 분리 명시, ③ 앱 종료 안전 처리 책임 boundary (queue-page는 isProcessing source만, dialog는 별도 피처), ④ DownloadProgress/ExtractProgress ref COMMANDS.md 동기화 필요. **누락 6건**: ⑤ FR-21 단축키 (Delete/Ctrl+A/Ctrl+O/Escape), ⑥ FR-17 metadata 추출 (yt-dlp `--skip-download` + ffprobe), ⑦ FR-16 yt-dlp `--output` 강제, ⑧ FR-18 개별 cancel + QueueHandle 패턴, ⑨ SC-14/15 yt-dlp 실패 + corrupt video, ⑩ FR-19 model = "htdemucs_ft" 고정. **보강 3건**: ⑪ Phase 시간 추정 (~3/4/2h), ⑫ Phase 의존성 그래프 + 단독 완료 조건, ⑬ Convention 5종 추가 (Toast/UUID/DropZone visual/yt-dlp args/ffmpeg args). | rhino-ty |
| 0.3 | 2026-04-29 | Iteration 2 자체 review fix (13건). **A** Context Anchor RISK 5→12, **B** Executive Summary refresh (metadata/cancel/단축키 반영), **C** §6.1 Impact 5종 추가 (queue.ts / Toast.svelte / queue_tmp_dir / QueueHandle / queue-store.json), **D** §6.2 Future Consumer 5종 → 7종 (ModelSelector 인터페이스 + HistoryPage addToQueue + 앱 종료 다이얼로그), **E** NFR 9개 → 18개 (cancel response / metadata fetch / invalid URL / concurrency / memory / hard limits / DropZone visual / store size 등), **F** SC-1 명확화 (placeholder vs metadata 분리), **G** Phase 1 In Scope에 Toast 컴포넌트 추가, **H** Phase 2/3 In Scope에 metadata 추출 추가, **I** §6.3 Verification Phase별 분리 (Phase 1/2/3 각 4-5 항목), **J** §1.3 Related Documents에 SHORTCUTS / MODEL_SELECTOR / HISTORY 추가, **K** §7.4 schema migration 정책 추가, **M** 명시적 limit 정책 (큐 hard limit 없음, 다운로드 timeout 없음, store 크기 100KB 가정). | rhino-ty |
| 0.4 | 2026-04-29 | Iteration 3 self-review fix (12건). **중복 1건**: ① SC-1 ↔ SC-19 의미 중복 → SC-19 재정의 ("metadata fetch 5초"). **추정 정정 1건**: ② Phase 2 ~4h → ~3h, 합계 9h → 8h (setup-page와 동일). **모순 4건**: ③ Phase 의존성 그래프 "Phase 4" → "후속 피처 (ProcessPage)" 명칭 정정, ④ Phase 1 단독 완료 "alert + 비활성" → "disabled + tooltip" 정정, ⑤ "Cmd/Ctrl+A" → "Ctrl+A" (Windows 전용), ⑥ HISTORY 체크박스 vs queue-page — 체크박스 미사용 결정 + 사유 명시 (UI.md 일치). **미명시 5건**: ⑦ SC-18 Ctrl+O 전역 동작 명시 (auto navigateTo), ⑧ Phase 1 [▶ 분리 시작] disabled tooltip 한국어 명시, ⑨ Convention Toast §10.2 일관성 (자체 구현 결정), ⑩ translate_error 위치 결정 (common.rs §5 Error Translation), ⑪ FR-02 다중 파일 동시 드래그 명시, ⑫ EmptyState 형식 결정 (이모지 + 1줄 안내, SVG는 v1.1+). | rhino-ty |
| 0.5 | 2026-04-29 | Iteration 4 fresh-read fix (13건). ① Context Anchor SCOPE Phase 2 ~4h → ~3h sync (v0.4 정정 누락분), ② §1.2 Background table에 dir_size / available_space_mb / dev_log / translate_error 패턴 / dialog 4종 자원 추가, ③ ref COMMANDS.md sync 책임 명시 (Phase 3 종료 시 별도 docs commit), ④ Phase 2 ffmpeg timeout 30분 vs Phase 3 yt-dlp timeout 없음 분리 명시, ⑤ FR-04 cancel 메커니즘 FR-18과 공유 명시 (`removeFromQueue` 내부), ⑥ FR-08 URL은 exists() 체크 면제 명시 (file/video만 적용), ⑦ Phase 1 ModelSelector 슬롯 위치 Design 결정 명시, ⑧ §6.1 stores.ts navigateTo 시그니처 확장 (page, payload?) 추가, ⑨ FR-20 frontend store vs Rust is_processing() 책임 boundary 명시 (Rust는 v1.1), ⑩ EmptyState 문구 후보 2종 명시, ⑪ SC-18 "선택된 N개" 명시, ⑫ QueueItem.addedAt 정렬 기준 (top-down) 명시, ⑬ Convention dev_log 공유 정책 (setup.log + `queue:` prefix). | rhino-ty |
| **0.6** | 2026-04-29 | **Iteration 5 stability check fix (6건) — Plan 안정화 완료**. ① §6.3 Phase 1 verification "[▶ 분리 시작] alert" → "disabled + tooltip" 정정 (v0.4 fix와 모순 제거), ② §7.3 `QueueItem.label` 설명 → SC-19 "다운로드 준비 중..." → metadata 갱신 흐름과 일관, ③ SC-20 (FR-13 처리 중 시각화) + SC-21 (FR-19 model 고정 grep 검증) 신규 추가 → SC 19→21건, ④ §9.1 Design 체크리스트에 v0.4/v0.5 결정 8건 ✅ 표시 + Open 9건 분리, ⑤ §9.2 → §6.2 Future Consumers reference로 단순화 (중복 제거), ⑥ §7.2 K8 (subprocess Handle 일반화) Architectural Decision으로 추가. **다음 단계**: `/pdca design queue-page` 진입 가능. | rhino-ty |
