# MR Extractor — CLAUDE.md

## 프로젝트 개요

Demucs 기반 보컬 분리 및 반주(MR) 추출 데스크탑 앱 (Windows).
유튜브 URL, 오디오 파일, 영상 파일 → 보컬 제거 → 반주 추출.
분리된 스템을 앱 내에서 직접 재생하며 파트별 볼륨 + 키 조절 가능.

**킬러 피처:**

- 유튜브 URL → MR 한 방에 (다운로드 + 분리 자동)
- 영상 파일(MP4/MKV 등) 드래그&드롭 → 오디오 자동 추출 후 분리
- 스템 믹서: 보컬/드럼/베이스/기타 볼륨 실시간 조절하며 재생
- 키 조절: 반음 단위 ±12 피치 시프트

---

## 기술 스택

| 항목            | 선택                            |
| --------------- | ------------------------------- |
| UI              | PyQt6                           |
| 유튜브 다운로드 | yt-dlp                          |
| 음원 분리       | demucs (HTDemucs ft 모델)       |
| 오디오 처리     | pydub                           |
| 스템 재생/믹싱  | sounddevice + numpy             |
| 키(피치) 조절   | librosa                         |
| ffmpeg          | static-ffmpeg (자동 다운로드)   |
| 패키징          | PyInstaller (onefile, windowed) |

**핵심 원칙: 앱만 실행하면 됨. 별도 설치 없음.**

---

## demucs 패키지 관련 중요 사항

- `facebookresearch/demucs` 레포는 2025년 1월 1일부로 아카이브 (read-only)
- 원저자 개인 포크 `adefossez/demucs` 로 이전됐으나 버그픽스만 처리
- `demucs-infer` 라는 PyTorch 2.x 호환 유지보수 포크도 존재 (pip 패키지)
- **결론: `pip install demucs` 그대로 사용. 별도 포크 설치 불필요.**
- 현재 최고 모델: `htdemucs_ft` (SDR 9.20dB, fine-tuned 버전)

---

## 버전 관리 정책 (중요)

### 메이저 버전 고정

demucs v5 등 메이저 업데이트는 breaking change 가능성이 있음.
**마이너/패치 업데이트만 자동으로 허용.**

```python
# SetupWorker에서 패키지 설치 시 메이저 버전 고정
PACKAGE_CONSTRAINTS = {
    "demucs":        "demucs>=4,<5",
    "pydub":         "pydub>=0.25,<1",
    "static-ffmpeg": "static-ffmpeg>=2,<3",
    "yt-dlp":        "yt-dlp>=2024,<2027",   # yt-dlp는 날짜 기반 버전
    "sounddevice":   "sounddevice>=0.4,<1",
    "librosa":       "librosa>=0.10,<1",
}
```

업데이트 확인 시에도 메이저 버전 범위 내에서만:

```python
def _get_latest_in_range(self, pkg, constraint):
    # pip index versions 결과에서 constraint 범위 내 최신 버전만 반환
    ...
```

### yt-dlp는 예외적으로 자주 업데이트

유튜브 정책 변경에 대응하기 위해 yt-dlp는 항상 최신으로:

```python
"yt-dlp": "yt-dlp"  # 버전 제한 없이 항상 최신
```

---

## 지원 파일 포맷

### 오디오 (직접 분리)

```
.mp3  .wav  .flac  .ogg  .m4a  .aac  .opus  .aiff  .wma  .ape
```

### 영상 (오디오 자동 추출 후 분리)

```
.mp4  .mkv  .mov  .avi  .webm  .wmv  .flv  .ts  .m2ts
```

ffmpeg가 있으므로 사실상 ffmpeg가 지원하는 모든 포맷 가능.

### 처리 흐름

```
오디오 파일 → demucs 직접 투입
영상 파일   → ffmpeg로 오디오 추출(WAV) → demucs 투입 → tmp WAV 삭제
```

영상 파일은 FileCard에서 🎬 아이콘으로 구분.
큐에 추가 시 "(영상에서 추출)" 표시.

### DropZone 확장자 분류

