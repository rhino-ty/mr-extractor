# Model Selector

## Purpose

QueuePage의 demucs 모델 선택 드롭다운 + [?] 도움말 패널 스펙.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 기본값: `htdemucs_ft`
- 툴팁은 `setItemData(index, tooltip, Qt.ItemDataRole.ToolTipRole)`

---

## 드롭다운 위치

QueuePage 설정 섹션:

```
모델 선택   [htdemucs_ft  ▼]  [?]
```

## 모델 목록

| 값            | 표시명              | 툴팁                                                                    |
| ------------- | ------------------- | ----------------------------------------------------------------------- |
| `htdemucs`    | HTDemucs (기본)     | 빠르고 안정적. 보컬/드럼/베이스/기타 4트랙 분리. 일반적인 용도에 적합.  |
| `htdemucs_ft` | HTDemucs FT ⭐ 권장 | Fine-tuned 버전. 현재 최고 품질 (SDR 9.20dB). 소스별 전용 모델 4개를 실행하여 **처리 시간 약 4배**. |
| `htdemucs_6s` | HTDemucs 6스템      | 기타·피아노 트랙 추가 분리 (총 6트랙). 피아노는 bleeding/아티팩트 있음 (실험적). |

## [?] 도움말 패널

`QToolButton` + `QFrame` 토글. 애니메이션으로 부드럽게 펼치기.

```
┌──────────────────────────────────────────────────────┐
│  모델 선택 가이드                               [✕]   │
│                                                      │
│  🏆 HTDemucs FT (권장)                               │
│     현재 가장 높은 분리 품질. 대부분의 경우 이걸 쓰세요. │
│     단, 처리 시간이 기본 모델의 약 4배 소요.             │
│                                                      │
│  ⚡ HTDemucs (기본)                                   │
│     FT보다 약간 빠름. 빠른 미리듣기용으로 적합.        │
│                                                      │
│  🎸 HTDemucs 6스템                                   │
│     기타·피아노까지 따로 분리하고 싶을 때.              │
│     단, 피아노 분리 품질은 아직 완벽하지 않음.          │
└──────────────────────────────────────────────────────┘
```

## Related Docs

- [WORKERS.md](WORKERS.md) — SeparationWorker에서 모델명 사용
