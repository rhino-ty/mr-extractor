# Roadmap

## v1 — MVP (Must)

### 핵심 파이프라인
- [ ] SetupPage: Python/demucs/ffmpeg 환경 자동 감지 + 안내
- [ ] QueuePage: 유튜브 URL 입력 + 오디오/영상 파일 드래그&드롭
- [ ] ProcessPage: 다운로드 → 분리 진행 상태 (Channel 스트리밍)
- [ ] PlayerPage: 스템 믹서 재생 + 파형 시각화 (wavesurfer.js)

### Rust Commands
- [ ] setup: 환경 감지 (python, demucs, ffmpeg, yt-dlp)
- [ ] youtube: yt-dlp subprocess + 진행률 Channel
- [ ] video_extract: ffmpeg subprocess + 진행률 Channel
- [ ] separate: demucs subprocess + 진행률 Channel
- [ ] export: 현재 믹스 내보내기 (WAV/MP3)

### 필수 기능
- [ ] Web Audio API 4트랙 실시간 믹싱
- [ ] 키(피치) 조절: ±12 반음 (Tone.js 미리듣기 + ffmpeg rubberband 내보내기)
- [ ] 파형 시각화 (wavesurfer.js)
- [ ] 구간 반복 재생 A-B Loop (wavesurfer.js regions 플러그인)
- [ ] 처리 완료 시 시스템 알림
- [ ] 다크 테마 UI (Tailwind)
- [ ] 다중 선택 삭제
- [ ] 배치 내보내기

---

## v1.1 — 편의 기능

- [ ] 모델 선택 드롭다운 (htdemucs, htdemucs_ft, htdemucs_6s)
- [ ] 처리 히스토리 (Tauri Store + JSON)
- [ ] 중복 파일 감지 다이얼로그
- [ ] 앱 종료 안전 처리 (처리 중 확인)
- [ ] 전역 단축키 (global-shortcut 플러그인)
- [ ] BPM 감지

---

## v1.2 — 설정 & 관리

- [ ] SettingsPage: 패키지 버전 현황
- [ ] 저장 정책 옵션 (전체 저장 / 임시 / 반주만)
- [ ] 저장 공간 현황 + 고아 파일 정리
- [ ] 알림 설정 토글
- [ ] 내보내기 포맷 선택 (WAV/MP3/FLAC + 비트레이트)
- [ ] 앱 자동 업데이트 (updater 플러그인)

---

## v2+ — 백로그 (Later)

- [ ] GPU 가속 자동 감지 + 설정 UI (CUDA/MPS/CPU, segment 조절)
- [ ] 노래방 모드 (`--two-stems=vocals`)
- [ ] 이퀄라이저 (Web Audio BiquadFilterNode)
- [ ] 리버브/딜레이 효과 (Web Audio ConvolverNode)
- [ ] 가사 싱크 (.lrc)
- [ ] 프리셋 저장 (믹서 설정)
- [ ] 다국어 (영/한)
- [ ] 드래그 구간 선택 → 부분 내보내기
- [ ] MIDI 연동 (Web MIDI API)
- [ ] macOS / Linux 지원
