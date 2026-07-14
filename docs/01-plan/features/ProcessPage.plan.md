# ProcessPage Planning Document

> **Summary**: queue-page와 PlayerPage 사이의 처리 단계. queue-page에서 [▶ 분리 시작]한 wav 항목을 demucs로 분리 → 4 stems wav를 `{queue-tmp}/{id}/`에 보관 → 첫 완료 항목 자동 PlayerPage 진입. **순차 1개씩 처리** (GPU 메모리 안전). setup-page Foundation API 3차 재사용 검증.
>
> **Project**: MR Extractor
> **Version**: 0.1
> **Author**: rhino-ty
> **Date**: 2026-05-12
> **Status**: Draft v0.1 — Checkpoint 1+2 통과 후 작성, 사용자 검토 대기

---

## Executive Summary

| Perspective | Content |
|---|---|
| **Problem** | queue-page에서 [▶ 분리 시작] 클릭 후 실제 demucs 분리 처리가 없음. wav 파일은 `{queue-tmp}/{id}.wav`에 준비됐지만 4 stems 추출 단계 누락 → "URL → MR" 킬러 피처 미완성. 처리는 5분~30분 걸리는 작업이고, 사용자는 처리 중 다른 작업을 원함. |
| **Solution** | ProcessPage = 카드 리스트 (queue-page FileCard 패턴 답습) + separate.rs (demucs subprocess + tqdm 진행률 파싱 + 결과 glob 탐색) + **순차 1개씩 처리** + 첫 완료 항목 자동 PlayerPage 진입. queue-page의 QueueHandle + kill_process_tree + common::* Foundation API 전부 재사용. |
| **Function/UX Effect** | queue-page에서 N개 선택 → ProcessPage 진입 → 1번 항목 처리 → 첫 완료 즉시 PlayerPage 자동 진입 → 사용자가 믹싱하는 동안 나머지 N-1개 백그라운드 계속 처리. ProcessPage unmount해도 queueStore 통해 진행률 유지. 개별 cancel 가능. |
| **Core Value** | MR Extractor의 **킬러 피처 핵심 처리 단계**. setup-page Foundation (sidecar / queue_tmp / ErrorContext / kill_process_tree) + queue-page 패턴 (QueueHandle / 2-step 진행률 / 친절 에러)을 3차 재사용 — Foundation API가 1) setup install 2) queue download/extract 3) process separate까지 검증되면 후속 피처(PlayerPage export / v1.1 ModelSelector / v2 GPU 옵션)에 그대로 적용 가능한 안정 기반 확보. |

---

## Context Anchor

> Plan에서 자동 추출. Design/Do 핸드오프 시 전략적 맥락 유지.

| Key | Value |
|---|---|
| **WHY** | "URL → MR" 킬러 피처의 핵심 처리 단계 — queue-page에서 준비된 wav를 실제 demucs로 4 stems 분리해야 PlayerPage에서 믹싱/내보내기 가능. 처리는 오래 걸리지만 사용자는 다른 작업을 원함 → 백그라운드 + 자동 진입 패턴 필수. |
| **WHO** | queue-page 통과한 사용자 — URL/파일 입력 + 다중 선택 + [▶ 분리 시작] 클릭한 사용자. |
| **RISK** | ① demucs subprocess hang (큰 파일). ② tqdm 출력 포맷 변경 → 진행률 파싱. ③ GPU OOM (사용자 GPU 메모리 부족). ④ 모델 캐시 미스 (setup-page 보장이지만 사용자 캐시 삭제 시). ⑤ 결과 경로 glob 실패 (모델 디렉토리 못 찾음). ⑥ 순차 처리 중 1개 cancel → 다음 항목 자동 시작 안 됨 (for-await 흐름 깨짐). ⑦ 처리 완료 후 사용자 앱 종료 → queueStore 메모리 손실 (영속화는 pending만, done은 미영속). ⑧ Python ImportError (demucs는 OK인데 torchaudio가 망가짐). ⑨ TORCH_HOME 환경변수 전파 실패 → 모델 재다운로드 (느림). ⑩ ProcessPage unmount/remount 시 처리 중단. |
| **SUCCESS** | queue-page payload `{ ids, model }` → 순차 1개씩 처리. tqdm 진행률 < 2초 단위. 결과 4 stems wav 정확히 `{queue-tmp}/{id}/*/{vocals,drums,bass,other}.wav` glob 패턴 일치. 첫 완료 자동 PlayerPage 진입. 나머지 백그라운드 계속. 개별 cancel → 좀비 0. unmount 후에도 queueStore subscribe로 진행 유지. setup-page + queue-page Foundation 재사용 검증 완료. |
| **SCOPE** | Phase 1 (~3h): ProcessPage UI shell + 카드 리스트 + queueStore subscribe + cancel UI + 단축키. Phase 2 (~3h): separate.rs 본구현 + TORCH_HOME 전파 + tqdm 파싱 + glob 탐색 + QueueHandle 재사용. Phase 3 (~2h): 순차 정책 + 완료 자동 라우팅 + 친절 에러 매핑 + ProcessPage unmount 보존. **합계 ~8h** (setup/queue 동일 스케일). |

---

## 1. Overview

### 1.1 Purpose

queue-page와 PlayerPage 사이의 처리 단계. queue-page에서 `ready-to-separate` 상태인 큐 항목을 받아 demucs로 4 stems 분리 → 첫 완료 자동 PlayerPage 진입 → 나머지 백그라운드 처리. "URL → MR" 사용자 흐름의 마지막 비-사용자-주도 자동 단계.

### 1.2 Background

