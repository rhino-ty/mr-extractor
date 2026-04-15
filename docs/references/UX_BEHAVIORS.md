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

## 첫 실행 UX (SetupPage)

**원칙: 사용자는 기술 용어를 모른다. 기다리기만 하면 된다.**

### 첫 실행 플로우

```
앱 실행 → SetupPage (자동)
  ├─ 환경 감지 (0.5초)
  ├─ sidecar 확인 (ffmpeg, yt-dlp) → 즉시 ✅
  ├─ Embedded Python 확인 → 즉시 ✅
  ├─ demucs 미설치 감지 → 자동 pip install 시작
  │     "음원 분리 엔진 설치 중..." (진행률 바, 1~2분)
  ├─ 모델 미다운로드 감지 → 자동 다운로드
  │     "AI 모델 다운로드 중..." (진행률 바, 1~2분)
  └─ 완료 → 1초 후 QueuePage (클릭 0회)
```

### SetupPage 화면 상태

**설치 중:**
```
┌─────────────────────────────────────────────────────┐
│              🎵 MR Extractor                         │
│                                                      │
│         앱을 사용할 준비를 하고 있어요...               │
│                                                      │
│  ✅ 오디오 변환 도구                                  │
│  ✅ 유튜브 다운로더                                   │
│  ✅ 실행 환경                                         │
│  ⏳ 음원 분리 엔진       설치 중...                    │
│  ○  AI 모델                                          │
│                                                      │
│  ━━━━━━━━━━━━━━━━●━━━━━━━━━  67%                    │
│                                                      │
│  처음 실행 시 한 번만 설치됩니다.                      │
│  인터넷 연결이 필요해요. (약 2~3분)                    │
│                                                      │
└─────────────────────────────────────────────────────┘
```

**이미 설치됨 (2회차 이후):**
```
┌─────────────────────────────────────────────────────┐
│              🎵 MR Extractor                         │
│                                                      │
│         모든 준비가 완료되었어요! ✨                   │
│                                                      │
│  ✅ 오디오 변환 도구                                  │
│  ✅ 유튜브 다운로더                                   │
│  ✅ 실행 환경                                         │
│  ✅ 음원 분리 엔진                                    │
│  ✅ AI 모델                                          │
│                                                      │
│              (1초 후 자동 진입)                        │
└─────────────────────────────────────────────────────┘
```

**설치 실패:**
```
┌─────────────────────────────────────────────────────┐
│              ⚠ 설치 중 문제가 발생했어요               │
│                                                      │
│  ✅ 오디오 변환 도구                                  │
│  ✅ 유튜브 다운로더                                   │
│  ✅ 실행 환경                                         │
│  ❌ 음원 분리 엔진                                    │
│                                                      │
│  인터넷 연결을 확인하고 다시 시도해주세요.              │
│  계속 문제가 발생하면 아래 오류 정보를 보내주세요.       │
│                                                      │
│  [▼ 오류 상세 보기]                                   │
│  ┌─────────────────────────────────────────────┐     │
│  │ pip install demucs failed: ...               │     │
│  └─────────────────────────────────────────────┘     │
│                                                      │
│       [🔄 다시 시도]      [📋 오류 복사]              │
└─────────────────────────────────────────────────────┘
```

**인터넷 없음:**
```
┌─────────────────────────────────────────────────────┐
│              📡 인터넷 연결이 필요해요                 │
│                                                      │
│  처음 사용하시는 경우                                 │
│  음원 분리에 필요한 파일을 다운로드해야 합니다.         │
│  Wi-Fi 또는 유선 인터넷에 연결해주세요.                │
│                                                      │
│              [🔄 다시 확인]                           │
└─────────────────────────────────────────────────────┘
```

### UX 규칙

- 기술 용어 노출 금지 (Python, pip, PyTorch, demucs 등)
- 진행률 바 항상 표시 — 멈춰 보이면 안 됨
- 설치 완료 후 1초 대기 → 자동 페이지 전환 (클릭 불필요)
- 오류 상세는 접힌 상태 기본 → [▼] 클릭 시 펼침 (개발자용)
- [📋 오류 복사] → 클립보드에 복사 → "복사되었습니다" 토스트
- 2회차 이후: 모두 ✅ → 1초 후 자동 진입 (빠른 사용자 경험)

### 업데이트 알림 (2회차 이후, 비강제)

설정 페이지에서만 표시. SetupPage에서 강제 업데이트 안 함.
```
트레이 알림: "더 나은 음원 분리를 위한 업데이트가 있어요."
→ 설정 > 업데이트에서 선택적 실행
```

## Related Docs

- [COMMANDS.md](COMMANDS.md) — setup.rs 환경 감지 + 자동 설치 커맨드
- [HISTORY.md](HISTORY.md) — 중복 감지와 히스토리 연동
