# Workers

## Purpose

`app/workers.py`의 QThread 워커 클래스 상세 스펙. 신호 정의, 진행률 파싱, 코드 스니펫 포함.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 모든 워커는 QThread 상속, signal/slot으로 UI 통신
- UI 스레드에서 직접 처리 금지

---

## SetupWorker

```python
PACKAGE_CONSTRAINTS = {
    "demucs":        "demucs>=4,<5",        # ⚠ torchaudio<2.2 제약 포함
    "pydub":         "pydub>=0.25,<1",
    "static-ffmpeg": "static-ffmpeg>=2,<3",
    "yt-dlp":        "yt-dlp",              # 항상 최신
    "sounddevice":   "sounddevice>=0.4,<1",
    "librosa":       "librosa>=0.10,<1",
}
# 주의: demucs가 torchaudio<2.2를 요구하므로
# PyTorch 2.2+ 환경에서는 venv 격리 또는 demucs-infer 대안 검토

def run(self):
    # 1. 미설치 패키지 확인 → 설치
    # 2. 설치된 패키지 버전 확인 → 범위 내 업데이트
    # 3. static_ffmpeg.add_paths()
    # 4. done.emit(True, "")
```

업데이트 확인:

```python
def _needs_update(self, pkg, constraint):
    current = self._get_installed_version(pkg)
    latest_in_range = self._get_latest_in_range(pkg, constraint)
    return current != latest_in_range

def _get_latest_in_range(self, pkg, constraint):
    # pip index versions 결과 → packaging.specifiers.SpecifierSet 필터링
    ...
```

## VideoExtractWorker

영상 → WAV 추출. ffmpeg 사용.

```python
class VideoExtractWorker(QThread):
    progress = pyqtSignal(int, str)
    done = pyqtSignal(bool, str)  # success, wav_path

    def run(self):
        cmd = [
            "ffmpeg", "-i", self.video_path,
            "-vn", "-acodec", "pcm_s16le", "-ar", "44100", "-ac", "2", "-y",
            str(out_wav)
        ]
        # 진행률: "time=HH:MM:SS" 파싱. 총 재생시간은 ffprobe로 사전 확인
```

영상 처리 체이닝:

```
VideoExtractWorker → SeparationWorker → 완료
진행바: 0~20% 추출 / 20~100% 분리
```

## YtdlpWorker

```python
ydl_opts = {
    "format": "bestaudio/best",
    "outtmpl": str(Path(out_dir) / "%(title)s.%(ext)s"),
    "postprocessors": [{
        "key": "FFmpegExtractAudio",
        "preferredcodec": "mp3",
        "preferredquality": "320",
    }],
    "progress_hooks": [self._hook],
    "quiet": True,
}
```

진행률: `d["_percent_str"]` → `(\d+(?:\.\d+)?)%` 파싱.
플레이리스트: 여러 파일 → 각각 개별 큐 아이템.

## SeparationWorker

### CLI 서브프로세스 방식 (현재 설계)

```python
cmd = [sys.executable, "-m", "demucs",
       "-n", "htdemucs_ft",
       "--out", str(tmp),
       self.file_path]
# GPU 메모리 부족 시: "--segment", "7" 추가 (VRAM 3~6GB)
# 노래방 모드: "--two-stems", "vocals" 추가 (vocals + no_vocals 출력)
```

stdout char 단위 읽기 → tqdm `\r` 처리 → `(\d+)%` 파싱.
결과 경로: `tmp/*/stem_name/` 패턴 탐색.

반환:

```python
{"vocals": "path/vocals.wav", "drums": "path/drums.wav",
 "bass": "path/bass.wav", "other": "path/other.wav"}
```

### Python API 대안 (권장 — CLI 파싱보다 안정적)

```python
import demucs.api

separator = demucs.api.Separator(
    model="htdemucs_ft",
    device="cuda",           # auto: cuda > mps > cpu
    callback=self._on_progress,
    progress=False           # tqdm 비활성화
)

origin, separated = separator.separate_audio_file(self.file_path)
# separated = {"vocals": tensor, "drums": tensor, ...}

for stem, source in separated.items():
    demucs.api.save_audio(source, f"{out_dir}/{stem}.wav",
                          samplerate=separator.samplerate)
```

콜백으로 진행률 추적:
```python
def _on_progress(self, info):
    if info["audio_length"]:
        pct = int(info["segment_offset"] / info["audio_length"] * 100)
        self.progress.emit(pct, f"분리 중... {pct}%")
```

### GPU 메모리 옵션

| VRAM | 권장 |
|---|---|
| 3GB 미만 | `device="cpu"` |
| 3~6GB | `segment=7` |
| 7GB+ | 기본값 |

## StemPlayerWorker

`sounddevice` + `numpy` 실시간 믹싱.

```python
def callback(outdata, frames, time, status):
    mixed = np.zeros((frames, 2), dtype="float32")
    for name, data in self.stems.items():
        if self.muted[name]:
            continue
        segment = data[self._pos:self._pos + frames]
        mixed += segment * self.volumes[name]
    outdata[:] = np.clip(mixed, -1.0, 1.0)
    self._pos += frames
```

## PitchShiftWorker

```python
y, sr = librosa.load(path, sr=None, mono=False)
shifted = librosa.effects.pitch_shift(y, sr=sr, n_steps=self.semitones)
sf.write(out_path, shifted.T, sr)  # .T → samples, channels
```

처리 시간: 3분 곡 CPU 기준 약 5~15초.

## VersionCheckWorker

```python
class VersionCheckWorker(QThread):
    result = pyqtSignal(list)
    # [{"name", "installed", "latest_in_range", "latest_any", "constraint",
    #   "status": "ok"|"updatable"|"major_available"|"not_installed"}]
```

## Related Docs

- [SETTINGS.md](SETTINGS.md) — VersionCheckWorker 결과 표시
- [FILE_FORMATS.md](FILE_FORMATS.md) — VideoExtractWorker 입력 포맷