queue-page까지 wav 파일이 `{queue-tmp}/{id}.wav` 위치에 준비됨 (URL의 경우 yt-dlp 다운로드 + ffmpeg extract 후, 영상 파일의 경우 ffmpeg extract 후, 오디오 파일은 원본 path 또는 yt-dlp 다운로드 후). ProcessPage가 이 wav를 demucs로 분리하고 결과를 `{queue-tmp}/{id}/htdemucs_ft/{vocals,drums,bass,other}.wav`(또는 모델 와일드카드 패턴)에 보관. PlayerPage가 4 stems를 읽어 믹서 + 파형 + 키 조절 + 내보내기 단계 진행.

| 자원 | 출처 | ProcessPage에서 사용처 |
|---|---|---|
| `common::sidecar_dir` | setup-page Foundation | Embedded Python 경로 resolve (`python -m demucs`) |
| `common::queue_tmp_dir` | setup-page → queue-page에서 활용 | 입력 wav 위치 = 출력 디렉토리 부모 |
| `common::ErrorContext` enum | setup-page → queue-page에서 확장 | `Separation` variant 신규 추가 (Phase 2) |
| `common::translate_error` | setup-page → queue-page에서 활용 | demucs stderr 한국어 매핑 |
| `common::kill_process_tree` | setup-page → queue-page에서 활용 | 개별 cancel 시 subprocess tree kill |
| `common::dev_log` | setup-page Foundation | `process:` prefix 사용 (Convention §8) |
| `queue.rs::QueueHandle` | queue-page에서 도입 | PID 등록 → `cancel_queue_item` 재사용 가능 (큐 항목 ID 동일) |
| `queue.rs::cancel_queue_item` | queue-page에서 도입 | ProcessPage [✕] 클릭 시 호출 — 신규 cancel command 불필요 |
| `TORCH_HOME` 환경변수 | setup-page에서 설정 (`%APPDATA%/com.rhinoty.mr-extractor/torch-cache/`) | separate_audio subprocess에 동일 적용 — 모델 캐시 재사용 |
| `queueStore`, `QueueItem.status` | queue-page에서 도입 | 진행률 갱신 / status 전이 (`ready-to-separate` → `in-progress` → `done`/`error`) |
| `navigateTo({ kind, payload? })` | queue-page에서 확장 | 첫 완료 시 `navigateTo({ kind: "player", id })` |
| `pagePayload` store | queue-page에서 도입 | ProcessPage 진입 시 `{ ids, model }` 추출 |
| Tauri Channel API | queue-page에서 활용 | `SeparationProgress` 페이로드 스트리밍 |

### 1.3 Related Documents

- **선행 피처**:
  - [queue-page.report.md](../../archive/2026-05/queue-page/queue-page.report.md) — QueueHandle 패턴 + 2-step 진행률 + 친절 에러 매핑 검증
  - [setup-page.report.md](../../04-report/setup-page.report.md) — Foundation API 14종 + TORCH_HOME 설정
- **참조 스펙**:
  - [docs/references/COMMANDS.md](../../references/COMMANDS.md) — `separate.rs` 시그니처 + GPU 메모리 옵션 + 결과 경로 glob 규칙
  - [docs/references/MODEL_SELECTOR.md](../../references/MODEL_SELECTOR.md) — v1.1까지 `htdemucs_ft` 고정. ProcessPage는 model 파라미터로 받음 (queue-page → ProcessPage payload)
  - [docs/references/UX_BEHAVIORS.md](../../references/UX_BEHAVIORS.md) — 처리 중 종료 안전 처리 (v1.1 별도 피처)
  - [docs/references/UI.md](../../references/UI.md) — 진행률 표시 규칙 + 카드 디자인 패턴
- **로드맵**: [docs/ROADMAP.md](../../ROADMAP.md) v1 — `separate.rs` + ProcessPage

### 1.4 CLAUDE.md Do Not 준수

- ✅ demucs 결과 경로에 **모델명 하드코딩 금지** — `{queue-tmp}/{id}/*/{stem}.wav` 와일드카드 glob 사용 (SC-12)
- ✅ Tauri 플러그인 권한 누락 금지 — 신규 권한 추가 없음 (shell + fs + store 기존 권한만)
- ✅ `@tauri-apps/api/tauri` 사용 금지 → `@tauri-apps/api/core` v2 (Convention §)
- ✅ ffmpeg/ffprobe 진행률 ≠ 적용 (이건 queue-page에서 처리). demucs는 tqdm

---

## 2. Scope

### 2.1 In Scope

#### Phase 1 — UI Shell (Frontend only, ~3h)

- [ ] `src/pages/ProcessPage.svelte` 본구현 — placeholder 제거, 카드 리스트 + 단축키 listener
- [ ] queue-page의 `FileCard.svelte` 패턴 답습 (반복 vs 재사용 — Design §10.2에서 결정. 권장: 별도 `ProcessCard.svelte` 신규, 패턴만 답습)
- [ ] `src/components/process/ProcessCard.svelte` (신규) — 카드 1개 (라벨 + duration + 진행률 바 + step 텍스트 + cancel [✕] + 완료 시 [열기 →] 버튼)
- [ ] `src/components/process/EmptyState.svelte` (신규) — "처리할 항목이 없어요" + [큐로 돌아가기] (사용자가 실수로 ProcessPage 직접 진입 방어)
- [ ] `pagePayload` subscribe → `{ ids: string[], model: "htdemucs_ft" }` 추출
- [ ] `queueStore.filter(it => ids.includes(it.id))` → 표시 항목 결정 (queueStore는 SoT)
- [ ] 카드 클릭 비활성 (Phase 1에서). 완료 항목 [열기 →] 버튼만 navigateTo player
- [ ] 단축키 통합: `Escape` = goBack to queue, `Delete` = 선택 항목 cancel (선택 UI는 Phase 1에선 단일 클릭 단순화)
- [ ] Phase 2/3 도입 전엔 [열기 →] disabled + tooltip "분리 완료 후 사용 가능" — alert 사용 X (queue-page 패턴 일관)
- [ ] App.svelte의 페이지 라우팅은 이미 등록됨 (변경 없음)

