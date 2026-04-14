# History

## Purpose

처리 히스토리 JSON 구조, UI, 뱃지 시스템, 삭제 다이얼로그 스펙.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 저장 위치: `%APPDATA%/MR Extractor/history.json`
- 페이지 열릴 때 백그라운드로 파일 존재 확인

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
│  🎵  소란 - 사랑한 마음엔 죄가 없다            2026.04.12 │
│      htdemucs_ft  ·  파일                                │
│      [🎚 스템 있음]                                       │
│      [▶ 재생]  [📁 폴더 열기]  [🔄 재처리]  [🗑]         │
└──────────────────────────────────────────────────────────┘
```

## 뱃지

| 뱃지         | 조건                   | 색상   | 클릭 동작                        |
| ------------ | ---------------------- | ------ | -------------------------------- |
| 🎚 스템 있음 | 스템 wav 4개 모두 존재 | ACCENT | PlayerPage 오픈                  |
| 🎵 반주만    | instrumental만 존재    | MUTED  | "재처리하면 스템 생성 가능" 툴팁 |
| ⚠ 파일 없음  | 출력 파일 모두 삭제됨  | WARN   | —                                |

## 항목 액션

- **▶ 재생** → 스템 있으면 믹서 PlayerPage, 반주만이면 심플 플레이어
- **📥 반주 받기** → instrumental 파일 복사/저장
- **📁 폴더 열기** → `os.startfile(out_dir)`
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
│    삭제될 파일 (총 234 MB):                        │
│      · 소란_instrumental.wav       180 MB         │
│      · ...                                       │
│              [취소]      [🗑 삭제]                │
└──────────────────────────────────────────────────┘
```

## Related Docs

- [SETTINGS.md](SETTINGS.md) — 저장 정책, 저장 공간 관리
- [UX_BEHAVIORS.md](UX_BEHAVIORS.md) — 중복 파일 처리
