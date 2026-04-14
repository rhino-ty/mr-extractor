# 지원 파일 포맷

## 오디오 (직접 분리)

```
.mp3  .wav  .flac  .ogg  .m4a  .aac  .opus  .aiff  .wma  .ape
```

## 영상 (오디오 자동 추출 후 분리)

```
.mp4  .mkv  .mov  .avi  .webm  .wmv  .flv  .ts  .m2ts
```

ffmpeg가 있으므로 사실상 ffmpeg가 지원하는 모든 포맷 가능.

## 처리 흐름

```
오디오 파일 → demucs 직접 투입
영상 파일   → ffmpeg로 오디오 추출(WAV) → demucs 투입 → tmp WAV 삭제
```

영상 파일은 FileCard에서 아이콘으로 구분.
큐에 추가 시 "(영상에서 추출)" 표시.

## DropZone 확장자 분류

```python
AUDIO_EXTS = {".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac",
              ".opus", ".aiff", ".wma", ".ape"}
VIDEO_EXTS = {".mp4", ".mkv", ".mov", ".avi", ".webm",
              ".wmv", ".flv", ".ts", ".m2ts"}
ALL_EXTS   = AUDIO_EXTS | VIDEO_EXTS
```

파일 타입 아이콘: 🔗 URL / 🎵 오디오 / 🎬 영상