#### Phase 2 — separate.rs 본구현 (Backend, ~3h)

- [ ] `src-tauri/src/commands/separate.rs` (신규) — 단일 파일, queue-page Option C 패턴 답습
  - § 1. SeparationProgress / SeparationResult 페이로드
  - § 2. `separate_audio(item_id, file_path, model, on_progress) -> Result<SeparationResult, String>`
  - § 3. demucs subprocess + TORCH_HOME 환경변수 전파
  - § 4. tqdm 진행률 파싱 (정규식 + indeterminate fallback)
  - § 5. 결과 경로 glob 탐색 — `{queue-tmp}/{id}/*/{vocals,drums,bass,other}.wav`
  - § 6. cancel hook (QueueHandle PID 등록 — queue.rs 재사용)
  - § 7. friendly error mapping (translate_error w/ ErrorContext::Separation)
- [ ] `src-tauri/src/commands/common.rs` §5 — `ErrorContext::Separation` variant 추가 + translate_error 분기
- [ ] `src-tauri/src/commands/mod.rs` — `pub mod separate;` 추가
- [ ] `src-tauri/src/lib.rs` — `separate::separate_audio` handler 등록 (`generate_handler![]`)
- [ ] `src/lib/types.ts` — `SeparationProgress` + `SeparationResult` + `QueueItem.outputs?: { vocals: string; drums: string; bass: string; other: string }` 신규 필드
- [ ] `src/lib/commands.ts` — `separateAudio(itemId, filePath, model, onProgress)` invoke wrapper (Channel)
- [ ] demucs 호출 인자: `python -m demucs -n {model} --out {queue-tmp}/{id}/ {file_path}` — `--out` 디렉토리는 별도 미생성, demucs가 자동 생성

#### Phase 3 — 순차 정책 + 완료 자동 라우팅 + 에러 (Frontend + Backend, ~2h)

- [ ] ProcessPage onMount → 받은 ids 순회 (sequential await): `for (const id of ids) { await separateAudio(id, ...) }`
- [ ] 각 항목 처리 시작 직전 `queueStore.updateItem(id, { status: "in-progress" })`
- [ ] 진행률 Channel emit → `queueStore.updateItem(id, { progress: { percent, step } })` — 카드 자동 갱신
- [ ] 항목 성공 시 `queueStore.updateItem(id, { status: "done", outputs: {...} })`
- [ ] **첫 완료 감지** → `navigateTo({ kind: "player", id })` 자동 호출. 단, ProcessPage 첫 진입 후 첫 완료에만 적용 (사용자가 player에서 back 후 다시 ProcessPage 진입 시 재트리거 안 함 — `hasRoutedRef` 가드)
- [ ] 항목 실패 시 `queueStore.updateItem(id, { status: "error", error: { code, message } })` → 다음 항목으로 계속 (cancel과 동일 흐름)
- [ ] 친절 에러 매핑 (translate_error ErrorContext::Separation):
  - GPU OOM (`CUDA out of memory`) → "그래픽 카드 메모리가 부족해요. 더 작은 파일로 시도해 주세요"
  - 모델 캐시 미스 (`No such file`) → "AI 모델을 찾을 수 없어요. 설정을 다시 확인해 주세요"
  - Python ImportError → "음원 분리 엔진에 문제가 생겼어요. 설정 화면으로 돌아가 주세요"
  - 결과 경로 glob 실패 → "분리 결과를 찾을 수 없어요. 다시 시도해 주세요"
- [ ] ProcessPage unmount/remount: queueStore SoT라 진행률 자동 유지. ids는 pagePayload에 저장돼 있어 재진입 시 같은 ids 사용
- [ ] `is_processing` derived (queue-page에서 도입)에 `in-progress` 이미 포함 — 별도 작업 X

### 2.2 Out of Scope

- **MR 합본 자동 생성** → **PlayerPage 책임** (사용자 결정 4번). ProcessPage는 4 stems wav만 출력
- **export.rs (믹스 내보내기 + 피치 시프트)** → **PlayerPage** (v1)
- **ModelSelector UI** → **v1.1**. ProcessPage는 model = `htdemucs_ft` 고정 (queue-page payload에서 받음, 변경 X)
- **VRAM 자동 감지 + GPU 옵션 UI** → **v2+** (ROADMAP). v1은 demucs 기본 옵션 (demucs가 자체적으로 GPU/CPU 결정)
- **노래방 모드 (`--two-stems=vocals`)** → **v2+**
- **동시 다중 처리 (2개+)** → **v1.1+** (사용자 결정 1번 — 순차 1개 고정)
- **다른 페이지에서 진행률 모니터링** → **v1.1+** (현재는 ProcessPage 활성 시에만 full UI. 다른 페이지에서는 `queueStore.isProcessing` derived만 활용 — 앱 종료 다이얼로그 등)
- **처리 중 종료 안전 다이얼로그** → **v1.1** (UX_BEHAVIORS.md `is_processing()` Rust + 다이얼로그 UI는 별도 피처)
- **앱 종료 시 outputs 영속화** → **v1.2 app-lifecycle**. v1은 메모리만 (PlayerPage가 sequential consumer라 큰 문제 없음. v1.1 HistoryPage 도입 시 영속화)
- **결과 wav 검증 (재생 가능 여부)** → **PlayerPage**의 wavesurfer/Web Audio 로드 시 검증
- **BPM / 키 감지** → **v1.1 / v2+**
- **별도 ProcessHandle 도입** → **불필요**. queue-page의 `QueueHandle` (PID 등록) + `cancel_queue_item` (Tauri command) 재사용 — 항목 ID 동일, cancel 인터페이스 일관

