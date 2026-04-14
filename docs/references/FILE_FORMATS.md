# File Formats

## Purpose

지원 파일 포맷 정의 및 DropZone 확장자 분류.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 오디오 → demucs 직접 투입
- 영상 → ffmpeg로 WAV 추출 → demucs → tmp WAV 삭제

---

## 오디오 (직접 분리)

```
.mp3  .wav  .flac  .ogg  .m4a  .aac  .opus  .aiff  .wma  .ape
```

## 영상 (오디오 추출 후 분리)

```
.mp4  .mkv  .mov  .avi  .webm  .wmv  .flv  .ts  .m2ts
```

## DropZone 확장자 분류

```python
AUDIO_EXTS = {".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac",
              ".opus", ".aiff", ".wma", ".ape"}
VIDEO_EXTS = {".mp4", ".mkv", ".mov", ".avi", ".webm",
              ".wmv", ".flv", ".ts", ".m2ts"}
ALL_EXTS   = AUDIO_EXTS | VIDEO_EXTS
```

## 아이콘 구분

- 🔗 URL (유튜브)
- 🎵 오디오 파일
- 🎬 영상 파일 — 큐에 추가 시 "(영상에서 추출)" 표시

## Related Docs

- [WORKERS.md](WORKERS.md) — VideoExtractWorker, SeparationWorker