```python
AUDIO_EXTS = {".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac",
              ".opus", ".aiff", ".wma", ".ape"}
VIDEO_EXTS = {".mp4", ".mkv", ".mov", ".avi", ".webm",
              ".wmv", ".flv", ".ts", ".m2ts"}
ALL_EXTS   = AUDIO_EXTS | VIDEO_EXTS
```

---

## 프로젝트 구조

```
mr_extractor/
├── CLAUDE.md
├── main.py
├── requirements.txt
├── app/
│   ├── __init__.py
│   ├── styles.py
│   ├── workers.py
│   ├── pages/
│   │   ├── __init__.py
│   │   ├── setup_page.py
│   │   ├── queue_page.py
│   │   ├── process_page.py
│   │   └── player_page.py
│   └── widgets/
│       ├── __init__.py
│       ├── drop_zone.py
│       ├── url_input.py
│       ├── file_card.py
│       └── stem_mixer.py
└── build.bat
```

---

## 페이지 흐름

```
앱 시작
  └─▶ SetupPage (자동, 클릭 불필요)
        ├─ 패키지 버전 범위 내 업데이트 확인 및 자동 설치
        ├─ yt-dlp는 항상 최신으로
        ├─ static_ffmpeg.add_paths()
        └─ 완료 → 1초 후 QueuePage

  └─▶ QueuePage
        ├─ [URL 섹션] 유튜브 URL (플레이리스트 지원)
        ├─ [파일 섹션] 오디오/영상 드래그&드롭 혼합 가능
        ├─ 큐 목록 (🔗 URL / 🎵 오디오 / 🎬 영상 아이콘 구분)
        ├─ 출력 폴더 / 동시 처리 수
        └─ "분리 시작" → ProcessPage

  └─▶ ProcessPage
        ├─ 파일별 FileCard
        │     URL:  다운로드 → 분리
        │     오디오: 분리
        │     영상:  오디오 추출 → 분리
        ├─ 전체 진행률
        ├─ 완료 시 트레이 알림
        └─ 완료 항목 클릭 → PlayerPage

  └─▶ PlayerPage
        ├─ 재생/탐색 컨트롤
        ├─ StemMixer: 4트랙 볼륨 + 뮤트
        ├─ 키 조절 (-12 ~ +12 반음)
        ├─ 현재 믹스 내보내기
        └─ "목록으로" → QueuePage
```

---

## workers.py 상세 스펙

### SetupWorker(QThread)

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

### VideoExtractWorker(QThread)

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

### YtdlpWorker(QThread)

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

### SeparationWorker(QThread)

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

### StemPlayerWorker (오디오 엔진)

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

### PitchShiftWorker(QThread)

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

## PlayerPage UI

```
┌─────────────────────────────────────────────────────┐
│  ← 목록                    소란 - 사랑한 마음엔...    │
├─────────────────────────────────────────────────────┤
│  ████████████████░░░░░░░░░░░  2:14 / 3:42          │
│         [◀◀]  [▶ 재생]  [■ 정지]                    │
├─────────────────────────────────────────────────────┤
│  🎤 Vocals  [M]  ━━━━━━━━━●━━━  85                 │
│  🥁 Drums   [M]  ━━━━━━━━━━━━━●  100               │
│  🎸 Bass    [M]  ━━━━━━━●━━━━━  70                 │
│  🎹 Other   [M]  ━━━━━━━━━━━━━●  100               │
├─────────────────────────────────────────────────────┤
│  키 조절   [-12 ━━━━━━●━━━━━━ +12]  +2반음          │
│                           [적용]  [원래대로]         │
├─────────────────────────────────────────────────────┤
│              [💾 현재 믹스 내보내기]                  │
└─────────────────────────────────────────────────────┘
```

---

## UI 스타일

다크 테마 색상:

```python
BG      = "#0d0d1a"
SURFACE = "#16162e"
BORDER  = "#2a2a4e"
ACCENT  = "#7c5cfc"
SUCCESS = "#3dd68c"
WARN    = "#f0a500"
DANGER  = "#e05c5c"
MUTED   = "#8888aa"
```

파일 타입 아이콘: 🔗 URL / 🎵 오디오 / 🎬 영상

---

## 출력 파일명 규칙