---

## 3. Requirements

### 3.1 Functional Requirements

| ID | Requirement | Priority | Status |
|---|---|---|---|
| FR-01 | ProcessPage 진입 시 `pagePayload`에서 `{ ids, model }` 추출 → `queueStore.filter(it => ids.includes(it.id))` 결과만 카드로 표시 | High | Pending |
| FR-02 | **순차 처리 1개씩** — `for (const id of ids) { await separateAudio(...) }`. 동시 demucs 인스턴스 ≤ 1 | High | Pending |
| FR-03 | demucs subprocess 호출: `python -m demucs -n {model} --out {queue-tmp}/{id}/ {wav_path}`. `--out` 디렉토리 명시, 작업 디렉토리에 결과 생성 금지 | High | Pending |
| FR-04 | TORCH_HOME 환경변수를 separate_audio subprocess에 전파 — setup-page와 동일 (`%APPDATA%/com.rhinoty.mr-extractor/torch-cache/`). 모델 재다운로드 방지 | High | Pending |
| FR-05 | tqdm stdout/stderr 진행률 파싱 → Channel<SeparationProgress> emit (2초 단위). step 텍스트는 한국어 ("모델 로드 중...", "음원 분리 중...", "스템 추출 중...") | High | Pending |
| FR-06 | 결과 경로 glob 탐색 — `{queue-tmp}/{id}/*/{vocals,drums,bass,other}.wav` 패턴. **모델명 하드코딩 금지** (CLAUDE.md Do Not). 4개 모두 발견되지 않으면 Err | High | Pending |
| FR-07 | 첫 완료 감지 시 `navigateTo({ kind: "player", id })` 자동 호출. ProcessPage 첫 진입 후 첫 완료에만 적용 (`hasRoutedRef` 가드, 사용자가 player에서 back 후 재진입 시 재트리거 X) | High | Pending |
| FR-08 | 처리 중 항목 [✕] cancel → `cancel_queue_item(id)` invoke (queue-page 재사용) → subprocess tree kill + 임시 디렉토리(`{queue-tmp}/{id}/`) 삭제 → status="error", 다음 항목 계속 처리 | Medium | Pending |
| FR-09 | 친절 에러 매핑 (ErrorContext::Separation): GPU OOM / 모델 캐시 미스 / Python ImportError / glob 실패 / 일반 Python 에러. 한국어 + [상세] 토글 (raw stderr) | Medium | Pending |
| FR-10 | ProcessPage unmount 시에도 처리 계속 (`queueStore` SoT). 다른 페이지 이동 후 ProcessPage 재진입 시 진행률 그대로 표시 (subscribe pattern) | Medium | Pending |
| FR-11 | 카드 UI (`ProcessCard.svelte`): 라벨 + duration + 진행률 바 (%) + step 텍스트 + cancel [✕] (처리 중) + [열기 →] (완료) | Medium | Pending |
| FR-12 | 빈 상태 — pagePayload.ids 비어 있거나 일치 항목 없을 때 `EmptyState` ("처리할 항목이 없어요" + [큐로 돌아가기]) — 실수 진입 방어 | Low | Pending |
| FR-13 | 단축키 통합: `Escape` = `navigateTo({ kind: "queue" })` (goBack 패턴), `Delete` = 선택 항목 cancel (단일 클릭 선택 UI — Phase 1 단순화) | Medium | Pending |
| FR-14 | `QueueItem` 데이터 모델 확장: `outputs?: { vocals: string; drums: string; bass: string; other: string }` 신규 필드 — PlayerPage가 이를 읽어 4 stems wav 로드 | High | Pending |
| FR-15 | demucs subprocess timeout = **없음** (큰 파일 허용, queue-page yt-dlp 다운로드와 동일 정책). 대신 cancel UI [✕]로 제어 (FR-08) | Medium | Pending |
| FR-16 | 처리 시작 직전 queueStore item status 전이: `ready-to-separate` → `in-progress`. 진행률 갱신은 `it.progress = { percent, step }` 신규 필드 (`QueueItem.progress?` optional) | Medium | Pending |

### 3.2 Non-Functional Requirements

