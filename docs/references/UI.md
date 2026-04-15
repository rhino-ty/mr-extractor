# UI

## Purpose

PlayerPage 레이아웃, Web Audio 오디오 아키텍처, UX 퀄리티 가이드.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 판매용 수준 완성도 — 빈 화면 절대 금지
- 모든 액션에 즉각적인 시각 피드백 (< 100ms)
- **다크 테마 전용** (라이트 모드 없음 — 오디오 앱 업계 표준)
- Web Audio API로 오디오 처리 (싱글턴 AudioContext)

---

## PlayerPage 레이아웃

```
┌─────────────────────────────────────────────────────┐
│  ← 목록                    소란 - 사랑한 마음엔...    │
├─────────────────────────────────────────────────────┤
│  ▁▂▃▅▇▅▃▂▁▂▃▅▇▅▃▂▁  (wavesurfer.js 파형)          │
│  ████████████████░░░░░░░░░░░  2:14 / 3:42          │
│         [◀◀]  [▶ 재생]  [■ 정지]                    │
├─────────────────────────────────────────────────────┤
│  🎤 Vocals  [M]  ━━━━━━━━━●━━━  85                 │
│  🥁 Drums   [M]  ━━━━━━━━━━━━━●  100               │
│  🎸 Bass    [M]  ━━━━━━━●━━━━━  70                 │
│  🎹 Other   [M]  ━━━━━━━━━━━━━●  100               │
│  (htdemucs_6s 선택 시 아래 2트랙 추가)               │
│  🎸 Guitar  [M]  ━━━━━━━━━━━━━●  100               │
│  🎹 Piano   [M]  ━━━━━━━━━━━━━●  100               │
├─────────────────────────────────────────────────────┤
│  A-B 구간  [A ●━━━━━━━━━━━━● B]  [🔁 반복]           │
├─────────────────────────────────────────────────────┤
│  키 조절   [-12 ━━━━━━●━━━━━━ +12]  +2반음          │
│            Tone.js 실시간 미리듣기 / ffmpeg 내보내기  │
├─────────────────────────────────────────────────────┤
│              [💾 현재 믹스 내보내기]                  │
└─────────────────────────────────────────────────────┘
```

## Web Audio 아키텍처

```
AudioContext (싱글턴)
  ├── vocals: AudioBufferSourceNode → GainNode ──┐
  ├── drums:  AudioBufferSourceNode → GainNode ──┤
  ├── bass:   AudioBufferSourceNode → GainNode ──┼→ Tone.PitchShift → destination
  └── other:  AudioBufferSourceNode → GainNode ──┘
```

- `GainNode.gain.value` → 볼륨 (0~1)
- `gain.value = 0` → 뮤트
- wavesurfer.js → 파형 렌더링 + 재생 위치 싱크

### wavesurfer.js 통합

```typescript
import WaveSurfer from 'wavesurfer.js';

const wavesurfer = WaveSurfer.create({
    container: '#waveform',
    waveColor: '#8888aa',
    progressColor: '#7c5cfc',
    cursorColor: '#7c5cfc',
    height: 80,
    barWidth: 2,
    barGap: 1,
});
```

## UX 퀄리티 기준

### 필수

- 모든 버튼에 `transition-all duration-200` hover 효과
- 로딩 중 스피너 표시 (빈 화면 절대 금지)
- 에러 시 친절한 한국어 메시지 + 원인 힌트
- 툴팁: `title` 또는 커스텀 컴포넌트 (hover 0.5초 후)
- 빈 상태: 일러스트 + 안내 문구
- 페이지 전환: Svelte `transition:fade|slide` (200ms)

### 폰트/간격 (Tailwind)

- 타이틀: `text-xl font-bold`
- 섹션: `text-sm font-semibold`
- 본문: `text-sm`
- 캡션: `text-xs text-muted`
- 카드 padding: `p-4` (16px)
- 아이콘 + 텍스트 gap: `gap-2` (8px)

### 다중 선택 UI

큐/히스토리 목록에서:
- `Ctrl+클릭`: 개별 선택/해제
- `Shift+클릭`: 범위 선택
- 선택된 항목 하이라이트 (`bg-accent/20`)
- 하단 액션 바: `[🗑 삭제 (3)]  [💾 내보내기 (3)]`

## Related Docs

- [SHORTCUTS.md](SHORTCUTS.md) — 단축키
- [SETTINGS.md](SETTINGS.md) — 설정 UI
- [HISTORY.md](HISTORY.md) — 히스토리 UI