```
{stem}_instrumental.wav
{stem}_instrumental_320k.mp3
기본 출력: ~/Desktop/MR Extractor/
```

---

## 빌드

```bat
@echo off
pip install pyinstaller
pyinstaller --onefile --windowed --name "MR Extractor" main.py
echo 완료: dist\MR Extractor.exe
pause
```

---

## 구현 주의사항

1. **버전 고정**: `pip install --upgrade demucs` 금지. 반드시 `"demucs>=4,<5"` 형태로 constraint 지정.

2. **yt-dlp 예외**: 유튜브 정책 변경 대응을 위해 버전 제한 없이 항상 최신.

3. **영상 파일**: ffmpeg로 오디오 추출 후 tmp WAV 생성 → 분리 완료 후 tmp WAV 삭제.

4. **ffmpeg 진행률**: `time=HH:MM:SS` 패턴 파싱. 총 재생시간은 ffprobe로 사전 확인.

5. **tqdm 진행률**: char 단위 읽기. `readline()` 금지.

6. **demucs 결과 경로**: `tmp/*/stem/` 패턴 탐색. 모델명 하드코딩 금지.

7. **librosa pitch_shift**: `mono=False` 스테레오 유지. sf.write 시 `.T` 필요.

8. **sounddevice 콜백**: `CallbackStop`으로 재생 종료. UI 업데이트 50ms 타이머 분리.

9. **PyQt6 API**:
   - `Qt.AlignmentFlag.AlignCenter`
   - `QAbstractItemView.DragDropMode.DropOnly`
   - `Qt.ItemDataRole.UserRole`
   - `QSystemTrayIcon.MessageIcon.Information`

---

## 첫 실행 UX

```
앱 실행 → 자동 설치/업데이트 → 자동 진입 (클릭 0회)
URL 붙여넣기 → 추가 → 분리 시작 → 스템 믹서로 재생
```

---

## SettingsPage 스펙

메인 화면 우상단 ⚙ 버튼으로 진입. 별도 페이지 또는 모달.

### 패키지 버전 현황 패널

앱 시작 시 백그라운드에서 버전 정보 수집 → 설정 페이지 열면 표시.

```
┌─────────────────────────────────────────────────────────┐
│  패키지 버전                              [🔄 새로고침]  │
├──────────────┬──────────┬──────────┬────────────────────┤
│  패키지       │ 설치됨   │ 최신     │ 상태               │
├──────────────┼──────────┼──────────┼────────────────────┤
│  demucs      │ 4.0.1    │ 4.0.1    │ ✅ 최신            │
│  yt-dlp      │ 2024.11  │ 2025.03  │ 🔄 업데이트 가능   │
│  pydub       │ 0.25.1   │ 0.25.1   │ ✅ 최신            │
│  static-ffmpeg│ 2.1.0   │ 2.2.0    │ 🔄 업데이트 가능   │
│  sounddevice │ 0.4.3    │ 0.4.3    │ ✅ 최신            │
│  librosa     │ 0.10.2   │ 0.10.2   │ ✅ 최신            │
├──────────────┴──────────┴──────────┴────────────────────┤
│  ⚠ 메이저 업데이트는 자동 적용되지 않습니다              │
│    (demucs v5 등 → 개발자 검토 후 앱 업데이트 배포)      │
│                                    [선택 업데이트]       │
└─────────────────────────────────────────────────────────┘
```

**상태 표시 규칙:**

- `✅ 최신` — 설치 버전 = 허용 범위 내 최신
- `🔄 업데이트 가능` — 범위 내 더 높은 버전 있음 → 업데이트 버튼 활성
- `🔒 메이저 업데이트 있음` — constraint 밖 버전 존재 (자동 업데이트 안 함, 표시만)
- `❌ 미설치` — 설치 안 된 경우

**[선택 업데이트] 버튼:**

- `🔄` 상태인 패키지만 체크박스 선택 가능
- `🔒` 패키지는 체크박스 비활성화 (클릭 안 됨)
- 업데이트 진행 시 pip 로그 인라인 표시

**VersionCheckWorker(QThread):**

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

---

## UX 퀄리티 가이드 (판매용 수준)