| Category | Criteria | Measurement |
|---|---|---|
| Performance | 진행률 Channel emit < 2초 단위 | dev_log 출력 빈도 |
| Performance | 첫 완료 감지 → PlayerPage 자동 진입 < 500ms | useEffect/store subscribe 즉시성 |
| Performance | ProcessPage 진입 → 첫 카드 표시 < 100ms | console.time, queueStore filter cost |
| Performance | 5분 영상 (44.1kHz stereo, ~50MB wav) 처리 시간 (참고치): GPU 7GB+ ~1분, CPU ~3분. 사용자 환경 의존 | 벤치마크 |
| Reliability | 동시 demucs 실행 ≤ 1 | invariant test (Phase 3 verification) |
| Reliability | demucs subprocess timeout = 없음 (큰 파일 허용) | NFR Limits, queue-page와 동일 정책 |
| Reliability | ProcessPage unmount/remount 시 처리 중단 X | queueStore SoT |
| Reliability | 단일 항목 실패 → 다음 항목 계속 처리 | for-await loop 흐름 |
| Concurrency | ProcessPage는 ids만 받아 순차 처리. ProcessPage 재진입 (중복 클릭 등) 시 `is_processing` 가드 | onMount 검사 |
| Memory | 4 stems 결과 wav 디스크 점유 ~2배 원본 (44.1kHz 16-bit stereo, 4 trillion samples) | 디스크 측정. queue-tmp 누수 v1.2 cleanup 의존 |
| Storage | 출력 위치 = `{queue-tmp}/{id}/*/` — 디스크 청결 유지 (CLAUDE.md Output Rules ~/Desktop/MR Extractor/은 PlayerPage export 시 사용) | 디스크 검사 |
| UX | 다크 테마 전용 (`--color-*` CSS 변수 준수) | 코드 리뷰 |
| UX | 진행률 step 텍스트 한국어 ("모델 로드 중..." / "음원 분리 중..." / "스템 추출 중...") | UI 검수 |
| Tech Jargon | UI 본문 grep — "demucs" / "torch" / "tqdm" / "python" / "pip" 0건 | SC-11 grep |
| Error Clarity | 다운로드 실패 한국어 + [상세] 토글 | translate_error 패턴 (queue-page 일관) |
| Limits | 동시 처리 = 1 (순차). v1.1+ 동시 다중 도입 시 GPU 메모리 가드 필수 | 운영 가이드 |
| Compatibility | TORCH_HOME ENV 일치 (setup-page와) | dev_log subprocess env 출력 검증 |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] **SC-1**: queue-page에서 [▶ 분리 시작 (N)] → ProcessPage 진입 → `pagePayload.ids`의 N개 항목만 카드 표시. queueStore의 다른 항목은 미표시 (FR-01)
- [ ] **SC-2**: 순차 처리 1개씩 — 동시에 demucs subprocess 인스턴스 ≤ 1. 작업 관리자/`tasklist` 확인 (FR-02)
- [ ] **SC-3**: tqdm 진행률 < 2초 단위 갱신 (Channel emit 빈도 측정) (FR-05)
- [ ] **SC-4**: 결과 4 stems wav가 `{queue-tmp}/{id}/*/vocals.wav`, `drums.wav`, `bass.wav`, `other.wav` glob 패턴 일치 (FR-06)
- [ ] **SC-5**: 첫 완료 감지 → 자동 PlayerPage 진입 (`navigateTo({ kind: "player", id })`). 사용자 클릭 0회 (FR-07)
- [ ] **SC-6**: 나머지 N-1개 항목은 백그라운드 계속 처리 — PlayerPage 활성 상태에서도 queueStore의 `status` 자동 갱신 (FR-10)
- [ ] **SC-7**: 처리 중 [✕] cancel → 5초 내 demucs subprocess 종료 + 좀비 0 + `{queue-tmp}/{id}/` 디렉토리 삭제 (FR-08)
- [ ] **SC-8**: Cancel 후 다음 항목 자동 처리 시작 (for-await loop continue) (FR-02 + FR-08)
- [ ] **SC-9**: 친절 에러 매핑 검증 — GPU OOM 시뮬레이션 (큰 파일 + CPU 강제), 모델 캐시 삭제 후 진입, Python venv 망가뜨리기 (FR-09)
- [ ] **SC-10**: ProcessPage unmount (다른 페이지 이동) → queueStore.subscribe 통해 진행률 계속 갱신. 재진입 시 진행 그대로 표시 (FR-10)
- [ ] **SC-11**: UI 본문 grep — "demucs" / "torch" / "tqdm" / "python" / "pip" / "ffmpeg" 0건 (Tech Jargon)
- [ ] **SC-12**: 모델명 하드코딩 grep — separate.rs 코드/glob 패턴 위치에 `htdemucs_ft` 리터럴 없음 (`--out` 디렉토리 자식 wildcard 사용) (CLAUDE.md Do Not 준수)
- [ ] **SC-13**: TORCH_HOME 환경변수가 separate_audio subprocess에 전파됨 — dev_log 출력 검증 또는 모델 재다운로드 발생 X 확인 (FR-04)
- [ ] **SC-14**: `pnpm tauri build` 성공, `cargo check` 0 warnings, `pnpm check` 0 errors
- [ ] **SC-15**: setup-page Foundation API 3차 재사용 검증 — `common::sidecar_dir` (python 경로), `queue_tmp_dir` (입력 위치), `ErrorContext::Separation` (신규 variant), `kill_process_tree` (cancel) 모두 활용 + `QueueHandle` (queue-page) + `cancel_queue_item` (queue-page) 재사용
- [ ] **SC-16**: ProcessPage 직접 진입 (queue-page payload 없이 — pagePayload.ids 비어있음) → `EmptyState` 표시 + [큐로 돌아가기] 버튼 동작 (FR-12)
- [ ] **SC-17**: PlayerPage에서 [뒤로] 후 ProcessPage 재진입 시 자동 player 라우팅 재트리거 X (`hasRoutedRef` 가드, FR-07)
- [ ] **SC-18**: 단축키 검증 — `Escape` = queue-page 복귀, `Delete` = 선택 항목 cancel (FR-13)
- [ ] **SC-19**: capabilities scope 추가 0건 — 기존 shell/fs/store/sidecar 권한만으로 동작
- [ ] **SC-20**: `QueueItem.outputs` 필드가 4개 stem 절대경로 모두 채워짐 (성공 시) — PlayerPage가 이를 읽어 wavesurfer 로드 가능 (FR-14)

