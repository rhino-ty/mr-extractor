# Archive Index — 2026-05

> PDCA 완료 후 정리된 피처 문서들. 각 피처는 `{feature}/` 디렉토리에 plan/design/analysis/report 4종 문서로 보존.

## Features

| Feature | Match Rate | SC | Critical | Important | Archived | Path |
|---|---:|---:|---:|---:|---|---|
| queue-page | 99.5% | 21/21 | 0 | 0 | 2026-05-11 | [queue-page/](queue-page/) |

## queue-page

- **요약**: 사용자 메인 허브. URL/파일 입력 → 큐 적재 → 다중 선택 + [▶ 분리 시작] → ProcessPage 라우팅.
- **기간**: 2026-04-29 → 2026-05-11 (~8.5h 작업)
- **PDCA**: Plan v0.6 → Design v0.4 → Do (Phase 1+2+3) → Check 97.6% → Iterate 1 → 99.5%
- **Decision Record**: 13/13 준수 (K1-K13, Option C — Pragmatic Balance)
- **Foundation 재사용**: setup-page common::§5 Error Translation + §6 Process Helpers + queue_tmp_dir
- **다음 피처 dependency**: ProcessPage (queue-page 사용자 흐름의 종착지)
- **Documents**:
  - [Plan v0.6](queue-page/queue-page.plan.md)
  - [Design v0.4](queue-page/queue-page.design.md)
  - [Analysis (Iterate 1)](queue-page/queue-page.analysis.md)
  - [Report](queue-page/queue-page.report.md)
