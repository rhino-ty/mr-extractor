# ProcessPage Design Document

> **Summary**: ProcessPage.plan.md v0.1의 구현 설계. queue-page Option C — Pragmatic Balance 답습.
> Plan §7.2의 "Design에서 최종" 결정 사항을 확정하고, 실제 코드베이스 대조에서 발견된 편차를 명시.
>
> **Project**: MR Extractor
> **Version**: 1.0 (as-built)
> **Author**: rhino-ty
> **Date**: 2026-07-14
> **Status**: Implemented (Phase 1+2+3)
> **Plan Ref**: [ProcessPage.plan.md](../../01-plan/features/ProcessPage.plan.md)

---

## 1. 확정된 아키텍처 결정 (Plan §7.2 → 최종)

| Decision | 확정 | 근거 |
|---|---|---|
| Loop 위치 | **`src/lib/process.ts` 신규 모듈 (store layer)** | ProcessPage unmount와 독립적인 module-level Promise. 페이지는 트리거만 (FR-10 / SC-10) |
| 카드 컴포넌트 | **신규 `ProcessCard.svelte`** | FileCard 패턴 답습 + 코드 복제로 결합도 낮춤 |
| hasRoutedRef 위치 | **process.ts module state** | 배치 시작 시 reset. 페이지 재진입과 무관하게 배치당 1회 보장 (SC-17) |
| 진행률 페이로드 | **`{ item_id, step, percent }`** | DownloadProgress 패턴 일관 |
| Handle | **QueueHandle 재사용** | 신규 State 등록 없음. cancel_queue_item 그대로 활용 |

## 2. 코드베이스 대조에서 확정된 편차 (Plan과 다른 지점)

| # | Plan 표기 | 실제 구현 | 이유 |
|---|---|---|---|
| 1 | `common::sidecar_dir`로 Python resolve (Plan §1.2) | **`common::venv_python_path`** + `common::python_env_vars` | demucs는 setup-page가 venv에 설치. sidecar_dir은 ffmpeg/yt-dlp용. TORCH_HOME은 python_env_vars가 일괄 주입 (FR-04 충족) |
| 2 | `QueueItem.progress?: { percent, step }` 신규 필드 (FR-16) | **기존 `progress: number` + `step?: string` 재사용** | queue-page가 이미 도입한 필드. FileCard/ProcessCard 모두 동일 소비 |
| 3 | cancel 시 `{queue-tmp}/{id}/` 삭제를 cancel_queue_item이 담당 (FR-08) | **separate.rs 실패 경로에서 out dir 정리** | Plan §7.3 "queue.rs 변경 없음" 유지. kill → wait 반환 → 실패 경로에서 remove_dir_all |
| 4 | 결과 glob `{queue-tmp}/{id}/*/{stem}.wav` 1-depth (FR-06) | **최대 3-depth 재귀 탐색** | demucs 실제 출력은 `{out}/{model}/{track}/{stem}.wav` 2-depth. 이름 기반 재귀 매칭으로 구조 독립 (모델명 하드코딩 없음, SC-12) |
| 5 | goBack 재진입 시 pagePayload로 ids 복원 (FR-10) | **`processBatch` store fallback** | `goBack()`은 pagePayload를 null로 비움 → 마지막 배치를 process.ts store에 보존 |

## 3. 데이터 흐름

```
QueuePage [▶ 분리 시작] → navigateTo("process", { ids, model })
  → ProcessPage onMount → startSeparationBatch(ids, model)   # process.ts, 재진입 가드
    → for (id of ids):                                        # 순차 1개 (FR-02)
        queueStore: ready-to-separate → in-progress
        separateAudio(id, tmpPath, model, onProgress)         # commands.ts Channel
          → Rust separate_audio:
              venv python -m demucs -n {model} --out {queue-tmp}/{id}/ {wav}
              env: TORCH_HOME / PIP_CACHE_DIR / PYTHONUNBUFFERED / PATH+sidecar
              stdout: "bag of N" → 진행률 분모  /  stderr: tqdm %| → Channel emit
              성공 → 재귀 탐색 4 stems → SeparationResult
              실패/취소 → out dir 정리 → 친절 에러 (ErrorContext::Separation)
        성공: queueStore done + outputs { vocals, drums, bass, other }
              첫 완료 → navigateTo("player", { itemId })      # hasRouted 가드 (FR-07)
        실패: queueStore error + errorDetail → 다음 항목 계속  # SC-8
```