### 4.2 Quality Criteria

- [ ] Gap Analysis Match Rate ≥ 90%
- [ ] 한국어 라벨 전수 검수 (UI.md UX 규칙 준수)
- [ ] setup-page + queue-page Foundation API 재사용 검증 — Foundation 패턴이 3차 사용까지 일관됨 (sidecar / queue_tmp / ErrorContext / kill_process_tree / QueueHandle)
- [ ] capabilities scope 추가 없이 기존 권한만으로 동작
- [ ] CLAUDE.md Design & Implementation Checklist 전수 충족

---

## 5. Risks and Mitigation

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| demucs subprocess hang (큰 파일/네트워크 문제 없는데 무한 대기) | High | Low | cancel UI [✕] + kill_process_tree. subprocess timeout = 없음 (큰 파일 의도적 허용). 사용자가 5분 이상 진행률 갱신 없으면 dev_log 경고 → 수동 cancel |
| tqdm 출력 포맷 변경 (demucs 버전 업) | Medium | Medium | 정규식 + indeterminate fallback (queue-page yt-dlp 패턴 답습). demucs 버전 고정 (setup-page 의존, ≥4,<5) |
| GPU OOM (`CUDA out of memory`) | Medium | Medium | 친절 에러 매핑 ("그래픽 카드 메모리 부족"). v1.1 ModelSelector 도입 시 더 작은 모델 선택지. v2+ GPU 옵션 UI에서 `--device cpu` 강제 옵션 |
| 모델 캐시 미스 (TORCH_HOME 정상이지만 사용자가 캐시 삭제) | Medium | Low | 친절 에러 ("AI 모델을 찾을 수 없어요. 설정을 다시 확인해 주세요") + 설정 화면으로 안내. setup-page 재실행 유도 |
| 결과 경로 glob 실패 (demucs는 성공인데 우리 패턴 매칭 실패) | Medium | Low | 명확한 에러 메시지 + dev_log + glob 패턴 명시. demucs 출력 디렉토리 트리를 dev_log에 dump |
| 순차 처리 중 1개 cancel → 다음 항목 자동 시작 안 됨 (for-await throw 미흡) | Medium | Medium | cancel은 throw 아닌 status="error"로 처리 → loop continue. test: 3개 항목 중 가운데 cancel → 첫/세 번째 정상 처리 (SC-8) |
| 처리 완료 후 사용자 강제 종료 (앱 닫기) | Low | Low | outputs는 queueStore 메모리만, 디스크 wav는 남음. v1.1 HistoryPage 도입 시 영속화 검토 (Plan §2.2 Out of Scope) |
| Python ImportError (demucs는 OK인데 torchaudio 망가짐) | Medium | Low | 친절 에러 + setup-page 재실행 유도. setup-page에서 health check 강화 가능 (Plan §1.2 자원 표 참조) |
| TORCH_HOME 환경변수 전파 실패 (subprocess command builder 누락) | Medium | Low | SC-13 검증. dev_log subprocess env 출력. setup-page에서 동일 패턴 검증됨 |
| ProcessPage unmount/remount 시 처리 중단 (잘못된 onDestroy 핸들러) | High | Low | queueStore SoT, separate_audio invoke는 page 라이프사이클과 독립. onMount에서 시작한 for-await loop은 page와 무관. 단, loop 자체는 ProcessPage 메모리에 있으므로 unmount 시 loop 변수도 해제 → **유의점**: loop를 store layer (queue.ts)로 이전하거나, ProcessPage가 unmount될 때도 loop는 별도 Promise로 유지 |
| ProcessPage 재진입 시 자동 player 라우팅 재트리거 | Medium | Medium | `hasRoutedRef` 가드 — sessionStorage 또는 store 레벨 boolean (SC-17) |
| 동시 ProcessPage 진입 (사용자 빠른 더블 클릭) | Low | Low | `is_processing` derived 검사 + onMount 가드 |

---

## 6. Impact Analysis

### 6.1 Changed Resources

| Resource | Type | Change |
|---|---|---|
| `src/pages/ProcessPage.svelte` | Svelte Page | placeholder → 본구현 (카드 리스트 + queueStore subscribe + 순차 처리 + 단축키) |
| `src/components/process/` (신규 디렉토리) | Components | ProcessCard.svelte + EmptyState.svelte 2종 |
| `src/lib/types.ts` | TS (확장) | `SeparationProgress`, `SeparationResult`, `QueueItem.outputs?` (4 stems), `QueueItem.progress?` ({ percent, step }), `NavigatePayload`에 `{ kind: "player", id: string }` variant 확인 (없으면 추가) |
| `src/lib/commands.ts` | TS (확장) | `separateAudio(itemId, filePath, model, onProgress)` invoke wrapper (Channel) |
| `src/lib/queue.ts` | TS (확장, 선택) | `updateItemProgress(id, percent, step)` / `addOutputs(id, outputs)` 헬퍼 (또는 ProcessPage 인라인) |
| `src/lib/stores.ts` | TS (확장, 선택) | `hasRoutedRef` boolean store (또는 ProcessPage 컴포넌트 state) |
| `src-tauri/src/commands/separate.rs` | Rust (신규) | demucs subprocess + Channel + glob 탐색 + cancel hook + friendly error |
| `src-tauri/src/commands/common.rs` | Rust (확장) | §5 `ErrorContext::Separation` variant 추가 + translate_error 분기 (CUDA OOM / 모델 캐시 / Python Import) |
| `src-tauri/src/commands/mod.rs` | Rust (확장) | `pub mod separate;` |
| `src-tauri/src/lib.rs` | Rust (확장) | `separate::separate_audio` handler 등록 (`generate_handler![]`). QueueHandle 재사용 — 신규 State 등록 X |
| `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/{id}/` | Runtime Asset | 디렉토리 자식으로 모델 디렉토리 (예: `htdemucs_ft/`) 생성. uninstall v1.2 처리 |
| `tauri-plugin-shell` capabilities | Library | python 명령 이미 허용됨 (setup-page 검증). 추가 작업 X |

