# Quick Reference

## 커맨드

```bash
# 실행
python main.py

# 의존성 설치
pip install -r requirements.txt

# 빌드 (Windows exe)
build.bat
# → dist/MR Extractor.exe
```

## 버전 제약

```
demucs>=4,<5        pydub>=0.25,<1      static-ffmpeg>=2,<3
sounddevice>=0.4,<1 librosa>=0.10,<1    yt-dlp (무제한)
```

## demucs CLI (내부 사용)

```bash
python -m demucs -n htdemucs_ft --out <tmp_dir> <input_file>

# GPU 메모리 부족 시
python -m demucs -n htdemucs_ft --segment 7 --out <tmp_dir> <input_file>

# 노래방 모드 (vocals + no_vocals)
python -m demucs -n htdemucs_ft --two-stems=vocals --out <tmp_dir> <input_file>
```

결과: `<tmp_dir>/<model_name>/<track_name>/vocals.wav, drums.wav, bass.wav, other.wav`
⚠ 경로에 모델명 하드코딩 금지 → glob 패턴 탐색

## ffmpeg (영상 → 오디오 추출)

```bash
ffmpeg -i <video> -vn -acodec pcm_s16le -ar 44100 -ac 2 -y <output.wav>
```

## 앱 데이터 경로

```
출력 폴더:     ~/Desktop/MR Extractor/
히스토리:      %APPDATA%/MR Extractor/history.json
모델 캐시:     ~/.cache/torch/hub/checkpoints/
```

## 다크 테마 색상

```
BG=#0d0d1a  SURFACE=#16162e  BORDER=#2a2a4e  ACCENT=#7c5cfc
SUCCESS=#3dd68c  WARN=#f0a500  DANGER=#e05c5c  MUTED=#8888aa
```

## 단축키 (주요)

| 키        | 동작          |
| --------- | ------------- |
| Space     | 재생/일시정지 |
| ←/→       | 5초 이동      |
| Shift+←/→ | 30초 이동     |
| ↑/↓       | 볼륨 ±5       |
| M         | 전체 뮤트     |
| Ctrl+O    | 파일 열기     |
| Ctrl+H    | 히스토리      |
| Ctrl+,    | 설정          |
| Esc       | 뒤로/닫기     |
