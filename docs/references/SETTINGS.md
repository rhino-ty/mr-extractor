# Settings

## Purpose

SettingsPage 전체 스펙: 환경 상태, 저장 정책, 저장 공간, 알림 설정.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 메인 화면 우상단 ⚙ 버튼으로 진입
- 설정 저장: Tauri Store 플러그인 (`settings.json`)
- 기본 저장 정책: `keep_all` (스템 전체 저장)

---

## 환경 상태 패널

```
┌─────────────────────────────────────────────────────────┐
│  환경 상태                                [🔄 새로고침]  │
├──────────────┬──────────┬──────────────────────────────┤
│  도구         │ 버전      │ 상태                        │
├──────────────┼──────────┼──────────────────────────────┤
│  Python      │ 3.11.5   │ ✅ 설치됨                    │
│  demucs      │ 4.0.1    │ ✅ 설치됨                    │
│  ffmpeg      │ 6.1      │ ✅ 설치됨                    │
│  yt-dlp      │ 2026.03  │ 🔄 업데이트 가능             │
│  AI 모델     │ htdemucs_ft │ ✅ 캐시됨 (1.3 GB)       │
└──────────────┴──────────┴──────────────────────────────┘
```

Rust `check_environment` 커맨드로 확인. setup-page에서 사용하는 `EnvStatus` 재사용.

> **2026-04-24 수정**: `AI 모델` row 추가. setup-page Plan v0.4에서 `check_environment`가 5개 항목(ffmpeg/yt-dlp/python/demucs/**model**)을 반환하도록 확정됨. Settings 패널은 이 결과를 그대로 표시.

---

## 저장 정책

```typescript
type StoragePolicy = 'keep_all' | 'session_only' | 'inst_only';
```

- `keep_all`: 스템 전체 저장 (기본값, 곡당 약 200~600 MB)
- `session_only`: 재생 후 스템 자동 삭제, 반주만 유지
- `inst_only`: 반주 MP3/WAV만 저장 (곡당 약 10~30 MB)

---

## 저장 공간

```
┌──────────────────────────────────────────────────────────┐
│  저장 공간                                                │
├──────────────────────────────────────────────────────────┤
│  출력 폴더     ~/Desktop/MR Extractor/   [📁 폴더 열기]  │
│  전체 사용량   2.3 GB  (14개 파일)                        │
│                                                          │
│    스템 파일   1.8 GB  ███████████████████████████  78%  │
│    반주 파일   0.4 GB  █████  17%                        │
│    기타        0.1 GB  █  5%                             │
│                                                          │
│  [🗑 전체 삭제]      [🧹 고아 파일 정리]                  │
└──────────────────────────────────────────────────────────┘
```

- **고아 파일 정리**: 히스토리에 없는 파일 탐지 → 선택 삭제
- 용량 계산: Rust 커맨드로 비동기 스캔 (`common::dir_size`, setup-page에서 제공)

### AI 모델 관리 *(2026-04-24 신규 추가 — setup-page Plan §9.2 후속)*

```
┌──────────────────────────────────────────────────────────┐
│  AI 모델                                          ℹ️       │
├──────────────────────────────────────────────────────────┤
│  전체 사용량:   1.3 GB                                    │
│                                                          │
│  ✅ HTDemucs FT  (현재 사용)   1.3 GB  [🗑 삭제]         │
│  ○ HTDemucs                    —     [⬇ 설치 (~80MB)]   │
│  ○ HTDemucs 6스템              —     [⬇ 설치 (~300MB)]  │
└──────────────────────────────────────────────────────────┘
```

- 실측 크기: `common::dir_size(torch-cache/hub/checkpoints/)` per 모델
- 설치: ref MODEL_SELECTOR.md §on-demand 다운로드 플로우 재사용
- 삭제: 해당 모델 파일 제거. 현재 사용 중 모델 삭제 시 경고 다이얼로그 + 기본 모델로 폴백.
- 버튼의 `~80MB`/`~300MB`는 **동적 probing 결과** (하드코딩 금지, Plan FR-14 준수)

### 앱 전용 데이터 폴더 *(2026-04-24 개선)*

```
%APPDATA%/com.rhinoty.mr-extractor/
  ├── venv/              ← 음원 분리 엔진 (실행 환경)
  ├── torch-cache/       ← AI 모델 (위 AI 모델 관리에서 표시)
  └── .setup-complete    ← 설치 마커
```

경로는 `common::app_data_dir()` 기반. Rust가 Python subprocess 실행 시 `TORCH_HOME` 환경변수 주입으로 홈 폴더 오염 방지 (setup-page Plan FR-12).

이전 `~/.cache/torch/`의 "약 300~400MB"는 단일 모델 기준 오표기. htdemucs_ft는 Bag of 4라 실제 ~1.3GB.

---

## 알림 설정

| 설정 | 기본값 |
|---|---|
| 처리 완료 시 시스템 알림 | ON |
| 오류 발생 시 알림 | ON |

구현: `@tauri-apps/plugin-notification`

---

## 내보내기 포맷 설정

| 설정 | 기본값 |
|---|---|
| 기본 포맷 | WAV |
| MP3 비트레이트 | 320 kbps |
| 출력 폴더 | `~/Desktop/MR Extractor/` |

## Related Docs

- [COMMANDS.md](COMMANDS.md) — check_environment, export_mix
- [HISTORY.md](HISTORY.md) — 히스토리 삭제 연동
- [SHORTCUTS.md](SHORTCUTS.md) — Ctrl+, 설정 열기