### 6.2 Future Consumers

| Future Feature | Operation | Reuses (Plan §6.1 export) |
|---|---|---|
| **PlayerPage** (다음 피처) | 4 stems wav 읽기 + Web Audio 믹서 + 파형 + 키 조절 | `QueueItem.outputs` (4 stems 경로) + `queueStore` subscribe |
| **export.rs / PlayerPage 내보내기** | MR 합본 (vocals 제외) + 피치 시프트 + ~/Desktop/MR Extractor/ 출력 | `QueueItem.outputs` + 사용자 설정 (피치/포맷) |
| **v1.1 ModelSelector** | model 인자 분기 (`htdemucs` / `htdemucs_ft` / `htdemucs_6s`) | `separate_audio` model 파라미터 + UI 슬롯 (queue-page에서 reserve) |
| **v1.1 HistoryPage** | 완료된 처리 결과 보관 + 재처리 진입 | `QueueItem.status='done' + outputs` + `addToQueue` (queue.ts) |
| **v1.2 SettingsPage** | "임시 파일 정리" + 저장 공간 표시 | `common::queue_tmp_dir` + `dir_size` (Foundation) — Process 결과도 정리 대상 |
| **v2+ GPU 옵션** | `--segment` / `--device` UI | `separate_audio` 추가 인자 (확장) |
| **v2+ 노래방 모드** | `--two-stems=vocals` | `separate_audio` 옵션 추가 |

### 6.3 Verification (Phase별)

**Phase 1**:
- [ ] `pnpm tauri dev` → queue-page에서 [▶ 분리 시작] → ProcessPage 진입 → 받은 ids만 카드 표시
- [ ] queueStore 직접 mock (모의 status 갱신) → 카드 UI 자동 갱신 확인
- [ ] [열기 →] disabled + tooltip "분리 완료 후 사용 가능" (Phase 2/3 도입 전)
- [ ] EmptyState (ProcessPage 직접 진입 시) + [큐로 돌아가기]
- [ ] 단축키: Escape → queue-page 복귀

**Phase 2**:
- [ ] 단일 wav 항목 separate_audio invoke → tqdm 진행률 → 4 stems 결과 glob 매칭
- [ ] TORCH_HOME 전파 확인 (dev_log subprocess env 출력)
- [ ] 결과 wav 파일 4개 존재 + 재생 가능 (외부 player로 OK 확인)
- [ ] friendly error 매핑 검증 — 모델 캐시 삭제 후 호출 시 한국어 에러
- [ ] cancel 검증 — subprocess 종료 + 디렉토리 삭제 (queue-page cancel과 일관)

**Phase 3**:
- [ ] N=3 ids로 ProcessPage 진입 → 순차 1개씩 처리 → 첫 완료 자동 PlayerPage 진입 → 나머지 2개 백그라운드 계속
- [ ] 가운데 항목 cancel → 다음 항목 시작 (loop continue)
- [ ] PlayerPage에서 뒤로 → ProcessPage 재진입 → 자동 라우팅 재트리거 X (`hasRoutedRef`)
- [ ] ProcessPage unmount (queue 페이지로 이동) → queueStore subscribe로 진행률 갱신 계속

---

## 7. Architecture Considerations

### 7.1 Project Level

| Level | Selected |
|---|:--:|
| **Starter** | ☐ |
| **Dynamic** | ☑ |
| **Enterprise** | ☐ |

이미 setup/queue-page와 동일한 Dynamic level. Tauri v2 + Svelte 5 + pnpm 스택 유지.

### 7.2 Key Architectural Decisions

| Decision | Options | Plan 권장 (Design에서 최종) | Rationale |
|---|---|---|---|
| **동시 실행 정책** | 순차 1개 / VRAM 자동 분기 / CPU 고정 순차 | **순차 1개씩** | 사용자 결정 (Checkpoint 2). GPU 메모리 안전, 구현 단순, 사용자 예측 가능 |
| **출력 위치** | queue-tmp/{id}/ / ~/Desktop/ / hybrid | **`{queue-tmp}/{id}/`** | 사용자 결정. PlayerPage가 export 시 ~/Desktop/MR Extractor/로 이동 (책임 명확) |
| **완료 라우팅** | 자동 첫 완료 / 사용자 클릭 / 모두 완료 후 | **자동 첫 완료** | 사용자 결정. 사용자가 다음 작업으로 빠르게 진입, 나머지는 백그라운드 |
| **MR 합본 책임** | ProcessPage / PlayerPage / 둘 다 | **PlayerPage 책임 (내보내기 시)** | 사용자 결정. ProcessPage = 4 stems만. PlayerPage가 믹스 조절 후 ffmpeg 합본 |
| **separate.rs 파일 구조** | 단일 파일 + 섹션 주석 / 모듈화 | **단일 파일** (Option C 답습) | setup/queue 일관, 학습 비용 0 |
| **Handle 재사용** | QueueHandle 재사용 / ProcessHandle 별도 | **QueueHandle 재사용** | 항목 ID 동일, cancel 인터페이스 일관, queue-page `cancel_queue_item` 그대로 활용. 신규 State 등록 X |
| **진행률 페이로드** | { step, percent } / { percent } 단일 / Channel<u32> | **{ item_id, step, percent }** | DownloadProgress / ExtractProgress 패턴 일관. step 텍스트로 사용자에게 단계 인식 |
| **Loop 위치** | ProcessPage onMount / store layer (queue.ts) / 별도 service | **store layer 권장** (Design에서 최종) | ProcessPage unmount 시 처리 중단 방지. queueStore 또는 별도 module-level Promise |
| **카드 컴포넌트** | queue-page FileCard 재사용 / 신규 ProcessCard | **신규 ProcessCard 권장** | 책임 분리 (FileCard는 queue 항목, ProcessCard는 처리 항목). 패턴만 답습, 코드 복제로 결합도 낮춤 |

