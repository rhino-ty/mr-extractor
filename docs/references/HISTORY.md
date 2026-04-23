# History

## Purpose

처리 히스토리 JSON 구조, UI, 뱃지 시스템, 다중 선택, 삭제 다이얼로그 스펙.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 저장: Tauri Store 또는 Rust 측 JSON (`%APPDATA%/com.rhinoty.mr-extractor/history.json`)
- 페이지 열릴 때 Rust에서 파일 존재 확인 (비동기)

> **2026-04-24 수정**: Tauri bundle identifier가 `com.rhinoty.mr-extractor`로 확정됨 (`tauri.conf.json`). 이전 문서의 `com.mr-extractor.app`는 오타. `appDataDir()` API는 identifier 기반 경로를 반환하므로 전체 프로젝트에서 이 경로 준수 필요. — setup-page Plan v0.4 교차 검증에서 발견.

---

## JSON 구조

```json
{
  "entries": [
    {
      "id": "uuid",
      "date": "2026-04-12T19:30:00",
      "source_type": "youtube | file | video",
      "source": "https://youtube.com/... | /path/to/file.mp3",
      "title": "소란 - 사랑한 마음엔 죄가 없다",
      "model": "htdemucs_ft",
      "out_dir": "/path/to/output/",
      "files": {
        "wav": "/path/..._instrumental.wav",
        "mp3": "/path/..._instrumental_320k.mp3",
        "stems": {
          "vocals": "/path/vocals.wav",
          "drums": "/path/drums.wav",
          "bass": "/path/bass.wav",
          "other": "/path/other.wav"
        }
      },
      "status": "done | error",
      "error_msg": null
    }
  ]
}
```

## UI 레이아웃

```
┌──────────────────────────────────────────────────────────┐
│  처리 히스토리                      [🗑 전체 삭제]  [✕]  │
├──────────────────────────────────────────────────────────┤
│  ☐ 🎵  소란 - 사랑한 마음엔 죄가 없다         2026.04.12 │
│       htdemucs_ft  ·  파일                               │
│       [🎚 스템 있음]                                      │
│       [▶ 재생]  [📁 폴더 열기]  [🔄 재처리]  [🗑]        │
├──────────────────────────────────────────────────────────┤
│  ☐ 🔗  Adele - Hello                          2026.04.11 │
│       htdemucs_ft  ·  YouTube                            │
│       [🎵 반주만]                                         │
├──────────────────────────────────────────────────────────┤
│  선택된 항목: 2개                                         │
│  [🗑 선택 삭제]  [💾 선택 내보내기]                        │
└──────────────────────────────────────────────────────────┘
```

## 다중 선택

- `Ctrl+클릭`: 개별 선택/해제
- `Shift+클릭`: 범위 선택
- 체크박스(☐/☑) + 선택 하이라이트
- 하단 액션 바: 선택 삭제 / 선택 내보내기

## 뱃지

| 뱃지 | 조건 | 색상 | 클릭 동작 |
|---|---|---|---|
| 🎚 스템 있음 | 스템 wav 모두 존재 (4개 또는 6개) | accent | PlayerPage 오픈 |
| 🎵 반주만 | instrumental만 존재 | muted | "재처리하면 스템 생성 가능" 툴팁 |
| ⚠ 파일 없음 | 출력 파일 모두 삭제됨 | warn | — |

## 항목 액션

- **▶ 재생** → 스템 있으면 믹서 PlayerPage, 반주만이면 심플 플레이어
- **📥 반주 받기** → instrumental 파일 복사/저장 (dialog 플러그인)
- **📁 폴더 열기** → `open()` (shell 플러그인)
- **🔄 재처리** → 원본 파일/URL을 큐에 다시 추가
- **🗑 삭제** → 삭제 옵션 다이얼로그

## 삭제 다이얼로그

```
┌──────────────────────────────────────────────────┐
│  삭제                                             │
│  소란 - 사랑한 마음엔 죄가 없다                   │
│                                                  │
│  ○ 기록만 삭제 (파일 유지)                         │
│  ● 기록 + 파일 모두 삭제                           │
│    삭제될 파일 (총 234 MB): ...                    │
│              [취소]      [🗑 삭제]                │
└──────────────────────────────────────────────────┘
```

## Related Docs

- [SETTINGS.md](SETTINGS.md) — 저장 정책, 저장 공간
- [UX_BEHAVIORS.md](UX_BEHAVIORS.md) — 중복 파일 처리
