# UX Behaviors

## Purpose

앱 종료 안전 처리, 중복 파일 감지, 첫 실행 UX 동작 스펙.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 처리 중이 아니면 확인 없이 즉시 종료
- 중복 기본값: 건너뜀 (추가 안 함)
- URL 정규화: YouTube `v=` 파라미터만 보존

---

## 앱 종료 안전 처리

처리 중 창 닫기(X 버튼) 또는 Alt+F4:

```
┌──────────────────────────────────────┐
│  ⚠ 처리 중입니다                      │
│  2개 파일을 처리 중입니다.              │
│  지금 종료하면 작업이 취소됩니다.        │
│  [계속 처리]   [취소 후 종료]           │
└──────────────────────────────────────┘
```

```python
def closeEvent(self, event):
    if self._is_processing():
        reply = QMessageBox.question(...)
        if reply == QMessageBox.StandardButton.Yes:
            self._cancel_all_workers()
            event.accept()
        else:
            event.ignore()
    else:
        event.accept()
```

## 중복 파일 처리

같은 파일/URL 큐에 재추가 시:

```
┌──────────────────────────────────────────┐
│  이미 추가된 파일입니다                    │
│  소란 - 사랑한 마음엔 죄가 없다.mp3        │
│  ○ 건너뜀 (추가 안 함)     ← 기본          │
│  ○ 중복 추가 (같은 파일 두 번 처리)        │
│  □ 앞으로 이 선택 기억하기                 │
│               [확인]                      │
└──────────────────────────────────────────┘
```

URL 정규화:

```python
def normalize_url(url):
    from urllib.parse import urlparse, parse_qs
    parsed = urlparse(url)
    if "youtube" in parsed.netloc or "youtu.be" in parsed.netloc:
        v = parse_qs(parsed.query).get("v", [""])[0]
        return f"https://www.youtube.com/watch?v={v}"
    return url
```

## 첫 실행 UX

```
앱 실행 → 자동 설치/업데이트 → 자동 진입 (클릭 0회)
URL 붙여넣기 → 추가 → 분리 시작 → 스템 믹서로 재생
```

## Related Docs

- [HISTORY.md](HISTORY.md) — 중복 감지와 히스토리 연동
