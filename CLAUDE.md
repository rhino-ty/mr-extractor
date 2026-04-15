# MR Extractor

## Project Overview

MR Extractor: Demucs 기반 보컬 분리 및 반주(MR) 추출 데스크탑 앱
Stack: Tauri v2 (Rust) + Svelte 5 + Tailwind CSS + pnpm

킬러 피처:
- 유튜브 URL → MR 한 방에 (다운로드 + 분리 자동)
- 영상 파일(MP4/MKV 등) 드래그&드롭 → 오디오 자동 추출 후 분리
- 스템 믹서: 보컬/드럼/베이스/기타 볼륨 실시간 조절 + 파형 시각화
- 키 조절: 반음 단위 ±12 피치 시프트
- 다중 선택 삭제 및 배치 내보내기

**핵심 원칙: 최소 설치로 바로 사용.**
- ffmpeg/yt-dlp: Tauri sidecar로 앱에 번들 (별도 설치 불필요)
- Python + demucs: Embedded Python 번들 + 첫 실행 시 demucs 자동 pip install
- SetupPage: 기술 용어 노출 금지. 사용자는 기다리기만 하면 됨 (클릭 0회)

## Commands

```
pnpm tauri dev       # 개발 모드 (HMR + Rust 재컴파일)
pnpm tauri build     # 프로덕션 빌드 → 설치 파일
pnpm dev             # 프론트엔드만 개발
pnpm build           # 프론트엔드만 빌드
pnpm tauri add <p>   # Tauri 플러그인 추가
```

## Project Structure

```
src/                          # Svelte 프론트엔드
  App.svelte                  # 루트 + 페이지 라우팅
  main.ts
  lib/
    commands.ts               # Tauri invoke 래퍼
    stores.ts                 # Svelte stores (상태 관리)
    types.ts                  # TypeScript 타입 정의
  pages/
    SetupPage.svelte
    QueuePage.svelte
    ProcessPage.svelte
    PlayerPage.svelte
    HistoryPage.svelte
    SettingsPage.svelte
  components/
    DropZone.svelte
    UrlInput.svelte
    FileCard.svelte
    StemMixer.svelte          # Web Audio API 믹서 + wavesurfer.js
    ModelSelector.svelte
    HistoryCard.svelte
    WaveformPlayer.svelte     # 파형 시각화
src-tauri/                    # Rust 백엔드
  Cargo.toml
  tauri.conf.json
  capabilities/
    default.json              # 권한 정의
  src/
    main.rs
    lib.rs
    commands/
      setup.rs                # 패키지 설치/업데이트
      youtube.rs              # yt-dlp 호출
      separate.rs             # demucs 호출
      video.rs                # ffmpeg 오디오 추출
      export.rs               # 믹스 내보내기
    history.rs                # 히스토리 JSON 관리
docs/
  references/                 # 상세 스펙 문서
  01-plan/ ~ 04-report/       # PDCA
```

## Key Conventions

- Svelte 5 runes: `$state`, `$derived`, `$effect` 사용
- Tailwind CSS: 다크 테마 기본, 커스텀 색상 `--color-*`
- Tauri IPC: `invoke` (요청-응답), `Channel` (진행률 스트리밍)
- import: `@tauri-apps/api/core` (invoke), `@tauri-apps/plugin-*` (기능별)
- 컴포넌트: PascalCase, `.svelte` 파일
- 타입: `src/lib/types.ts`에 중앙 관리

## Theme Colors

```css
--bg: #0d0d1a;
--surface: #16162e;
--border: #2a2a4e;
--accent: #7c5cfc;
--success: #3dd68c;
--warn: #f0a500;
--danger: #e05c5c;
--muted: #8888aa;
```

## Output Rules

```
{stem}_instrumental.wav
{stem}_instrumental_320k.mp3
기본 출력: ~/Desktop/MR Extractor/
```

## Audio Architecture

```
demucs/yt-dlp/ffmpeg → Rust (subprocess) → Channel → Svelte
Web Audio API: AudioContext → GainNode (볼륨) → BiquadFilter (EQ)
Tone.js PitchShift: 실시간 피치 조절 (미리듣기)
ffmpeg rubberband: 피치 시프트 내보내기
wavesurfer.js: 파형 시각화 + 구간 선택
```

## CLI Tools (Rust에서 subprocess 호출)

- `python -m demucs` — 음원 분리
- `yt-dlp` — 유튜브 다운로드 (sidecar 번들)
- `ffmpeg` / `ffprobe` — 영상→오디오 추출, 포맷 변환, 피치 시프트 내보내기 (sidecar 번들)

## Version Policy

```
demucs>=4,<5 | yt-dlp (sidecar 번들) | ffmpeg (sidecar 번들)
```

⚠ demucs는 `torchaudio<2.2` 제약 있음 — Python venv 격리 권장

## demucs

- `pip install demucs` 사용 (별도 포크 불필요)
- 기본 모델: `htdemucs_ft` (SDR 9.20dB, Bag of 4 모델이라 4배 느림)
- 결과 경로: `tmp/*/stem/` 패턴 탐색 (모델명 하드코딩 금지)

## Do Not

- demucs 결과 경로에 모델명 하드코딩 금지
- Tauri 플러그인 사용 시 capabilities 권한 누락 금지
- Rust `.plugin()` 등록 없이 프론트엔드에서 플러그인 호출 금지
- `@tauri-apps/api/tauri` 사용 금지 → `@tauri-apps/api/core` (v2)
- Web Audio AudioContext를 컴포넌트마다 생성 금지 → 싱글턴 관리
- ffmpeg 진행률 파싱 시 총 재생시간은 ffprobe로 사전 확인 필수

## Page Flow

```
SetupPage (자동) → QueuePage ← 메인
  ├─ "분리 시작" → ProcessPage → 완료 항목 클릭 → PlayerPage
  ├─ ⚙ → SettingsPage
  └─ 🕐 → HistoryPage
```

## References

→ 상세 스펙: [docs/references/](docs/references/)
→ 문서 지도: [docs/INDEX.md](docs/INDEX.md)
→ 로드맵: [docs/ROADMAP.md](docs/ROADMAP.md)