## 4. tqdm 진행률 파싱 (§4, separate.rs)

- tqdm은 `\r` in-place 갱신 → line reader 대신 **바이트 chunk 읽기 + `\r`/`\n` 분리** (SC-3: < 2초 갱신)
- `%|` 마커를 신뢰 신호로 사용 (일반 에러 텍스트 `%` 오인 방지). 파싱 실패 시 indeterminate fallback
- Bag-of-N 합산: stdout "Selected model is a bag of N models" 파싱 → `overall = (bars_done*100 + pct) / N`
  - percent 급락(−5 초과)으로 새 바 시작 감지, max 클램프로 역행 방지, 완료 전 상한 99%
- step 텍스트 (FR-05): "모델 로드 중..." (spawn 직후) → "음원 분리 중..." (tqdm) → "스템 추출 중..." (탐색)

## 5. 에러 매핑 (ErrorContext::Separation, common.rs §5)

| 패턴 (평가 순서) | 메시지 |
|---|---|
| `out of memory` | 그래픽 카드 메모리가 부족해요. 더 작은 파일로 시도해 주세요. |
| `importerror` / `modulenotfounderror` / `no module named` | 음원 분리 엔진에 문제가 생겼어요. 설정 화면으로 돌아가 주세요. |
| `no such file` / `filenotfounderror` | AI 모델을 찾을 수 없어요. 설정을 다시 확인해 주세요. |
| `traceback` (일반 Python 에러) | 음원 분리 중 문제가 발생했어요. 다시 시도해 주세요. |
| 결과 탐색 실패 (separate.rs 직접) | 분리 결과를 찾을 수 없어요. 다시 시도해 주세요. |
| 취소 (was_cancelled 판정) | 처리가 취소되었어요. |

취소 판정: `handle.take(&item_id).is_none()` (cancel_queue_item이 먼저 take) — 단, **성공 판정이 우선**
(완료 직후 cancel 경합 시 정상 결과 보존).

## 6. 파일 매니페스트

| 파일 | 변경 |
|---|---|
| `src-tauri/src/commands/separate.rs` | 본구현 (§1~§7 섹션, Option C) |
| `src-tauri/src/commands/common.rs` | `ErrorContext::Separation` + translate_error 분기 |
| `src-tauri/src/commands/mod.rs`, `lib.rs` | 변경 없음 (기존 등록 유지) |
| `src/lib/process.ts` | 신규 — startSeparationBatch / processBatch / isSeparationRunning |
| `src/lib/types.ts` | StemOutputs / SeparationProgress / SeparationResult / QueueItem.outputs |
| `src/lib/commands.ts` | separateAudio → Channel 래퍼 교체 |
| `src/pages/ProcessPage.svelte` | placeholder → 본구현 |
| `src/components/process/ProcessCard.svelte` | 신규 |
| `src/components/process/EmptyState.svelte` | 신규 |

## 7. 검증 결과 (Check phase)

- `cargo check` 0 warnings / `pnpm check` 0 errors 0 warnings / `pnpm build` 성공
- SC-11: UI 문자열 기술 용어 grep 0건 (주석 제외)
- SC-12: separate.rs 코드에 모델명 리터럴 0건
- SC-19: capabilities 변경 0건
- **gap-detector Match Rate 96.1%** (목표 90% 초과) — FR 16/16 충족, 미충족 갭 0건
- **code-analyzer**: Critical 0 / Important 3 → 2건 수정 반영:
  - 재진입 시 새 배치 무음 드랍 → 가드 토스트 + ProcessPage 실행 중 분기 추가
  - cancel의 입력 wav 삭제 동작과 주석 불일치 → 주석 정정 (queue-page cancel 의미론)
  - `text-white`(EmptyState)는 QueuePage accent 버튼과 동일 패턴이라 유지
- 런타임 SC 5건 (SC-3/4/7/9 + tauri build): `pnpm tauri dev` 수동 검증 필요 — Plan §6.3 체크리스트 참조
