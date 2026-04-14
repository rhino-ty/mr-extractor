# MR Extractor

## Project Overview

MR Extractor: Demucs 기반 보컬 분리 및 반주(MR) 추출 데스크탑 앱 (Windows)
Stack: PyQt6, demucs, yt-dlp, sounddevice, numpy, librosa, pydub, static-ffmpeg, PyInstaller

킬러 피처:
- 유튜브 URL → MR 한 방에 (다운로드 + 분리 자동)
- 영상 파일(MP4/MKV 등) 드래그&드롭 → 오디오 자동 추출 후 분리
- 스템 믹서: 보컬/드럼/베이스/기타 볼륨 실시간 조절하며 재생
- 키 조절: 반음 단위 ±12 피치 시프트

**핵심 원칙: 앱만 실행하면 됨. 별도 설치 없음.**

## Commands

run: python main.py
build: build.bat # PyInstaller onefile windowed
install: pip install -r requirements.txt

## Project Structure

app/
styles.py # 다크 테마 색상 + QSS
workers.py # QThread 워커 (Setup, Separation, Ytdlp, VideoExtract, StemPlayer, PitchShift, VersionCheck)
history.py # 처리 히스토리 JSON 관리
pages/
setup_page.py # 자동 패키지 설치/업데이트
queue_page.py # URL 입력 + 파일 드롭 + 큐
process_page.py # 분리 진행 상태
player_page.py # 스템 믹서 + 키 조절
history_page.py # 처리 이력
settings_page.py # 버전 현황 + 저장 정책
widgets/
drop_zone.py # 드래그&드롭
url_input.py # 유튜브 URL 입력
file_card.py # 큐/처리 항목 카드
stem_mixer.py # 4트랙 볼륨 + 뮤트
model_selector.py # demucs 모델 드롭다운
history_card.py # 히스토리 항목
docs/
references/ # 상세 스펙 문서
01-plan/ ~ 04-report/ # PDCA

## Key Conventions

- PyQt6 enum full path: `Qt.AlignmentFlag.AlignCenter`, `QAbstractItemView.DragDropMode.DropOnly`, `Qt.ItemDataRole.UserRole`, `QSystemTrayIcon.MessageIcon.Information`
- QThread 워커: signal/slot 통신, UI 스레드에서 직접 처리 금지
- 다크 테마 색상: BG=#0d0d1a, SURFACE=#16162e, ACCENT=#7c5cfc
- 파일 타입 아이콘: 🔗 URL / 🎵 오디오 / 🎬 영상
- 출력 파일명: `{stem}_instrumental.wav`, `{stem}_instrumental_320k.mp3`
- 기본 출력 폴더: `~/Desktop/MR Extractor/`

## Version Policy

메이저 버전 고정. 마이너/패치만 자동 허용.

```
demucs>=4,<5 | pydub>=0.25,<1 | static-ffmpeg>=2,<3
sounddevice>=0.4,<1 | librosa>=0.10,<1 | yt-dlp (항상 최신)
```

yt-dlp만 예외: 유튜브 정책 변경 대응을 위해 버전 제한 없음.

## demucs

- `pip install demucs` 그대로 사용 (아카이브된 포크 설치 불필요)
- 기본 모델: `htdemucs_ft` (SDR 9.20dB, fine-tuned)
- 결과 경로: `tmp/*/stem/` 패턴 탐색 (모델명 하드코딩 금지)

## Do Not

- `pip install --upgrade demucs` 금지 → 반드시 constraint 지정
- demucs 결과 경로에 모델명 하드코딩 금지
- tqdm 진행률 `readline()` 금지 → char 단위 읽기
- librosa pitch_shift `mono=True` 금지 → `mono=False` 스테레오 유지
- sounddevice 콜백에서 UI 직접 업데이트 금지 → 50ms QTimer 분리. 재생 종료는 `CallbackStop`
- ffmpeg 진행률 파싱 시 총 재생시간은 ffprobe로 사전 확인 필수
- librosa sf.write 시 `.T` (transpose) 필수 — `sf.write(out_path, shifted.T, sr)`
- 영상 파일 처리 후 tmp WAV 삭제 안 하기 금지

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
