# MR Extractor

유튜브 URL, 오디오 파일, 영상 파일에서 보컬을 분리하고 반주(MR)를 추출하는 데스크탑 앱.

분리된 스템(보컬/드럼/베이스/기타)을 앱 내에서 직접 재생하며 파트별 볼륨과 키를 조절할 수 있습니다.

## 주요 기능

- **유튜브 URL → MR 한 방에** — URL 붙여넣기만 하면 다운로드 + 분리 자동
- **영상 파일 지원** — MP4/MKV 등 드래그&드롭 → 오디오 자동 추출 후 분리
- **스템 믹서** — 보컬/드럼/베이스/기타 볼륨 실시간 조절하며 재생
- **파형 시각화** — wavesurfer.js 기반 오디오 파형 표시
- **키 조절** — 반음 단위 ±12 피치 시프트
- **다중 선택** — 큐/히스토리에서 여러 항목 선택 후 일괄 삭제/내보내기
- **모델 선택** — HTDemucs / HTDemucs FT / HTDemucs 6스템

## 스크린샷

> (추후 추가)

## 개발

```bash
# 의존성 설치
pnpm install

# 개발 모드
pnpm tauri dev

# 프로덕션 빌드
pnpm tauri build
```

### 개발 환경

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install)

### 사용자 환경 (앱에서 자동 감지 + SetupPage 안내)

- [Python](https://www.python.org/) 3.8+ + `pip install demucs` (음원 분리)
- ffmpeg, yt-dlp는 앱에 sidecar로 번들됨 (별도 설치 불필요)

## 기술 스택

| 항목 | 선택 |
|---|---|
| 앱 프레임워크 | Tauri v2 (Rust) |
| 프론트엔드 | Svelte 5 + Tailwind CSS |
| 오디오 재생/믹싱 | Web Audio API |
| 파형 시각화 | wavesurfer.js |
| 음원 분리 | demucs (CLI subprocess) |
| 유튜브 다운로드 | yt-dlp (CLI subprocess) |
| 영상 처리 | ffmpeg (CLI subprocess) |
| 키 조절 (미리듣기) | Tone.js PitchShift (Web Audio) |
| 키 조절 (내보내기) | ffmpeg rubberband 필터 |
| 패키지 매니저 | pnpm |

## 지원 포맷

**오디오**: mp3, wav, flac, ogg, m4a, aac, opus, aiff, wma, ape

**영상**: mp4, mkv, mov, avi, webm, wmv, flv, ts, m2ts

## 프로젝트 구조

```
mr_extractor/
├── src/                     # Svelte 프론트엔드
│   ├── App.svelte
│   ├── lib/                 # commands, stores, types
│   ├── pages/               # Setup, Queue, Process, Player, History, Settings
│   └── components/          # DropZone, StemMixer, WaveformPlayer, ...
├── src-tauri/               # Rust 백엔드
│   ├── src/commands/        # setup, youtube, separate, video, export
│   └── tauri.conf.json
└── docs/
    ├── INDEX.md
    ├── QUICK_REF.md
    ├── ROADMAP.md
    └── references/
```

## 라이선스

> (추후 추가)