이 앱은 모바일/웹 셀링용 레퍼런스 MVP임. 완성도가 비즈니스 신뢰도와 직결됨.

### 필수 퀄리티 기준

- 모든 버튼/슬라이더에 hover 상태 애니메이션
- 로딩 중 스피너 또는 인디케이터 항상 표시 (빈 화면 절대 금지)
- 에러 발생 시 친절한 한국어 메시지 + 원인 힌트 제공
- 모든 액션에 즉각적인 시각 피드백 (클릭 → 반응 < 100ms)
- 툴팁은 hover 0.5초 후 나타나게 (너무 즉각적이면 방해됨)
- 빈 상태(파일 없음, 히스토리 없음)는 일러스트 + 안내 문구로 처리
- 전환 애니메이션: 페이지 전환 시 fade 또는 slide (200ms)

### 폰트/간격

- 계층 구조 명확히: 타이틀 20px / 섹션 13px bold / 본문 13px / 캡션 11px
- 충분한 여백: 카드 내부 padding 최소 16px
- 아이콘 + 텍스트 조합 시 간격 6~8px

---

## 모델 선택 스펙

### 드롭다운 위치

QueuePage 설정 섹션에 배치:

```
모델 선택   [htdemucs_ft  ▼]  [?]
```

### 모델 목록 및 툴팁

드롭다운 각 항목에 `setItemData(index, tooltip, Qt.ItemDataRole.ToolTipRole)` 로 툴팁 등록.
항목 hover 시 툴팁 자동 표시.

| 값            | 표시명              | 툴팁 내용                                                               |
| ------------- | ------------------- | ----------------------------------------------------------------------- |
| `htdemucs`    | HTDemucs (기본)     | 빠르고 안정적. 보컬/드럼/베이스/기타 4트랙 분리. 일반적인 용도에 적합.  |
| `htdemucs_ft` | HTDemucs FT ⭐ 권장 | Fine-tuned 버전. 현재 최고 품질 (SDR 9.20dB). 처리 시간이 조금 더 걸림. |
| `htdemucs_6s` | HTDemucs 6스템      | 기타·피아노 트랙 추가 분리 (총 6트랙). 피아노 품질은 아직 실험적.       |

### [?] 버튼 동작

드롭다운 옆 `?` 버튼 클릭 시 인라인 도움말 패널 토글:

```
┌──────────────────────────────────────────────────────┐
│  모델 선택 가이드                               [✕]   │
│                                                      │
│  🏆 HTDemucs FT (권장)                               │
│     현재 가장 높은 분리 품질. 대부분의 경우 이걸 쓰세요. │
│                                                      │
│  ⚡ HTDemucs (기본)                                   │
│     FT보다 약간 빠름. 빠른 미리듣기용으로 적합.        │
│                                                      │
│  🎸 HTDemucs 6스템                                   │
│     기타·피아노까지 따로 분리하고 싶을 때.              │
│     단, 피아노 분리 품질은 아직 완벽하지 않음.          │
└──────────────────────────────────────────────────────┘
```

구현: `QToolButton` + `QFrame` 토글 방식. 애니메이션으로 부드럽게 펼치기.

---

## 처리 히스토리 스펙

### 위치

메인 화면(QueuePage) 하단 탭 또는 사이드 패널. 또는 별도 HistoryPage.

### 저장 방식

```python
# history.json — 앱 데이터 폴더에 저장
# Windows: %APPDATA%\MR Extractor\history.json

{
  "entries": [
    {
      "id": "uuid",
      "date": "2026-04-12T19:30:00",
      "source_type": "youtube" | "file" | "video",
      "source":  "https://youtube.com/..." | "/path/to/file.mp3",
      "title":   "소란 - 사랑한 마음엔 죄가 없다",
      "model":   "htdemucs_ft",
      "out_dir": "/path/to/output/",
      "files": {
        "wav": "/path/..._instrumental.wav",
        "mp3": "/path/..._instrumental_320k.mp3",
        "stems": {
          "vocals": "/path/vocals.wav",
          "drums":  "/path/drums.wav",
          "bass":   "/path/bass.wav",
          "other":  "/path/other.wav"
        }
      },
      "status": "done" | "error",
      "error_msg": null
    }
  ]
}
```

