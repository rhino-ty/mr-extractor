# Model Selector

## Purpose

QueuePage의 demucs 모델 선택 드롭다운 + 도움말 패널 스펙.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 기본값: `htdemucs_ft`
- Svelte 컴포넌트: `ModelSelector.svelte`

---

## 드롭다운 위치

QueuePage 설정 섹션:
```
모델 선택   [htdemucs_ft  ▼]  [?]
```

## 모델 목록

| 값 | 표시명 | 크기 | 기본 설치 | 툴팁 |
|---|---|---|:---:|---|
| `htdemucs` | HTDemucs | ~80 MB | ❌ | 빠르고 안정적. 보컬/드럼/베이스/기타 4트랙 분리. |
| `htdemucs_ft` | HTDemucs FT ⭐ 권장 | ~1.3 GB | ✅ | 최고 품질 (SDR 9.20dB). Bag of 4 (소스별 전용 모델 4개 실행 = 처리 시간 약 4배). setup-page에서 자동 설치. |
| `htdemucs_6s` | HTDemucs 6스템 | ~300 MB | ❌ | 기타·피아노 트랙 추가 분리 (총 6트랙). 피아노는 bleeding/아티팩트 있음 (실험적). |

> **2026-04-24 추가**: 크기 + 기본 설치 여부 컬럼. setup-page는 기본 모델(htdemucs_ft)만 설치. 나머지는 아래 on-demand 플로우.

---

## On-Demand 다운로드 플로우 *(2026-04-24 신규 추가)*

setup-page Plan FR-15: "기본 모델 외 추가 모델은 setup-page가 아닌 이 ModelSelector 피처가 책임."

### 플로우

```
사용자가 드롭다운에서 미설치 모델 선택
  ↓
1. 모델 크기 동적 probing (common::probe_url_size, setup-page에서 구현됨)
  ↓
2. 디스크 공간 체크 (common::check_disk_space, estimate × 1.5 임계값)
  ↓
  ├─ 충분 → 3. 확인 다이얼로그 ("HTDemucs 6스템 (~300MB) 다운로드 하시겠습니까?")
  │        ↓
  │        4. 다운로드 진행 (Channel 진행률)
  │        ↓
  │        5. 완료 → 해당 모델로 설정
  │
  └─ 부족 → 디스크 부족 안내 (UX_BEHAVIORS.md §디스크 공간 부족 화면 재사용)
```

### 드롭다운 UI (2026-04-24 수정)

```
모델 선택
  ┌─────────────────────────────────┐
  │ HTDemucs FT ⭐ (현재 사용 중)    │
  │ HTDemucs          (~80 MB 필요)  │ ← 미설치
  │ HTDemucs 6스템     (~300 MB 필요) │ ← 미설치
  └─────────────────────────────────┘
```

### 재사용 API (setup-page에서 제공)

- `common::probe_url_size(url)` — 모델 파일 크기 동적 조회
- `common::check_disk_space(required_mb)` — 디스크 여유 체크
- `common::app_data_dir()` + `torch-cache/hub/checkpoints/` — 모델 저장 경로

모두 `src-tauri/src/commands/common.rs`에 있음 (setup-page가 도입).

## [?] 도움말 패널

버튼 클릭 시 인라인 패널 토글. Svelte `transition:slide`:

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

- [COMMANDS.md](COMMANDS.md) — separate 커맨드에서 모델명 전달
