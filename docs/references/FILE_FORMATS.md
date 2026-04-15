# File Formats

## Purpose

지원 파일 포맷 정의 및 DropZone 확장자 분류.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 오디오 → demucs 직접 투입
- 영상 → ffmpeg로 WAV 추출 → demucs → tmp WAV 삭제
- demucs 내부: `torchaudio.load()` 시도 → 실패 시 ffmpeg 폴백
- 모델은 44100Hz 스테레오 기준. 다른 샘플레이트/모노는 자동 변환됨

---

## 오디오 (직접 분리)

```
.mp3  .wav  .flac  .ogg  .m4a  .aac  .opus  .aiff  .wma  .ape
```

## 영상 (오디오 추출 후 분리)

```
.mp4  .mkv  .mov  .avi  .webm  .wmv  .flv  .ts  .m2ts
```

## DropZone 확장자 (프론트엔드)

```typescript
const AUDIO_EXTS = new Set(['.mp3', '.wav', '.flac', '.ogg', '.m4a', '.aac',
                            '.opus', '.aiff', '.wma', '.ape']);
const VIDEO_EXTS = new Set(['.mp4', '.mkv', '.mov', '.avi', '.webm',
                            '.wmv', '.flv', '.ts', '.m2ts']);
const ALL_EXTS = new Set([...AUDIO_EXTS, ...VIDEO_EXTS]);
```

## 아이콘 구분

- 🔗 URL (유튜브)
- 🎵 오디오 파일
- 🎬 영상 파일 — 큐에 추가 시 "(영상에서 추출)" 표시

## 내보내기 포맷

| 포맷 | 용도 |
|---|---|
| WAV (16bit, 44100Hz) | 기본 출력, 무손실 |
| MP3 (320kbps) | 공유용, 용량 작음 |
| FLAC | 무손실 압축 |

## Related Docs

- [COMMANDS.md](COMMANDS.md) — video_extract, separate, export 커맨드