### 히스토리 UI

스템 보유 여부를 뱃지로 표시해 믹서 가능 여부를 한눈에 파악 가능.

```
┌──────────────────────────────────────────────────────────┐
│  처리 히스토리                      [🗑 전체 삭제]  [✕]  │
├──────────────────────────────────────────────────────────┤
│  🎵  소란 - 사랑한 마음엔 죄가 없다            2026.04.12 │
│      htdemucs_ft  ·  파일                                │
│      [🎚 스템 있음]                                       │
│      [▶ 재생]  [📁 폴더 열기]  [🔄 재처리]  [🗑]         │
├──────────────────────────────────────────────────────────┤
│  🔗  Adele - Hello                             2026.04.11 │
│      htdemucs_ft  ·  YouTube                             │
│      [🎵 반주만]                                          │
│      [📥 반주 받기]  [🔄 재처리로 스템 생성]  [🗑]        │
├──────────────────────────────────────────────────────────┤
│  ❌  알 수 없는 아티스트.mp3                   2026.04.10 │
│      오류: demucs 실행 오류                               │
│      [🔄 재처리]  [🗑]                                    │
└──────────────────────────────────────────────────────────┘
```

**뱃지 표시 규칙:**

| 뱃지           | 조건                           | 색상          |
| -------------- | ------------------------------ | ------------- |
| `🎚 스템 있음` | 스템 wav 4개 모두 존재         | 보라 (ACCENT) |
| `🎵 반주만`    | instrumental만 존재, 스템 없음 | 회색 (MUTED)  |
| `⚠ 파일 없음`  | 출력 파일이 모두 삭제된 경우   | 노랑 (WARN)   |

**뱃지 클릭 동작:**

- `🎚 스템 있음` 클릭 → PlayerPage 바로 오픈 (믹서 풀 사용)
- `🎵 반주만` 클릭 → "재처리하면 스템을 생성할 수 있습니다" 툴팁

**각 항목 액션:**

- `[▶ 재생]` → 스템 있으면 믹서 포함 PlayerPage, 반주만이면 심플 플레이어
- `[📥 반주 받기]` → instrumental 파일을 다른 폴더로 복사/저장
- `[📁 폴더 열기]` → `os.startfile(out_dir)` (Windows)
- `[🔄 재처리]` → 원본 파일/URL을 큐에 다시 추가 (스템 생성 포함)
- `[🗑]` → 삭제 옵션 다이얼로그 (기록만 / 기록+파일)

**파일 존재 여부 확인:**
히스토리 페이지 열릴 때 백그라운드로 파일 존재 확인.
없어진 파일은 자동으로 `⚠ 파일 없음` 뱃지로 전환.
`[▶ 재생]` / `[📁 폴더 열기]` 비활성화.

---

## 단축키 스펙

앱 전역 단축키 (`QShortcut` 로 MainWindow에 등록):

| 단축키                    | 동작                               |
| ------------------------- | ---------------------------------- |
| `Space`                   | PlayerPage: 재생/일시정지 토글     |
| `←` / `→`                 | PlayerPage: 5초 뒤로/앞으로        |
| `Shift + ←` / `Shift + →` | PlayerPage: 30초 이동              |
| `↑` / `↓`                 | PlayerPage: 마스터 볼륨 +5 / -5    |
| `M`                       | PlayerPage: 전체 뮤트 토글         |
| `Ctrl + O`                | 파일 열기 다이얼로그               |
| `Ctrl + H`                | 히스토리 패널 토글                 |
| `Ctrl + ,`                | 설정 페이지 열기                   |
| `Escape`                  | 모달/도움말 닫기, 현재 페이지 뒤로 |

**단축키 표시:**

- 버튼 툴팁에 단축키 병기: `"재생 (Space)"`
- 설정 페이지에 단축키 목록 표시 섹션 추가

---

## 페이지 흐름 (업데이트)

