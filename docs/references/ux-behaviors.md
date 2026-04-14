# UX 동작 스펙

## 앱 종료 안전 처리

처리 중 창 닫기(X 버튼) 또는 Alt+F4 시:

```
┌──────────────────────────────────────┐
│  ⚠ 처리 중입니다                      │
│                                      │
│  2개 파일을 처리 중입니다.              │
│  지금 종료하면 작업이 취소됩니다.        │
│                                      │
│  [계속 처리]   [취소 후 종료]           │
└──────────────────────────────────────┘
```

구현:

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

처리 중이 아니면 확인 없이 즉시 종료.

---

## 중복 파일 처리

같은 파일/URL을 큐에 다시 추가하려 할 때:

```
┌──────────────────────────────────────────┐
│  이미 추가된 파일입니다                    │
│                                          │
│  소란 - 사랑한 마음엔 죄가 없다.mp3        │
│                                          │
│  ○ 건너뜀 (추가 안 함)     ← 기본          │
│  ○ 중복 추가 (같은 파일 두 번 처리)        │
│                                          │
│  □ 앞으로 이 선택 기억하기                 │
│                                          │
│               [확인]                      │
└──────────────────────────────────────────┘
```

URL은 정규화 후 비교 (`?si=...` 같은 트래킹 파라미터 제거):

```python
def normalize_url(url):
    from urllib.parse import urlparse, urlencode, parse_qs
    parsed = urlparse(url)
    # YouTube: v= 파라미터만 남김
    if "youtube" in parsed.netloc or "youtu.be" in parsed.netloc:
        v = parse_qs(parsed.query).get("v", [""])[0]
        return f"https://www.youtube.com/watch?v={v}"
    return url
```

---

## 첫 실행 UX

```
앱 실행 → 자동 설치/업데이트 → 자동 진입 (클릭 0회)
URL 붙여넣기 → 추가 → 분리 시작 → 스템 믹서로 재생
```
