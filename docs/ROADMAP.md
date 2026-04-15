# Roadmap

## v1 — MVP (Must)

### 핵심 파이프라인

- [ ] SetupPage: 패키지 자동 설치/업데이트 + ffmpeg 경로 설정
- [ ] QueuePage: 유튜브 URL 입력 + 오디오/영상 파일 드래그&드롭
- [ ] ProcessPage: 다운로드 → 분리 진행 상태 표시
- [ ] PlayerPage: 스템 4트랙 믹서 재생

### Workers

- [ ] SetupWorker: 버전 범위 내 자동 설치/업데이트
- [ ] YtdlpWorker: 유튜브 다운로드 (플레이리스트 지원)
- [ ] VideoExtractWorker: 영상 → WAV 추출
- [ ] SeparationWorker: demucs 분리 (htdemucs_ft)
- [ ] StemPlayerWorker: sounddevice 실시간 믹싱

### 필수 기능

- [ ] 키(피치) 조절: ±12 반음 (librosa)
- [ ] 현재 믹스 내보내기 (WAV + MP3 320k)
- [ ] 처리 완료 시 트레이 알림
- [ ] 다크 테마 UI

---

## v1.1 — 편의 기능

- [ ] 모델 선택 드롭다운 (htdemucs, htdemucs_ft, htdemucs_6s)
- [ ] 처리 히스토리 (JSON 저장 + 재생/재처리)
- [ ] 중복 파일 감지 다이얼로그
- [ ] 앱 종료 안전 처리 (처리 중 확인)
- [ ] 단축키 전체 구현

---

## v1.2 — 설정 & 관리

- [ ] SettingsPage: 패키지 버전 현황 패널
- [ ] 저장 정책 옵션 (전체 저장 / 임시 / 반주만)
- [ ] 저장 공간 현황 + 고아 파일 정리
- [ ] 알림 설정 토글

---

## v2+ — 백로그 (Later)

- [ ] GPU 가속 자동 감지 + 설정 UI (CUDA/MPS/CPU 선택, segment 조절)
- [ ] 배치 처리 최적화 (동시 처리 수 조절)
- [ ] Python API 방식 전환 (CLI 서브프로세스 → `demucs.api.Separator`)
- [ ] 노래방 모드 (`--two-stems=vocals`)
- [ ] 커스텀 모델 지원
- [ ] 자동 업데이트 (앱 자체)
- [ ] macOS 지원