```
앱 시작
  └─▶ SetupPage (자동)
        └─ 완료 → QueuePage

  └─▶ QueuePage  ← 메인
        ├─ URL 입력 / 파일 드롭
        ├─ 큐 목록
        ├─ 모델 선택 드롭다운 + [?] 도움말
        ├─ ⚙ 설정  /  🕐 히스토리  (우상단 버튼)
        └─ "분리 시작" → ProcessPage

  └─▶ ProcessPage
        └─ 완료 항목 → PlayerPage

  └─▶ PlayerPage
        ├─ 스템 믹서 + 키 조절
        └─ "목록으로" → QueuePage

  └─▶ SettingsPage (⚙ 버튼)
        ├─ 패키지 버전 현황
        ├─ 단축키 목록
        └─ 출력 폴더 기본값 등

  └─▶ HistoryPage (🕐 버튼 or 패널)
        ├─ 처리 이력 목록
        └─ 재생 / 폴더 열기 / 재처리
```

---

## 프로젝트 구조 (업데이트)

```
mr_extractor/
├── CLAUDE.md
├── main.py
├── requirements.txt
├── app/
│   ├── __init__.py
│   ├── styles.py
│   ├── workers.py
│   ├── history.py               # 히스토리 저장/불러오기 (JSON)
│   ├── pages/
│   │   ├── __init__.py
│   │   ├── setup_page.py
│   │   ├── queue_page.py
│   │   ├── process_page.py
│   │   ├── player_page.py
│   │   ├── history_page.py      # 처리 히스토리
│   │   └── settings_page.py     # 버전 현황 + 단축키 목록
│   └── widgets/
│       ├── __init__.py
│       ├── drop_zone.py
│       ├── url_input.py
│       ├── file_card.py
│       ├── stem_mixer.py
│       ├── model_selector.py    # 모델 드롭다운 + 툴팁 + [?]
│       └── history_card.py      # 히스토리 항목 카드
└── build.bat
```

---

## 파일 정리 / 삭제 스펙

### 기본 방침

**기본값: 스템 전체 영구 보관.**
스템 파일이 있어야 믹서(볼륨 조절)를 언제든 다시 사용할 수 있음.
각 파일은 히스토리에 저장되고 언제든 불러와서 재생 가능.

---

### 저장 정책 옵션 (설정 페이지)

사용자가 용도에 맞게 선택:

```python
STORAGE_POLICY = {
    "keep_all":      "스템 전체 저장",        # 기본값 ← 믹서 항상 사용 가능
    "session_only":  "임시 저장",             # 재생 후 스템 자동 삭제
    "inst_only":     "반주 파일만 만들기",     # 스템 없음, 믹서 사용 불가
}
```

**설정 페이지 표시 (사용자 친화적 설명):**

