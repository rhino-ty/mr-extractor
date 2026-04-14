# MR Extractor

유튜브 URL, 오디오 파일, 영상 파일에서 보컬을 분리하고 반주(MR)를 추출하는 Windows 데스크탑 앱.

분리된 스템(보컬/드럼/베이스/기타)을 앱 내에서 직접 재생하며 파트별 볼륨과 키를 조절할 수 있습니다.

## 주요 기능

- **유튜브 URL → MR 한 방에** — URL 붙여넣기만 하면 다운로드 + 분리 자동
- **영상 파일 지원** — MP4/MKV 등 드래그&드롭 → 오디오 자동 추출 후 분리
- **스템 믹서** — 보컬/드럼/베이스/기타 볼륨 실시간 조절하며 재생
- **키 조절** — 반음 단위 ±12 피치 시프트
- **처리 히스토리** — 이전 작업을 언제든 다시 재생/재처리
- **모델 선택** — HTDemucs / HTDemucs FT / HTDemucs 6스템

## 스크린샷

> (추후 추가)

## 실행

```bash
# 의존성 설치
pip install -r requirements.txt

# 앱 실행
python main.py
```

앱 실행 시 필요한 패키지(demucs, yt-dlp, ffmpeg 등)를 자동으로 설치합니다.
별도 설치가 필요 없습니다.

## 빌드 (Windows exe)

```bash
build.bat
# → dist/MR Extractor.exe
```

## 기술 스택

| 항목            | 선택                          |
| --------------- | ----------------------------- |
| UI              | PyQt6                         |
| 유튜브 다운로드 | yt-dlp                        |
| 음원 분리       | demucs (HTDemucs FT 모델)     |
| 오디오 처리     | pydub                         |
| 스템 재생/믹싱  | sounddevice + numpy           |
| 키(피치) 조절   | librosa                       |
| ffmpeg          | static-ffmpeg (자동 다운로드) |
| 패키징          | PyInstaller                   |

## 지원 포맷

**오디오**: mp3, wav, flac, ogg, m4a, aac, opus, aiff, wma, ape

**영상**: mp4, mkv, mov, avi, webm, wmv, flv, ts, m2ts

## 앱 흐름

```
앱 실행 → 자동 설치/업데이트 → QueuePage
  ├─ URL 입력 or 파일 드래그&드롭
  ├─ "분리 시작" → 진행 상태 표시
  └─ 완료 → 스템 믹서로 재생 (볼륨 + 키 조절)
```

## 프로젝트 구조

```
mr_extractor/
├── main.py                  # 엔트리포인트
├── requirements.txt
├── build.bat                # PyInstaller 빌드
├── app/
│   ├── styles.py            # 다크 테마 QSS
│   ├── workers.py           # QThread 워커
│   ├── history.py           # 히스토리 JSON 관리
│   ├── pages/               # SetupPage, QueuePage, ProcessPage, PlayerPage, ...
│   └── widgets/             # DropZone, StemMixer, ModelSelector, ...
└── docs/
    ├── INDEX.md             # 문서 지도
    ├── QUICK_REF.md         # 치트시트
    ├── ROADMAP.md           # 로드맵
    └── references/          # 상세 기술 스펙
```

## 라이선스

> (추후 추가)
