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
- **결론: `pip install demucs` 그대로 사용. 별도 포크 설치 불필요.**
- 현재 최고 모델: `htdemucs_ft` (SDR 9.20dB, fine-tuned 버전)

---

## 버전 관리 정책 (중요)

### 메이저 버전 고정

demucs v5 등 메이저 업데이트는 breaking change 가능성이 있음.
**마이너/패치 업데이트만 자동으로 허용.**

```python
PACKAGE_CONSTRAINTS = {
    "demucs":        "demucs>=4,<5",
    "pydub":         "pydub>=0.25,<1",
    "static-ffmpeg": "static-ffmpeg>=2,<3",
    "yt-dlp":        "yt-dlp>=2024,<2027",   # yt-dlp는 날짜 기반 버전
    "sounddevice":   "sounddevice>=0.4,<1",
    "librosa":       "librosa>=0.10,<1",
}
```

### yt-dlp는 예외적으로 자주 업데이트

유튜브 정책 변경에 대응하기 위해 yt-dlp는 항상 최신으로:

```python
"yt-dlp": "yt-dlp"  # 버전 제한 없이 항상 최신
```

---

## 프로젝트 구조

```
mr_extractor/
├── CLAUDE.md
├── main.py
├── requirements.txt
├── docs/references/          # 상세 스펙 문서
├── app/
│   ├── __init__.py
│   ├── styles.py
│   ├── workers.py
│   ├── history.py
│   ├── pages/
│   │   ├── __init__.py
│   │   ├── setup_page.py
│   │   ├── queue_page.py
│   │   ├── process_page.py
│   │   ├── player_page.py
│   │   ├── history_page.py
│   │   └── settings_page.py
│   └── widgets/
│       ├── __init__.py
│       ├── drop_zone.py
│       ├── url_input.py
│       ├── file_card.py
│       ├── stem_mixer.py
│       ├── model_selector.py
│       └── history_card.py
└── build.bat
```

---

## 페이지 흐름

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

## 상세 스펙 문서 (docs/references/)

각 기능의 상세 구현 스펙은 아래 문서 참조:

| 문서 | 내용 |
| --- | --- |
| [workers-spec.md](docs/references/workers-spec.md) | Worker 클래스 상세 (Setup, VideoExtract, Ytdlp, Separation, StemPlayer, PitchShift, VersionCheck) |
| [file-formats.md](docs/references/file-formats.md) | 지원 파일 포맷, DropZone 확장자 분류 |
| [player-page-spec.md](docs/references/player-page-spec.md) | PlayerPage UI 레이아웃, UX 퀄리티 가이드 |
| [model-selector-spec.md](docs/references/model-selector-spec.md) | 모델 선택 드롭다운, 툴팁, [?] 도움말 패널 |
| [history-spec.md](docs/references/history-spec.md) | 처리 히스토리 JSON 구조, UI, 뱃지, 삭제 |
| [settings-spec.md](docs/references/settings-spec.md) | 설정 페이지 (버전 현황, 저장 정책, 저장 공간, 알림) |
| [shortcuts-spec.md](docs/references/shortcuts-spec.md) | 앱 전역 단축키 |
| [ux-behaviors.md](docs/references/ux-behaviors.md) | 앱 종료 안전 처리, 중복 파일 처리, 첫 실행 UX |