```
┌──────────────────────────────────────────────────────────┐
│  파일 저장 방식                                           │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ● 스템 전체 저장  ← 기본값                               │
│    보컬/드럼/베이스/기타 파일을 모두 저장합니다.            │
│    언제든지 믹서로 다시 볼륨을 조절할 수 있습니다.          │
│    (곡 1개당 약 200~600 MB)                               │
│                                                          │
│  ○ 임시 저장                                              │
│    재생이 끝나면 스템 파일을 자동으로 삭제합니다.           │
│    반주(instrumental) 파일은 남습니다.                    │
│    ⚠ 나중에 믹서를 다시 쓰려면 재처리가 필요합니다.        │
│                                                          │
│  ○ 반주 파일만 만들기                                     │
│    보컬이 제거된 반주 MP3/WAV만 저장합니다.                │
│    가장 적은 용량. 믹서 사용 불가.                         │
│    (곡 1개당 약 10~30 MB)                                  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

각 옵션에 `setToolTip()` 으로 상세 설명 추가.

---

### 히스토리 항목 삭제 UI

항목 우클릭 or [🗑] 버튼 → 삭제 옵션 선택:

```
┌──────────────────────────────────────────────────┐
│  삭제                                             │
│                                                  │
│  소란 - 사랑한 마음엔 죄가 없다                   │
│                                                  │
│  ○ 기록만 삭제                                    │
│    파일은 그대로 남습니다.                         │
│                                                  │
│  ● 기록 + 파일 모두 삭제                           │
│    삭제될 파일 (총 234 MB):                        │
│      · 소란_instrumental.wav       180 MB         │
│      · 소란_instrumental_320k.mp3   12 MB         │
│      · 소란_vocals.wav              21 MB         │
│      · 소란_drums.wav               11 MB         │
│      · 소란_bass.wav                 5 MB         │
│      · 소란_other.wav                5 MB         │
│                                                  │
│              [취소]      [🗑 삭제]                │
└──────────────────────────────────────────────────┘
```

파일 용량은 미리 계산해서 표시. 없는 파일은 목록에서 제외.

---

### 설정 페이지 저장 공간 섹션

```
┌──────────────────────────────────────────────────────────┐
│  저장 공간                                                │
├──────────────────────────────────────────────────────────┤
│  출력 폴더     ~/Desktop/MR Extractor/   [📁 폴더 열기]  │
│  전체 사용량   2.3 GB  (파일 14개)                        │
│                                                          │
│    스템 파일   1.8 GB  ···············█████████  78%     │
│    반주 파일   0.4 GB  ···············██  17%             │
│    기타        0.1 GB  ···············  5%                │
│                                                          │
│  [🗑 전체 삭제]      [🧹 고아 파일 정리]                  │
└──────────────────────────────────────────────────────────┘
```

**[전체 삭제]:** 확인 다이얼로그 필수. "히스토리 기록도 함께 삭제" 체크박스 포함.

**[고아 파일 정리]:** 히스토리에 없는 파일 탐지 → 목록 보여주고 선택 삭제.

```python
# StorageScanWorker (백그라운드 비동기)
all_files  = set(Path(out_dir).rglob("*.*"))
tracked    = set(히스토리 전체 파일 경로)
orphans    = all_files - tracked
total_size = sum(f.stat().st_size for f in all_files)
```

용량 계산은 `StorageScanWorker(QThread)` 로 백그라운드 처리.
설정 페이지 열릴 때 자동 실행, [🔄] 버튼으로 수동 갱신.

---

## 앱 종료 안전 처리

처리 중 창 닫기(X 버튼) 또는 Alt+F4 시:

```
┌──────────────────────────────────────┐
│  ⚠ 처리 중입니다                      │
│                                      │
│  2개 파일을 처리 중입니다.              │
│  지금 종료하면 작업이 취소됩니다.        │
│                                      │
│  [계속 처리]   [취소 후 종료]           │
└──────────────────────────────────────┘
```

구현:

```python
def closeEvent(self, event):
    if self._is_processing():
        reply = QMessageBox.question(...)
        if reply == QMessageBox.StandardButton.Yes:
            self._cancel_all_workers()
            event.accept()
        else:
            event.ignore()
    else:
        event.accept()
```

처리 중이 아니면 확인 없이 즉시 종료.

---

## 중복 파일 처리

같은 파일/URL을 큐에 다시 추가하려 할 때:

```
┌──────────────────────────────────────────┐
│  이미 추가된 파일입니다                    │
│                                          │
│  소란 - 사랑한 마음엔 죄가 없다.mp3        │
│                                          │
│  ○ 건너뜀 (추가 안 함)     ← 기본          │
│  ○ 중복 추가 (같은 파일 두 번 처리)        │
│                                          │
│  □ 앞으로 이 선택 기억하기                 │
│                                          │
│               [확인]                      │
└──────────────────────────────────────────┘
```

URL은 정규화 후 비교 (`?si=...` 같은 트래킹 파라미터 제거):

```python
def normalize_url(url):
    from urllib.parse import urlparse, urlencode, parse_qs
    parsed = urlparse(url)
    # YouTube: v= 파라미터만 남김
    if "youtube" in parsed.netloc or "youtu.be" in parsed.netloc:
        v = parse_qs(parsed.query).get("v", [""])[0]
        return f"https://www.youtube.com/watch?v={v}"
    return url
```

---

## 알림 설정

설정 페이지에서 토글:

| 설정                     | 기본값 |
| ------------------------ | ------ |
| 처리 완료 시 트레이 알림 | ON     |
| 오류 발생 시 알림        | ON     |
| 업데이트 완료 알림       | OFF    |

구현: `QSystemTrayIcon.showMessage()`. 알림 클릭 시 앱 포커스 + 해당 항목으로 스크롤.