### 7.3 Clean Architecture

```
Selected Level: Dynamic

src/
├── pages/
│   └── ProcessPage.svelte           # 페이지 (라우팅 + 단축키 + 순차 loop 트리거)
├── components/
│   └── process/                     # ProcessPage 전용 컴포넌트
│       ├── ProcessCard.svelte       # 카드 1개 (진행률 + cancel + 열기)
│       └── EmptyState.svelte        # 빈 상태
└── lib/
    ├── commands.ts                  # separateAudio invoke wrapper
    ├── stores.ts                    # queueStore (queue-page에서 도입) + hasRoutedRef (신규, 선택)
    ├── queue.ts                     # updateItemProgress / addOutputs 헬퍼 (선택)
    └── types.ts                     # SeparationProgress / SeparationResult / QueueItem.outputs / .progress

src-tauri/src/
├── commands/
│   ├── separate.rs (신규)           # demucs subprocess + Channel + glob + cancel
│   ├── common.rs (확장)             # §5 ErrorContext::Separation 추가
│   ├── queue.rs (재사용)            # QueueHandle + cancel_queue_item (변경 없음)
│   └── mod.rs                       # pub mod separate;
└── lib.rs                           # separate::separate_audio handler 등록
```

---

## 8. Convention Prerequisites

### 8.1 Existing Project Conventions

- [x] `CLAUDE.md` 코딩 규칙 — Svelte 5 runes, Tailwind `--color-*`, Tauri v2 `@tauri-apps/api/core`, types.ts 중앙 정의
- [x] `docs/references/COMMANDS.md` — separate.rs 시그니처 + GPU 메모리 옵션
- [x] CLAUDE.md "Do Not" — demucs 결과 경로 모델명 하드코딩 금지
- [x] CLAUDE.md "Design & Implementation Checklist" — Rust 커맨드 / Svelte 프론트엔드 / Tauri 설정 모두
- [x] queue-page `docs/references/COMMIT.md` 없음 → principled-git-commit skill 사용 (글로벌)

### 8.2 Conventions to Define/Verify

| Category | Current | To Define | Priority |
|---|---|---|---|
| **Naming** | Convention 명시 (PascalCase Svelte, camelCase TS, snake_case Rust) | 변경 없음 | - |
| **separate.rs 섹션 주석** | queue-page Option C 패턴 (§1~§7) | 답습 | High |
| **ErrorContext 확장** | setup/queue가 4 variants | `Separation` 5번째 variant 추가 | High |
| **dev_log prefix** | `setup:` / `queue:` 패턴 | `process:` prefix 추가 | Medium |

### 8.3 Environment Variables Needed

| Variable | Purpose | Scope | To Be Created |
|---|---|---|---|
| `TORCH_HOME` | demucs 모델 캐시 경로 | Rust subprocess env | ☐ (setup-page에서 이미 설정 — 전파만 필요) |

### 8.4 Pipeline Integration

| Phase | Status | Document Location |
|---|:--:|---|
| Phase 1 (Schema) | ☑ | `docs/references/` |
| Phase 2 (Convention) | ☑ | `CLAUDE.md` + `docs/references/` |
| Phase 3 (Mockup) | N/A | UI 본 구현으로 직행 (queue-page 패턴 답습) |
| Phase 4 (API) | 진행 중 | separate.rs Phase 2에서 본구현 |
| Phase 5 (Design System) | ✅ | 다크 토큰 (`--color-*`) 기존 정의 |
| Phase 6 (UI Integration) | 진행 중 | ProcessPage Phase 1 |
| Phase 7 (SEO/Security) | N/A | 데스크탑 앱 |
| Phase 8 (Review) | 진행 중 | gap-detector Check phase |
| Phase 9 (Deployment) | N/A | Tauri 빌드 |

---

## 9. Next Steps

1. [ ] Plan v0.1 사용자 검토 → iterate stabilize (queue-page는 v0.1 → v0.6 stabilized)
2. [ ] `/pdca design ProcessPage` — Option C — Pragmatic Balance 답습 권장
3. [ ] Implementation Phase 1 → Phase 2 → Phase 3
4. [ ] Check phase (gap-detector) — Match Rate ≥ 90% 목표
5. [ ] Report + Archive

---

## Version History

| Version | Date | Changes | Author |
|---|---|---|---|
| 0.1 | 2026-05-12 | Initial draft. Checkpoint 1 (요구사항 이해) + Checkpoint 2 (4 decisions: 순차 1개씩 / queue-tmp 출력 / 자동 첫 완료 / MR PlayerPage 책임) 통과 후 작성. setup-page + queue-page Foundation 재사용 패턴 답습. | rhino-ty |
