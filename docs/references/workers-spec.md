# workers.py 상세 스펙

## SetupWorker(QThread)

```python
# 메이저 버전 고정 설치
PACKAGE_CONSTRAINTS = {
    "demucs":        "demucs>=4,<5",
    "pydub":         "pydub>=0.25,<1",
    "static-ffmpeg": "static-ffmpeg>=2,<3",
    "yt-dlp":        "yt-dlp",              # 항상 최신
    "sounddevice":   "sounddevice>=0.4,<1",
    "librosa":       "librosa>=0.10,<1",
}

def run(self):
    # 1. 미설치 패키지 확인 → 설치
    # 2. 설치된 패키지 버전 확인 → 범위 내 업데이트
    # 3. static_ffmpeg.add_paths()
    # 4. done.emit(True, "")
```

업데이트 확인 로직:

```python
def _needs_update(self, pkg, constraint):
    current = self._get_installed_version(pkg)
    latest_in_range = self._get_latest_in_range(pkg, constraint)
    return current != latest_in_range

def _get_latest_in_range(self, pkg, constraint):
    # pip index versions {pkg} 결과 파싱
    # packaging.version.Version 으로 constraint 필터링
    from packaging.version import Version
    from packaging.specifiers import SpecifierSet
    ...
```

---

## VideoExtractWorker(QThread)

영상 파일 → 오디오 WAV 추출.

```python
import subprocess

class VideoExtractWorker(QThread):
    progress = pyqtSignal(int, str)
    done = pyqtSignal(bool, str)  # success, wav_path

    def run(self):
        out_wav = Path(self.out_dir) / f"{Path(self.video_path).stem}_extracted.wav"
        cmd = [
            "ffmpeg", "-i", self.video_path,
            "-vn",                    # 영상 스트림 제거
            "-acodec", "pcm_s16le",   # WAV
            "-ar", "44100",
            "-ac", "2",
            "-y",                     # 덮어쓰기
            str(out_wav)
        ]
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT,
                                text=True, encoding="utf-8", errors="replace")
        # ffmpeg 진행률: "time=00:01:23" 파싱
        for line in proc.stdout:
            m = re.search(r"time=(\d+):(\d+):(\d+)", line)
            if m and self.duration:
                elapsed = int(m.group(1))*3600 + int(m.group(2))*60 + int(m.group(3))
                pct = min(int(elapsed / self.duration * 100), 99)
                self.progress.emit(pct, f"오디오 추출 중... {pct}%")
        proc.wait()
        if proc.returncode == 0:
            self.done.emit(True, str(out_wav))
        else:
            self.done.emit(False, "오디오 추출 실패")
```

영상 파일 처리 체이닝:

```
VideoExtractWorker → SeparationWorker → 완료
진행바: 0~20% 추출 / 20~100% 분리
```

---

## YtdlpWorker(QThread)

```python
import yt_dlp

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

진행률: `d["_percent_str"]` 에서 `(\d+(?:\.\d+)?)%` 파싱.
플레이리스트: 여러 파일 → 각각 개별 큐 아이템.

---

## SeparationWorker(QThread)

```python
cmd = [sys.executable, "-m", "demucs",
       "-n", "htdemucs_ft",   # 최고 품질 모델 명시
       "--out", str(tmp),
       self.file_path]
```

stdout char 단위 읽기 → tqdm `\r` 처리 → `(\d+)%` 파싱.
결과 경로: `tmp/*/stem_name/` 패턴 탐색 (모델명 하드코딩 금지).

완료 후 반환:

```python
{
  "vocals": "path/vocals.wav",
  "drums":  "path/drums.wav",
  "bass":   "path/bass.wav",
  "other":  "path/other.wav",
}
```

---

## StemPlayerWorker (오디오 엔진)

`sounddevice` + `numpy` 실시간 믹싱.

```python
import sounddevice as sd
import numpy as np
import soundfile as sf

# 콜백에서 4트랙 볼륨 적용 후 합산
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

---

## PitchShiftWorker(QThread)

```python
import librosa, soundfile as sf

# 스테레오 처리 (mono=False → shape: channels, samples)
y, sr = librosa.load(path, sr=None, mono=False)
shifted = librosa.effects.pitch_shift(y, sr=sr, n_steps=self.semitones)
sf.write(out_path, shifted.T, sr)  # .T → samples, channels
```

처리 시간: 3분 곡 CPU 기준 약 5~15초.
[적용] 버튼 → 처리 중 로딩 → 새 스템으로 리로드.

---

## VersionCheckWorker(QThread)

```python
class VersionCheckWorker(QThread):
    result = pyqtSignal(list)
    # result: [
    #   {
    #     "name":             "demucs",
    #     "installed":        "4.0.1",
    #     "latest_in_range":  "4.0.1",   # constraint 범위 내 최신
    #     "latest_any":       "5.0.0",   # 전체 최신 (메이저 포함)
    #     "constraint":       ">=4,<5",
    #     "status":  "ok" | "updatable" | "major_available" | "not_installed"
    #   },
    #   ...
    # ]
```

SettingsPage 열릴 때 자동 실행.
[새로고침] 클릭 시 재실행.
