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

Tauri `on_window_event` + `CloseRequested`:

```rust
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if is_processing() {
            api.prevent_close();
            // 프론트엔드에 확인 다이얼로그 요청
            window.emit("confirm-close", ()).ok();
        }
    }
})
```

프론트엔드:
```svelte
<script>
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

let showConfirm = $state(false);

$effect(() => {
    listen('confirm-close', () => { showConfirm = true; });
});

function confirmClose() {
    cancelAllTasks();
    getCurrentWindow().destroy();
}
</script>
```

## 중복 파일 처리

같은 파일/URL 큐에 재추가 시 모달:

```
○ 건너뜀 (추가 안 함)     ← 기본
○ 중복 추가 (같은 파일 두 번 처리)
□ 앞으로 이 선택 기억하기
```

URL 정규화:
```typescript
function normalizeUrl(url: string): string {
    const parsed = new URL(url);
    if (parsed.hostname.includes('youtube') || parsed.hostname.includes('youtu.be')) {
        const v = parsed.searchParams.get('v') || '';
        return `https://www.youtube.com/watch?v=${v}`;
    }
    return url;
}
```

## 첫 실행 UX

```
앱 실행 → 환경 자동 감지 → 자동 진입 (클릭 0회)
URL 붙여넣기 → 추가 → 분리 시작 → 스템 믹서로 재생
```

미설치 도구 감지 시 SetupPage에서 설치 안내 표시.

## Related Docs

- [HISTORY.md](HISTORY.md) — 중복 감지와 히스토리 연동
