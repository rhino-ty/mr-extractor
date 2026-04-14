# Shortcuts

## Purpose

앱 전역 단축키 정의. `QShortcut`으로 MainWindow에 등록.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 버튼 툴팁에 단축키 병기: `"재생 (Space)"`
- 설정 페이지에 단축키 목록 섹션 포함

---

## 단축키 목록

| 단축키                    | 동작                   | 스코프     |
| ------------------------- | ---------------------- | ---------- |
| `Space`                   | 재생/일시정지 토글     | PlayerPage |
| `←` / `→`                 | 5초 뒤로/앞으로        | PlayerPage |
| `Shift + ←` / `Shift + →` | 30초 이동              | PlayerPage |
| `↑` / `↓`                 | 마스터 볼륨 +5 / -5    | PlayerPage |
| `M`                       | 전체 뮤트 토글         | PlayerPage |
| `Ctrl + O`                | 파일 열기 다이얼로그   | 전역       |
| `Ctrl + H`                | 히스토리 패널 토글     | 전역       |
| `Ctrl + ,`                | 설정 페이지 열기       | 전역       |
| `Escape`                  | 모달/도움말 닫기, 뒤로 | 전역       |

## Related Docs

- [UI.md](UI.md) — PlayerPage 레이아웃
- [SETTINGS.md](SETTINGS.md) — 설정에 단축키 목록 표시
