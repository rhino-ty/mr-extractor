# setup-page Planning Document

> **Summary**: 앱 최초 실행 시 Python + demucs + sidecar 바이너리 환경을 자동으로 감지/설치하고, 사용자는 클릭 0회로 다음 단계로 진입하게 하는 첫 실행 경험.
>
> **Project**: MR Extractor
> **Version**: 0.1.0
> **Author**: rhino-ty
> **Date**: 2026-04-24
> **Status**: Confirmed v0.6 — 6건 결정 + 4차 gap fix (fresh-read High 8건 교차 정리) (§10.7, Version History 참조)

---

## Executive Summary

| Perspective            | Content                                                                                                                                  |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **Problem**            | 앱 실행 후 "뭘 깔아야 하는지" 몰라 이탈. Python/demucs는 웹앱과 달리 사용자 머신에 무거운 런타임 설치가 필요한데 기술 용어 노출 시 실패. |
| **Solution**           | Embedded Python + sidecar 바이너리(ffmpeg/yt-dlp) 번들 → 첫 실행 시 demucs만 자동 설치 → 진행률 UI로 대기 경험 설계                      |
| **Function/UX Effect** | 앱 실행 → `SetupPage` 자동 → (첫 실행만 3~5분 대기, 100Mbps 기준) → `QueuePage` 자동 진입. 클릭 0회. 기술 용어 0개.                      |
| **Core Value**         | "웹앱처럼 설치 없이 바로 쓴다" — 데스크톱 앱의 설치 허들 제거. 이후 모든 피처의 실행 기반.                                               |

---

## Context Anchor

| Key         | Value                                                                                                                                    |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **WHY**     | Python/demucs 미설치는 앱을 못 쓰게 만들고, 설치 가이드 문서화는 사용자 이탈을 만든다. 번들 + 자동화로 "클릭 0회" 달성 필요.             |
| **WHO**     | 비개발자 포함 일반 사용자 (유튜브 URL 붙여넣고 MR 뽑고 싶은 사람). 개발자도 동일 경로 사용.                                              |
| **RISK**    | ① 설치 파일 비대 (~250MB, sidecar 4종 + Python). ② demucs 첫 설치 실패 (네트워크/PyTorch wheel 오류). ③ Windows SmartScreen/백신 오탐. ④ probing 실패 시 부정확한 크기 표시. |
| **SUCCESS** | 인터넷 연결 + 정상 환경 + 100Mbps에서 앱 첫 실행 후 **5분 이내** QueuePage 진입. 2회차 이후 **2초 이내** 진입.                            |
| **SCOPE**   | Phase 1: 환경 감지 + Embedded Python 번들, Phase 2: demucs 자동 설치 + 진행률 UI, Phase 3: 오류/오프라인 처리.                           |

---

## 1. Overview

### 1.1 Purpose

데스크톱 앱은 웹앱과 달리 **런타임이 사용자 머신에 물리적으로 존재**해야 한다. MR Extractor는 Python + demucs(PyTorch 포함, ~2GB) + ffmpeg + yt-dlp를 필요로 하는데, 일반 사용자에게 "Python 설치하세요, pip install demucs 하세요"를 요구하면 99%가 이탈한다.

이 피처는 그 허들을 **앱 번들 + 자동 설치**로 전환한다:

- ffmpeg/yt-dlp: 앱에 sidecar로 함께 배포 (사용자 설치 불필요)
- Python: Embedded Python을 앱에 함께 배포
- demucs(+PyTorch): 첫 실행 시 1회 자동 `pip install` (용량 크기 때문에 번들 대신 온디맨드 설치)

### 1.2 Background (웹 개발자 관점 번역)

| 웹앱 개념                               | 데스크톱 앱 대응                                        |
| --------------------------------------- | ------------------------------------------------------- |
| `npm install` (배포 전에 번들링 완료)   | sidecar 바이너리 (ffmpeg, yt-dlp) — 빌드 시 번들        |
| CDN에서 런타임 다운로드                 | Embedded Python — 앱에 함께 포함                        |
| `npm run postinstall` (사용자 머신에서) | 첫 실행 `pip install demucs` — SetupPage에서 자동 실행  |
| 서버에서 Node 프로세스 실행             | Rust가 subprocess로 Python을 실행 (같은 개념, 로컬에서) |

핵심 인사이트: **ffmpeg/yt-dlp는 정적 바이너리라 번들 쉬움, demucs(PyTorch)는 ~2GB라 번들하면 설치 파일이 2.5GB가 돼서 배포 비현실적** → 하이브리드 전략(static 바이너리는 번들, 무거운 Python 패키지는 첫 실행 시 설치)이 현실적 해법.

### 1.3 Related Documents

- **Plan 부모**: [project-setup.report.md](../../04-report/project-setup.report.md) — 스캐폴딩 완료, sidecar/Python 경로는 이 피처로 이관됨
- **참조 스펙**:
  - [docs/references/COMMANDS.md §setup.rs](../../references/COMMANDS.md) — Rust 커맨드 시그니처
  - [docs/references/UX_BEHAVIORS.md §첫 실행 UX](../../references/UX_BEHAVIORS.md) — 화면 상태 4종
- **로드맵**: [docs/ROADMAP.md](../../ROADMAP.md) — v1 MVP 첫 항목

---

## 2. Scope

### 2.1 In Scope

- [ ] `src-tauri/binaries/` 디렉토리에 sidecar 바이너리 3종 배치 (ffmpeg, ffprobe, yt-dlp) — Windows `x86_64-pc-windows-msvc` 네이밍 준수 (ffprobe/yt-dlp/python도 동일 규칙)
- [ ] Embedded Python 번들 전략 확정 + `src-tauri/binaries/python/` 배치
- [ ] `tauri.conf.json` `externalBin` 경로 등록 + `capabilities/default.json` shell 권한 추가
- [ ] **`src-tauri/src/commands/common.rs` 신규 모듈 구현** — 후속 피처 공유 유틸:
  - 경로: `sidecar_path()`, `app_data_dir()`, `venv_python_path()`, `torch_cache_path()`
  - 크기 probing: `probe_pypi_wheel_size(pkg)`, `probe_url_size(url)`, `dir_size(path)`
  - 디스크 체크: `check_disk_space(required_mb)`
  - 조합: `estimate_install_size()` (위 probe/metadata 합성)
- [ ] Rust 커맨드 본체 구현:
  - `check_environment` → `EnvStatus` 반환 (내부에서 `estimate_install_size()` 호출 → `install_size_estimate_mb` 필드 포함)
  - `check_internet` → `bool` 반환
  - `install_dependencies` → Channel 진행률 (size 실측 포함) + 완료 시 `Result<(), String>`
  - `cancel_install` → 설치 취소 (child process tree kill)
- [ ] `SetupPage.svelte` **6가지** 상태 UI 구현:
  - ① 환경 감지 중 (detecting)
  - ② 설치 중 (installing) — 진행률 바
  - ③ 이미 설치됨 (ready) — 1초 후 자동 전환
  - ④ 설치 실패 (error)
  - ⑤ 인터넷 없음 (no-internet)
  - ⑥ 디스크 공간 부족 (disk-full) — FR-11 연동, ref UX_BEHAVIORS.md에 신규 추가
- [ ] demucs 설치 중 네트워크 실패 복구 (재시도 버튼)
- [ ] 오류 상세 복사 기능 (개발자용 escape hatch)

### 2.2 Out of Scope

- 앱 자체 자동 업데이트 (v1.2 `SettingsPage`로 이관)
- GPU 가속 감지 (v2 백로그)
- demucs 버전 업그레이드 UI (v1.2 `SettingsPage`)
- htdemucs_ft 외 모델 사전 다운로드 (첫 실행은 기본 모델만, 나머지는 ModelSelector에서 on-demand)
- macOS/Linux sidecar 바이너리 (v2 백로그)
- **앱 uninstall 시 `%APPDATA%/com.rhinoty.mr-extractor/` 삭제 처리** — NSIS 커스텀 스크립트 필요, v1.2 `SettingsPage` 또는 별도 `app-lifecycle` 피처로 이관. 이 피처는 venv를 **생성**만 하고 **삭제는 OS/사용자 수동** (지금은 알려주기만).

---

## 3. Requirements

### 3.1 Functional Requirements

| ID    | Requirement                                                                                                                                          | Priority | Status  |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------- |
| FR-01 | 앱 실행 직후 `SetupPage`가 자동 표시 (현재 구현됨, 검증만 필요)                                                                                      | High     | Pending |
| FR-02 | `check_environment`가 5개 항목 상태를 반환 (ffmpeg/yt-dlp/python/demucs/model)                                                                       | High     | Pending |
| FR-03 | 모든 항목 ready → 1초 후 QueuePage 자동 전환                                                                                                         | High     | Pending |
| FR-04 | 하나라도 missing → **순차 확인**: ① `check_internet` → 실패 시 no-internet 화면(FR-07), ② `check_disk_space(estimate × 1.5)` → 부족 시 disk-full 화면(FR-11), ③ 둘 다 통과 시 `install_dependencies` 자동 호출 (사용자 확인 버튼 없음) | High     | Pending |
| FR-05 | 설치 진행률을 Channel로 프론트엔드에 스트리밍 (step 텍스트 + percent 0~100)                                                                          | High     | Pending |
| FR-06 | 기술 용어 노출 금지 — 표시 문자열은 "음원 분리 엔진", "AI 모델", "실행 환경" 등 한국어 별칭                                                          | High     | Pending |
| FR-07 | 설치 전 `check_internet` 호출 → 실패 시 안내 화면 + 재확인 버튼                                                                                      | High     | Pending |
| FR-08 | 설치 실패 시 오류 화면 + 재시도 버튼 + `[▼ 오류 상세]` 토글 + 클립보드 복사                                                                          | Medium   | Pending |
| FR-09 | 2회차 이후는 **3-way health check**만 하고 바로 진입 (마커 파일만으론 불충분): ① venv 디렉토리 존재 + `Scripts/python.exe` 실행 가능, ② `python -m demucs --help` exit 0 (import 성공), ③ 기본 모델 파일 (`torch-cache/hub/checkpoints/htdemucs_ft-*.th` 4개) 존재. 셋 중 하나라도 실패 시 재설치 트리거. | High     | Pending |
| FR-10 | sidecar 바이너리는 Tauri `externalBin` 규칙 준수 (`ffmpeg-x86_64-pc-windows-msvc.exe`)                                                               | High     | Pending |
| FR-11 | 설치 시작 **전** 디스크 여유 공간 확인. 임계값은 **예상 설치량 × 1.5배 (동적 계산, 하드코딩 금지)**. 부족 시 안내 화면 + breakdown(항목별 크기 + 여유) + 현재 공간 표시. | High     | Pending |
| FR-12 | 모델 다운로드 경로를 `%APPDATA%/com.rhinoty.mr-extractor/torch-cache/`로 리다이렉트 (`TORCH_HOME` 환경변수 설정) — 사용자 홈의 `~/.cache` 오염 방지. | Medium   | Pending |
| FR-13 | 설치 진행 중/완료 후 **실제 디스크 사용량 + 예상 잔여량**을 UI에 표시 (ex: "사용 중 1.7 GB / 예상 2.4 GB"). 완료 화면에 최종 실측값 명시. | High     | Pending |
| FR-14 | 예상 설치량은 **동적 probing으로 계산** — ① 로컬: sidecar 파일 `fs::metadata`, ② 원격: `GET https://pypi.org/pypi/{torch,demucs}/json` wheel size 파싱 + 모델 파일 `HEAD` Content-Length. probing 실패 시 보수적 상수 fallback + "정확한 크기를 확인할 수 없어요" 힌트. | High     | Pending |
| FR-15 | 기본 모델(htdemucs_ft) 외 추가 모델(htdemucs, htdemucs_6s)은 **setup-page에서 설치하지 않음**. QueuePage ModelSelector에서 선택 시 on-demand 다운로드 (후속 피처 책임). setup-page는 기본 모델만 보장. | High     | Pending |

### 3.2 Non-Functional Requirements

| Category               | Criteria                                                                                       | Measurement Method                                                                      |
| ---------------------- | ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Performance            | 2회차 이후 감지 완료 < 2초                                                                     | `console.time` in SetupPage `onMount`                                                   |
| Performance            | 첫 설치 완료 < 5분 (100Mbps 기준)                                                              | 수동 측정, Windows 10/11                                                                |
| Reliability            | 네트워크 중단 후 재시도 시 이어받기 또는 깨끗한 재시도                                         | pip 설치 중 Wi-Fi 끄고 재시도 시 멱등성 검증                                            |
| Bundle Size            | 최종 설치 파일 **< 250MB** (demucs 제외, ffmpeg+ffprobe+yt-dlp+Python 포함, NSIS LZMA 압축 후) | `pnpm tauri build` 산출물 크기 확인. 초과 시 ffmpeg/ffprobe 공유(단일 바이너리화) 검토. |
| Disk Space (영속)      | 설치 완료 후 `%APPDATA%/com.rhinoty.mr-extractor/` 실사용은 **동적 probing 결과에 의존** (현재 PyTorch/demucs/htdemucs_ft 기준 ~2.5GB 예상). 값은 참고치일 뿐, UI에 표시되는 모든 수치는 `estimate_install_size()` 결과에서 파생되어야 함. | 설치 직후 `common::dir_size(app_data_dir)` 측정 → Channel로 UI 전달                     |
| Dynamic Sizing         | Plan/UI/코드 어디에도 설치량/디스크 임계값 **하드코딩 금지**. 허용되는 상수는 probing 실패 시 `CONSERVATIVE_ESTIMATE_MB` 1개뿐. | 코드 리뷰 시 `2.5 GB`/`3 GB`/`4 GB`/`3_000_000_000` 같은 literal 검색 → 위 상수 외엔 전부 flag |
| UX                     | 진행률 바 최소 2초마다 업데이트 (멈춰 보이면 안 됨)                                            | Channel 이벤트 빈도 로그                                                                |
| Error Clarity          | 실패 시 한국어 안내 + 복사 가능한 원본 에러 제공                                               | UI 스펙 준수 검증                                                                       |
| **Offline Capability** | `check_internet`은 **setup-page에서만** 호출. 이후 process/player/export는 오프라인 완전 동작  | 수동: setup 완료 후 Wi-Fi 끄고 로컬 파일 MR 추출 + 재생 + 내보내기 모두 동작 확인       |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] SC-1: 클린 환경(demucs 미설치) Windows + **100Mbps 네트워크**에서 앱 실행 → **5분 이내** QueuePage 진입 (클릭 0회). 더 느린 네트워크에서는 비례 연장 허용.
- [ ] SC-2: 이미 설치된 환경에서 재실행 → 2초 이내 QueuePage 진입
- [ ] SC-3: **첫 실행** Wi-Fi 끈 상태에서 실행 → "인터넷 연결이 필요해요" 화면 표시, [🔄 다시 확인] 동작 (2회차는 FR-09 health check 통과 시 인터넷 무관 진입)
- [ ] SC-4: 설치 중 Wi-Fi 끊기 → 오류 화면 + [🔄 다시 시도] 동작
- [ ] SC-5: `pnpm tauri build` 성공, 설치 파일에 ffmpeg/yt-dlp/python 포함
- [ ] SC-6: Rust 컴파일 경고 0개, TypeScript 에러 0개
- [ ] SC-7: `check_environment`가 Plan §3 FR-02의 5개 항목을 정확히 반환
- [ ] SC-8: 에러 메시지에 Python 모듈 이름 등 기술 용어 노출 없음 (설치 과정 UI 전수 검증)
- [ ] SC-9: 디스크 공간 `estimate × 1.5` 미만 환경에서 실행 → 설치 시작 전 안내 화면 (FR-11). 임계값을 하드코딩하지 않았는지 소스 리뷰.
- [ ] SC-10: `%APPDATA%/com.rhinoty.mr-extractor/venv/` 수동 삭제 후 재실행 → 재설치 트리거 (FR-09 health check 동작)
- [ ] SC-11: 설치 중/완료 화면에 **실측 크기 + 예상 크기** 동시 노출 (FR-13). 완료 후 숫자가 실제 `%APPDATA%` 폴더 크기와 ±5% 이내 일치.
- [ ] SC-12: pypi.org 차단된 환경에서 설치 시작 → 보수적 fallback 추정치 사용 + "정확한 크기 확인 불가" 힌트 (FR-14).
- [ ] SC-13: ModelSelector에서 htdemucs_6s 선택 시 → setup-page가 아닌 **별도 피처**가 다운로드 (setup-page는 기본 모델만) (FR-15).

### 4.2 Quality Criteria

- [ ] Gap Analysis Match Rate ≥ 90%
- [ ] 한국어 문자열 전수 검수 (ref [UX_BEHAVIORS.md §UX 규칙](../../references/UX_BEHAVIORS.md))
- [ ] 빌드 아티팩트 크기 회귀 검증 (v0.1.0 대비 증분 리포트)

---

## 5. Risks and Mitigation

| Risk                                                                     | Impact | Likelihood | Mitigation                                                                                                                                  |
| ------------------------------------------------------------------------ | ------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Embedded Python 번들로 설치 파일 비대화                                  | High   | High       | `python-build-standalone` 사용(압축 후 ~30MB), installer NSIS LZMA 압축. NFR 상한 250MB 수용.                                               |
| demucs/PyTorch pip install 실패 (wheel mismatch, 네트워크)               | High   | Medium     | ① 재시도 로직, ② 상세 에러 노출 토글, ③ FAQ 링크 준비, ④ torch → demucs 순차 설치 실패 시 `pip install --force-reinstall`로 rollback.       |
| Windows Defender SmartScreen 오탐 (서명 없는 exe 차단)                   | High   | Medium     | ① 코드 서명 비용 고려 (별도 티켓), ② 당분간 "알 수 없는 게시자" 안내                                                                        |
| `%APPDATA%/com.rhinoty.mr-extractor/venv/` 부분 생성 후 중단 → 깨진 상태 | Medium | Medium     | `.setup-complete` 마커 + **venv 내부 `python -m demucs --help` 실제 실행 health check**. 둘 중 하나라도 실패 시 venv 통째로 삭제 후 재설치. |
| 디스크 공간 부족 (동적 임계값 `estimate × 1.5` 미만)                     | High   | Low        | 설치 **시작 전** `estimate_install_size()` → `× 1.5` 임계값 계산 → `sysinfo` crate로 비교. 부족 시 breakdown 안내 (FR-11).                   |
| pypi/fbaipublicfiles size probing 실패 (네트워크 제한/기업 프록시)       | Medium | Low        | 보수적 fallback 상수 `CONSERVATIVE_ESTIMATE_MB = 3500` 사용. UI에 "정확한 크기 확인 불가" 힌트. 이 상수가 코드에 남는 유일한 크기 literal. |
| venv 생성 실패 (`python -m venv` 권한/모듈 이슈)                         | Medium | Low        | Embedded Python에 `venv` 모듈 포함 확인 (`python-build-standalone`은 포함됨). 실패 시 에러 상세 + Design에서 복구 전략 확정.                |
| sidecar 파일명 규칙 오류 (target triple 잘못)                            | High   | Low        | CI에 `pnpm tauri build --target` 검증 추가. Tauri 빌드 에러가 명확히 뜨므로 조기 발견 가능.                                                 |
| 첫 실행 타임아웃 (사용자가 "고장난 줄 알고" 종료)                        | Medium | Medium     | 진행률 바 + "처음 실행 시 한 번만, 약 2~3분" 안내 문구. 2초마다 진행률 업데이트 보장.                                                       |
| 다중 인스턴스 실행 시 venv 동시 쓰기 충돌                                | Low    | Low        | Tauri `single-instance` 플러그인 도입 검토 (Design). 우선순위 낮음 — 첫 실행 중 재실행 시도 드물.                                           |
| Windows 백신 격리로 pip install 중 파일 날아감                           | Medium | Low        | FR-09 health check가 detect. 재설치 유도. 사용자에게 "백신 제외 경로 추가" 안내 문구 (§8.2 에러 메시지 패턴에 포함).                        |

---

## 6. Impact Analysis

### 6.1 Changed Resources

| Resource                                  | Type          | Change Description                                                                                                               |
| ----------------------------------------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/tauri.conf.json`               | Config        | `bundle.externalBin` 배열에 4개 항목(ffmpeg/ffprobe/yt-dlp/python) 추가, `build.beforeBuildCommand` 설정                         |
| `src-tauri/capabilities/default.json`     | Config        | `shell:allow-execute` 스코프에 sidecar + Python 명령 추가                                                                        |
| `src-tauri/binaries/`                     | Asset         | 바이너리 3종 + `python/` 디렉토리 신규 추가 (설치 파일 크기 증가). `.gitignore` 등록 (빌드 스크립트 캐시).                       |
| `src-tauri/src/commands/setup.rs`         | Rust Command  | placeholder → 실구현 (check_environment / install_dependencies / check_internet / cancel_install)                                |
| **`src-tauri/src/commands/common.rs`**    | Rust (신규)   | `sidecar_path()`, `app_data_dir()`, `venv_python_path()` 공통 헬퍼 (후속 피처도 공유)                                            |
| `src-tauri/src/commands/mod.rs`           | Rust          | `pub mod common;` 추가                                                                                                           |
| `src-tauri/src/lib.rs`                    | Rust          | `generate_handler![]`에 `check_internet`, `cancel_install` 추가                                                                  |
| **`scripts/download-binaries.js`**        | Node (신규)   | ffmpeg/ffprobe/yt-dlp/python-build-standalone 다운로드 + target triple rename + SHA256 검증 + 캐시                               |
| `package.json`                            | Config        | `scripts.setup:binaries` 추가, `scripts.tauri-dev`/`tauri-build`가 이를 선행 호출                                                |
| `.gitignore`                              | Config        | `src-tauri/binaries/**/*.exe`, `src-tauri/binaries/python/` 추가                                                                 |
| `src/pages/SetupPage.svelte`              | Svelte Page   | 빈 셸 → **6가지** 상태 UI (detecting/installing/ready/error/no-internet/disk-full) + Channel listener + breakdown 렌더링 (실측/예상)  |
| `src/lib/commands.ts`                     | TS Wrapper    | `checkEnvironment` 시그니처 확장, `installDependencies`에 Channel 연동, `checkInternet`, `cancelInstall`                         |
| `src/lib/types.ts`                        | TS Types      | `EnvItem`, `EnvItemStatus`, `EnvStatus`, `InstallProgress`, `SetupPageState` 추가                                                |
| **`%APPDATA%/com.rhinoty.mr-extractor/`** | Runtime Asset | 런타임 생성: `venv/` (demucs 설치), `torch-cache/` (모델), `.setup-complete` (멱등성 마커). **v1.2에서 uninstall 시 삭제 대상.** |
| (API 확장) `EnvStatus`                    | Rust struct   | `install_size_estimate_mb: u64` 필드 추가 (FR-14 probing 결과). 외부 API — `check_environment` 반환값. ref COMMANDS.md 동기화 필요. |
| (내부 헬퍼) `estimate_install_size()`    | Rust fn       | `common.rs` 내부 함수. `check_environment` + `install_dependencies`가 호출. 외부 노출 API 아님. §8.2 Size probing API의 composite. |
| (API 확장) `InstallProgress`              | Channel event | `current_size_mb: Option<u32>`, `estimated_final_mb: u32` 필드 추가 (FR-13). ref COMMANDS.md 동기화 필요.                          |

### 6.2 Current Consumers

첫 피처이므로 기존 consumer 없음. 단, 이 피처 완료 후 아래 피처들이 **sidecar 경로 + 공통 헬퍼에 의존**하게 됨:

| Resource                      | Operation         | Future Consumer                                  | Impact                                                                 |
| ----------------------------- | ----------------- | ------------------------------------------------ | ---------------------------------------------------------------------- |
| ffmpeg sidecar                | invoke subprocess | video.rs (extract_audio), export.rs (export_mix) | 경로 API (`app.shell().sidecar()`) 공통화                              |
| yt-dlp sidecar                | invoke subprocess | youtube.rs (download_youtube)                    | 동일                                                                   |
| Embedded Python + venv        | invoke subprocess | separate.rs (separate_audio)                     | `python -m demucs` 실행 경로 공유. `common::venv_python_path()` 재사용 |
| `probe_pypi_wheel_size()`     | HTTP probing      | v1.2 auto-updater                                | 업데이트 다운로드 크기 사전 확인에 재사용                              |
| `probe_url_size()`            | HTTP HEAD         | **v1.1 ModelSelector**                           | on-demand 모델 다운로드 전 크기 확인 + 디스크 체크에 필수 재사용       |
| `common::dir_size()`          | fs scan           | v1.2 Settings (§저장 공간)                       | 모델별 사용량 breakdown 표시에 재사용                                  |
| `estimate_install_size()`     | composite         | **v1.1 ModelSelector**                           | 추가 모델 다운로드 시 같은 probing 로직 재사용                         |

→ `src-tauri/src/commands/common.rs`에 **단순 sidecar_path()만 두지 말고** size/probing/fs 헬퍼까지 포함. Design 단계에서 모듈 구조 확정 (common 단일 vs common/sidecar.rs + common/sizing.rs 분리).

### 6.3 Verification

- [ ] `pnpm tauri dev` 실행 시 sidecar 경로로 `--version` 호출 성공 확인 (ffmpeg/yt-dlp/python)
- [ ] capabilities 누락으로 권한 거부되는지 확인
- [ ] 재빌드 시 번들 결정성 (같은 바이너리 hash) 확인

---

## 7. Architecture Considerations

### 7.1 Project Level Selection

| Level          | Characteristics                           | Recommended For               | Selected |
| -------------- | ----------------------------------------- | ----------------------------- | :------: |
| **Starter**    | Simple structure                          | Static sites, landing pages   |    ☐     |
| **Dynamic**    | Feature-based modules + Rust command 분리 | Tauri desktop apps, fullstack |    ✅    |
| **Enterprise** | DI, microservices                         | Large-scale systems           |    ☐     |

→ `project-setup`에서 이미 Dynamic 확정. 변경 없음.

### 7.2 Key Architectural Decisions

> **v0.2 확정**: 6건 모두 결정 완료. 근거는 §10 참조.

| Decision                        | Options                                                                | Decision                                            | Status  |
| ------------------------------- | ---------------------------------------------------------------------- | --------------------------------------------------- | :-----: |
| Embedded Python 번들 방식       | (A) python-build-standalone / (B) PyOxidizer / (C) 시스템 Python       | **(A) python-build-standalone**                     | ✅ 확정 |
| sidecar 바이너리 획득 방법      | (A) 수동 다운로드 커밋 / (B) 빌드 스크립트 자동화 / (C) 릴리스 시점 CI | **(B) 빌드 스크립트 (beforeBuildCommand)**          | ✅ 확정 |
| demucs 설치 위치                | (A) Embedded Python `site-packages` 직접 / (B) venv 생성               | **(B) `%APPDATA%/com.rhinoty.mr-extractor/venv/`**  | ✅ 확정 |
| 진행률 UI 세분화                | (A) 단계별(설치중/다운로드중) / (B) 퍼센트만 / (C) 스피너만            | **(A) 단계별 체크리스트 + 퍼센트**                  | ✅ 확정 |
| 네트워크 실패 처리              | (A) 안내화면만 / (B) 오프라인 번들 모드 / (C) 둘 다                    | **(A) 안내 화면 + 재시도 버튼** (setup/유튜브 국한) | ✅ 확정 |
| 피처 scope 분할                 | (A) 단일 피처 / (B) setup-detect + setup-install 분리                  | **(A) 단일 Plan/Design + Do에서 3 Phase 분할**      | ✅ 확정 |
| Rust: subprocess + Channel 패턴 | 기존 confirmed                                                         | `tokio::process::Command` + `tauri::ipc::Channel`   | ✅ 확정 |
| Svelte: 상태 관리               | Component-local `$state` vs stores                                     | Component-local `$state`                            | ✅ 확정 |

### 7.3 Clean Architecture Approach

```
Selected Level: Dynamic (Tauri desktop variant)

┌─── 빌드 시점 (저장소 / 번들) ──────────────────────────────┐
│ scripts/                                                   │
│   └── download-binaries.js     ← 빌드 전 sidecar 다운로드  │ (신규)
│ src-tauri/                                                 │
│   ├── binaries/                (.gitignore, 빌드 시 채움)  │
│   │   ├── ffmpeg-x86_64-pc-windows-msvc.exe    (~70MB)     │
│   │   ├── ffprobe-x86_64-pc-windows-msvc.exe   (~70MB)     │
│   │   ├── yt-dlp-x86_64-pc-windows-msvc.exe    (~15MB)     │
│   │   └── python/              (~30MB, 압축해제 후)        │
│   │       ├── python.exe                                   │
│   │       ├── python310.dll                                │
│   │       └── Lib/           ← venv 모듈 포함, 사용자 패키지는 여기에 설치 X │
│   └── src/commands/                                        │
│       ├── setup.rs             ← 본 피처 핵심              │
│       ├── common.rs            ← sidecar_path() 공통 헬퍼 (신규) │
│       └── ...                                              │
└────────────────────────────────────────────────────────────┘

┌─── 런타임 (%APPDATA% — 사용자별) ──────────────────────────┐
│ %APPDATA%\com.rhinoty.mr-extractor\                                    │
│   ├── venv/                    ← 첫 실행 시 생성           │
│   │   ├── Scripts/python.exe                               │
│   │   └── Lib/site-packages/   ← demucs + torch 여기 설치  │
│   ├── torch-cache/             ← TORCH_HOME 리다이렉트     │
│   │   └── hub/checkpoints/htdemucs_ft-*.th                 │
│   └── .setup-complete          ← 멱등성 마커 (schema v1)   │
└────────────────────────────────────────────────────────────┘

┌─── Frontend ───────────────────────────────────────────────┐
│ src/pages/SetupPage.svelte    ← 6가지 상태 UI              │
│ src/lib/commands.ts           ← checkEnv, installDeps, ... │
│ src/lib/types.ts              ← EnvItem, InstallProgress   │
└────────────────────────────────────────────────────────────┘
```

**경로 전략 요약**:

- **빌드 시점 Asset** (`src-tauri/binaries/`) → 앱 번들에 포함, 읽기 전용
- **런타임 State** (`%APPDATA%\com.rhinoty.mr-extractor\`) → 사용자 쓰기 가능, 재설치 대비 영속화
- 두 영역이 섞이지 않도록 `common.rs`에서 경로 엄격 분리

---

## 8. Convention Prerequisites

### 8.1 Existing Project Conventions

- [x] `CLAUDE.md` — 이미 규칙 확정 (다크 테마, Svelte 5 runes, `@tauri-apps/api/core`, sidecar 허용 목록)
- [x] Design & Implementation Checklist (CLAUDE.md §Design Checklist) — 자동 검증 대상
- [x] ESLint/Prettier — 기본값
- [x] `tsconfig.json` — 이미 구성됨

### 8.2 Conventions to Define/Verify (이 피처에서 확정할 것)

| Category                   | Current State | To Define                                                                                                                                 | Priority |
| -------------------------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | :------: |
| **사용자 표시 문자열**     | ref에 정의됨  | [ref COMMANDS.md §사용자에게 보이는 이름 매핑](../../references/COMMANDS.md) 5개 매핑 테이블 준수. Plan에선 중복 기술 금지.                |   High   |
| **sidecar 경로 헬퍼**      | missing       | `src-tauri/src/commands/common.rs::sidecar_path(name)` 단일 유틸                                                                          |   High   |
| **%APPDATA% 경로 헬퍼**    | missing       | `common::app_data_dir()`, `common::venv_python_path()`, `common::torch_cache_path()` — 모두 `{appDataDir}/com.rhinoty.mr-extractor/` 기반 |   High   |
| **설치 마커 파일 스키마**  | missing       | `.setup-complete` JSON: `{ version: 1, installedAt, demucsVersion, modelSha256 }` — 버전 관리 대비                                        |   High   |
| **EnvItem/EnvStatus 구조** | missing       | ref COMMANDS.md 명시: `EnvItem { label, status, version }` + enum `EnvItemStatus { Ready, Missing, Installing, Error }`                   |   High   |
| **Channel 이벤트 타입**    | 부분적        | `InstallProgress { step, percent, phase, current_size_mb: Option<u32>, estimated_final_mb: u32 }` — size 필드 2종 신규 (FR-13). ref 동기화 필요. | High |
| **Health check 로직**      | missing       | `check_environment` 시 `python -m demucs --help` 실행 → 0이면 ready, 아니면 missing                                                       |   High   |
| **에러 메시지 포맷**       | missing       | `{친절한 한국어}` + "백신 예외 등록 안내" 힌트 + `[▼ 상세] {원본}` 토글                                                                   |  Medium  |
| **환경 변수 주입 규칙**    | missing       | Python subprocess 실행 시: `TORCH_HOME={appdata}/torch-cache`, `PIP_CACHE_DIR={appdata}/pip-cache`, `PYTHONUNBUFFERED=1`, **`PATH` 앞에 sidecar ffmpeg 디렉토리 prepend** (demucs 내부 ffmpeg 폴백 대응, ref FILE_FORMATS.md) |   High   |
| **`check_internet` URL**   | missing       | ref COMMANDS.md 명시: `HEAD https://pypi.org` (1차), `https://dl.fbaipublicfiles.com` (2차 fallback). 타임아웃 3초.                        |  Medium  |
| **Size probing API**       | missing       | `common::probe_pypi_wheel_size(pkg: &str) -> Result<u64>` (GET pypi.org/pypi/{pkg}/json, 최신 win_amd64 wheel size 파싱), `common::probe_url_size(url) -> Result<u64>` (HEAD Content-Length), `common::dir_size(path) -> u64` (재귀 scan). 타임아웃 각 3초. | High |
| **Fallback 상수**          | missing       | `const CONSERVATIVE_ESTIMATE_MB: u64 = 3500;` — probing 실패 시 사용. 코드에 유일하게 허용되는 크기 상수. 그 외 모든 숫자는 동적.          |   High   |

### 8.3 Environment Variables Needed

해당 없음. Tauri 앱은 `appDataDir()` API로 경로 해결, 환경 변수 의존 없음.

### 8.4 Pipeline Integration

이 프로젝트는 9-phase pipeline 대신 PDCA + CLAUDE.md 규칙 기반으로 운영. N/A.

---

## 9. Next Steps

1. [x] §10 Key Decisions 6건 사용자 확정 (v0.2에서 완료)
2. [ ] `/pdca design setup-page` → 3가지 아키텍처 옵션 생성 후 선택
3. [ ] Design 확정 후 `/pdca do setup-page --scope phase-1` (sidecar + Python 번들)
4. [ ] Phase 2 (demucs 자동 설치), Phase 3 (에러/오프라인) 세션 분할 구현

### 9.1 Design 단계 필수 포함 체크리스트 (§10.6 후속)

scope를 3 Phase로 쪼갠 만큼 Design에서 다음 **4+5가지**가 **반드시** 기술돼야 함 — 누락 시 중간 상태에서 앱이 깨짐:

**필수 구조 (§10.6 후속):**

- [ ] **Phase별 Interface Contract** — Phase 1이 export하는 Rust 타입/커맨드/Svelte 타입을 정확히 명시. Phase 2/3가 어떻게 확장하는지 동일 문서에 기록.
- [ ] **Phase 간 의존성 그래프** — 다이어그램 (Phase 1: 감지만 → Phase 2: 설치 파이프라인 → Phase 3: 오류 경로). 의존성 역류 금지.
- [ ] **Success Criteria ↔ Phase 매핑** — SC-1~10 (§4.1)을 각 Phase에 배분. Phase별로 "완료 = 어떤 SC 충족" 명확화.
- [ ] **중간 상태 무결성** — Phase 1만 완료해도 `pnpm tauri dev` 실행 가능 + UI 깨지지 않음. Phase 2 미구현 상태에서도 "감지는 되고, 설치 UI는 빈 placeholder"로 앱이 열려야 함.

**세부 엣지 케이스 결정 (Plan v0.3 gap fix 후속):**

- [ ] **Embedded Python 압축 해제 시점** — 빌드 시점에 해제해서 sidecar로 포함? 런타임에 첫 실행 시 해제? (`python-build-standalone`은 `.tar.zst`)
- [ ] **venv 생성 실패 복구 전략** — Embedded Python에 `venv` 모듈 누락 시? 디렉토리 권한 거부 시? 재시도 정책.
- [ ] **부분 설치 rollback 흐름** — `pip install torch` 성공 → `pip install demucs` 실패. 재시도 시 `--force-reinstall` 사용? 아니면 venv 통째로 폐기?
- [ ] **pip subprocess 타임아웃/취소** — `cancel_install` 호출 시 child process tree kill. Windows에서 pip이 spawn한 자식 프로세스까지 잡는 방법 (Rust `std::process::Child::kill()`은 직접 자식만 kill).
- [ ] **환경 변수 주입 규칙 구체화** — `TORCH_HOME`, `PIP_CACHE_DIR`, `PYTHONUNBUFFERED=1` (진행률 스트림 버퍼링 방지). `tokio::process::Command::env()` 일괄 주입 헬퍼.

### 9.2 후속 피처 연관

| 후속 피처 | 의존 | 세부 |
|---|---|---|
| **v1.1 ModelSelector** | `common::probe_url_size`, `common::dir_size`, `estimate_install_size` 재사용 | ref MODEL_SELECTOR.md §on-demand 다운로드 플로우에 따라 htdemucs/htdemucs_6s 선택 시 크기 확인 → 디스크 체크 → 다운로드. setup-page FR-15 참조. |
| **v1.2 SettingsPage** | `common::dir_size` 재사용 | ref SETTINGS.md §AI 모델 섹션 — 모델별 실측 크기 breakdown + 개별 삭제 UI |
| **v1.2 app-lifecycle** | `%APPDATA%/com.rhinoty.mr-extractor/` 경로 | uninstall 다이얼로그로 venv + 모델 일괄 삭제 (§10.3 Q1 답변 참조) |
| **v1.2 auto-updater**  | `probe_pypi_wheel_size` 재사용 | 업데이트 다운로드 크기 사전 확인 |
| process/player/export  | 오프라인 동작 보장 | §3.2 NFR Offline Capability 참조 |

---

## 10. Key Decisions Needed (웹 개발자를 위한 심층 분석)

> 6개 의사결정을 내려야 Design 작성이 가능합니다. 각 항목은 **웹 개발자 관점에서 이해 가능한 비유 + 트레이드오프 + 권장안**으로 구성했습니다.

---

### 10.1 Embedded Python 번들 전략

**웹 개발자 비유**: 서버리스 함수에 Node 런타임을 포함시키느냐(AWS Lambda layer), 외부 런타임을 요구하느냐(Cloudflare Workers에서 Deno 설치 요구)의 차이.

| 옵션                            | 설치 파일 크기 | 설치 안정성 | 개발 편의성 | 특이사항                                                                                           |
| ------------------------------- | -------------- | ----------- | ----------- | -------------------------------------------------------------------------------------------------- |
| **(A) python-build-standalone** | +30~40MB       | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐    | indygreg가 만든 이식성 좋은 Python. 그냥 폴더 복사. demucs도 잘 돌아감. **업계 표준.**             |
| **(B) PyOxidizer (단일 .exe)**  | +25MB          | ⭐⭐        | ⭐          | 단일 exe로 패키징. 하지만 PyTorch처럼 C 확장 많은 패키지는 지옥. demucs 설치 시 깨질 수 있음.      |
| **(C) 시스템 Python 의존**      | 0MB            | ⭐⭐        | ⭐⭐        | 사용자가 Python을 이미 설치해야 함. 버전 충돌(3.9/3.10/3.11) 지옥. CLAUDE.md "클릭 0회" 원칙 위배. |

**권장: (A) python-build-standalone**

**이유 3가지**:

1. **"그냥 동작한다"** — 폴더 복사 후 `python.exe -m pip install demucs` 하면 끝. PyOxidizer는 C 확장 호환성 이슈로 demucs 같은 패키지에서 자주 깨짐.
2. **설치 파일 증분이 실사용 허용 수준** — 30MB는 Discord(100MB+), Notion(200MB+) 대비 작음. 압축 후 설치 파일 최종 ~80MB 예상.
3. **CLAUDE.md 핵심 원칙 부합** — "최소 설치로 바로 사용". (C)는 이 원칙 직접 위배.

**위험**: 첫 빌드에서 `python-build-standalone` 다운로드(~50MB) 필요. 빌드 머신에서 한 번만 하면 되므로 수용 가능.

**선택지**: (A) / (B) / (C) → **내 선택: A**

---

### 10.2 sidecar 바이너리 획득 방법

**웹 개발자 비유**: `node_modules`를 git에 커밋하느냐(수동), `package-lock.json`으로 재현하느냐(자동)의 차이.

| 옵션                                              | 장점                              | 단점                                                                     | 트레이드오프                                     |
| ------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------ |
| **(A) 수동 다운로드 후 git 커밋**                 | 단순. 첫 clone 후 바로 빌드.      | 저장소 크기 150MB+ 증가. 업데이트 수동.                                  | 초보자 친화적. `.gitignore`로 나중에 뺄 수 있음. |
| **(B) 빌드 스크립트 (beforeBuildCommand)**        | 저장소 깨끗. 버전 한 곳에서 관리. | 첫 빌드 느림(~2~3분 다운로드). 네트워크 필요.                            | 팀 개발/CI에 유리.                               |
| **(C) 릴리스 시점 GitHub Actions에서만 다운로드** | 로컬 개발 시 가벼움.              | 로컬 `pnpm tauri dev`는 여전히 바이너리 필요 → 결국 (A)나 (B) 혼합 필요. | 혼자 개발이면 오버엔지니어링.                    |

**권장: (B) 빌드 스크립트**

**이유**:

1. **저장소 가볍게 유지** — 150MB 커밋은 git clone/fetch를 느리게 함. 나중에 협업 시 후회.
2. **버전 업그레이드 쉬움** — 스크립트에 버전 상수 하나 바꾸면 됨. (A)는 수동 재커밋.
3. **Tauri v2 `beforeBuildCommand` 공식 지원** — `package.json`에 `"setup:binaries": "node scripts/download-binaries.js"` 추가 → `tauri.conf.json` `build.beforeBuildCommand` 지정.

**구현 방식 제안**:

```javascript
// scripts/download-binaries.js (Node 기반, fetch + fs)
const BINARIES = {
  ffmpeg: 'https://github.com/BtbN/FFmpeg-Builds/releases/...',
  'yt-dlp': 'https://github.com/yt-dlp/yt-dlp/releases/latest/...',
};
// 이미 있으면 skip (캐시), 없으면 다운로드 + rename to target triple
```

**단점 대처**: 첫 빌드 느린 건 `.gitignore`에서 `src-tauri/binaries/*.exe` 빼고 **로컬 캐시는 유지** → 두 번째 빌드부터는 skip.

**선택지**: (A) / (B) / (C) → **내 선택: B**

---

### 10.3 demucs 설치 위치

**웹 개발자 비유**: 글로벌 `npm install -g` vs 프로젝트 로컬 `node_modules`의 차이. 단 여기선 "앱 전용 파이썬"이라 글로벌 오염 우려는 없음.

| 옵션                                                    | 장점                                    | 단점                                                    |
| ------------------------------------------------------- | --------------------------------------- | ------------------------------------------------------- |
| **(A) Embedded Python의 site-packages 직접**            | 단순. 경로 하나. 속도 빠름.             | 앱 번들 디렉토리에 쓰기 — 일부 OS에서 권한 이슈 가능성. |
| **(B) `%APPDATA%/com.rhinoty.mr-extractor/venv/` 생성** | 사용자별 격리. Program Files 쓰기 회피. | venv 생성 1회 추가(~10초). 경로 2개 관리.               |

**권장: (B) `%APPDATA%` 아래 venv**

**이유**:

1. **Program Files 쓰기 권한 문제 회피** — Windows에서 앱이 `C:\Program Files\...`에 설치되면 `site-packages`에 pip install이 권한 거부될 수 있음. `%APPDATA%`는 항상 쓰기 가능.
2. **멱등성 보장** — `.setup-complete` 마커 파일을 `%APPDATA%/com.rhinoty.mr-extractor/.setup-complete`에 두면 설치 상태를 사용자 스코프에서 관리. 앱 재설치 해도 venv 유지됨(재설치 시간 단축).
3. **삭제/재시도 쉬움** — 문제 생기면 `%APPDATA%/com.rhinoty.mr-extractor/` 통째로 지우고 재시도. 번들 디렉토리는 건드리지 않아도 됨.

**단점 대처**: venv 생성 10초 추가는 전체 2~3분 설치 대비 무시 가능.

> **참고**: 처음엔 직관적으로 (A)가 나아 보이지만 Windows 권한 이슈로 실전에선 (B)가 표준.

**선택지**: (A) / (B) → **내 선택: B**

> **Follow-up Q**: 사용자가 앱 삭제할 때 venv도 같이 삭제되어야 하지 않나?
>
> **A**: 맞음. Tauri v2 Windows installer는 NSIS 기반이라 `uninstall.nsh` 커스텀으로 가능. 3가지 패턴 중 **(ii) 다이얼로그** ("설정과 설치 파일도 함께 삭제할까요?") 권장. Discord/Slack 표준 UX.
>
> **다만 이건 이 피처 scope에 섞으면 안 됨** — NSIS 커스텀 템플릿은 별도 작업이라 `v1.2 app-lifecycle / SettingsPage`로 이관. 지금은 `%APPDATA%/com.rhinoty.mr-extractor/` 구조만 잘 만들고, uninstall 삭제 로직은 다음 사이클에서.
>
> → §2.2 Out of Scope에 반영 완료.

---

### 10.4 진행률 UI 세분화

**웹 개발자 비유**: 단일 프로그레스 바(업로드 퍼센트만) vs 멀티 스텝 마법사(1/5, 2/5 표시)의 차이.

demucs + PyTorch 설치는 실제로 **여러 단계**로 쪼개짐:

```
① Embedded Python 압축 해제         (~5초,   5%)
② venv 생성 (10.3에서 B 선택 시)    (~10초,  10%)
③ pip install torch                 (~2분,   45%)
④ pip install demucs                (~30초,  55%)
⑤ htdemucs_ft 모델 다운로드         (~1~2분, 100%)
```

> **모델 크기 주의**: `htdemucs_ft`는 **Bag of 4 모델** (ref MODEL_SELECTOR.md) — 총 **~1.3GB** (320MB × 4). 100Mbps 기준 최소 2분. 이전 추정치 "~30초"는 단일 모델 가정. 수정됨.
>
> 전체 설치 시간 재추정: **3~5분 (100Mbps 기준)** — NFR "< 5분"은 유효 범위 내, 여유 없음. Design에서 모델 병렬 다운로드 고려.

| 옵션                                                             | 사용자 안심도 | 구현 난이도 | 노트                                                                               |
| ---------------------------------------------------------------- | ------------- | ----------- | ---------------------------------------------------------------------------------- |
| **(A) 단계별 체크리스트 + 퍼센트** (UX_BEHAVIORS.md 스펙 그대로) | ⭐⭐⭐⭐⭐    | ⭐⭐⭐      | `["오디오 변환 도구", "음원 분리 엔진", ...]` 리스트 + 현재 단계 `⏳` + 전체 % 바. |
| **(B) 전체 퍼센트만**                                            | ⭐⭐⭐        | ⭐          | 설치 오래 걸려서 사용자가 "고장난 줄" 알 가능성.                                   |
| **(C) 무한 스피너**                                              | ⭐            | ⭐          | ㅠㅠ. 2~3분 스피너는 이탈률 폭증.                                                  |

**권장: (A) 단계별 체크리스트**

**이유**:

1. **체감 대기 시간 감소** — 진행 항목이 ✅로 바뀌는 시각적 변화가 "멈추지 않았음"을 증명. 사용자 이탈 방지 핵심.
2. **이미 스펙 존재** — [UX_BEHAVIORS.md](../../references/UX_BEHAVIORS.md#setuppage-화면-상태)에 와이어프레임 있음. 추가 기획 불필요.
3. **구현 오버헤드 적음** — Rust에서 `InstallProgress { step: "음원 분리 엔진 설치 중...", percent: 45 }` 보내는 것만 하면 됨. Svelte 측은 step 변경 시 현재 리스트 항목 index 갱신.

**pip 출력 파싱 방법**: `pip install --progress-bar on` 옵션 사용 → stdout에 `[####    ] 40%` 형태로 나옴. 이걸 줄 단위 파싱.

**선택지**: (A) / (B) / (C) → **내 선택: A**

---

### 10.5 네트워크 실패 처리

**웹 개발자 비유**: offline-first PWA(Service Worker로 오프라인 번들) vs online-required SaaS(인터넷 없으면 안 됨)의 차이.

| 옵션                                                      | 번들 크기 증분  | UX         | 개발 비용 | 추천도                            |
| --------------------------------------------------------- | --------------- | ---------- | --------- | --------------------------------- |
| **(A) 안내 화면 + 재시도 버튼만**                         | +0MB            | ⭐⭐⭐     | 낮음      | v1.0에 적합                       |
| **(B) 오프라인 번들 (demucs + 모델도 설치 파일에 포함)**  | +2GB (!)        | ⭐⭐⭐⭐⭐ | 중간      | 설치 파일이 2.5GB → 배포 비현실적 |
| **(C) 둘 다 제공 (온라인 버전 + 오프라인 버전 2종 배포)** | 둘 다 유지 관리 | ⭐⭐⭐⭐   | 높음      | 사용자 많아지면 고려              |

**권장: (A) 안내 화면만**

**이유**:

1. **2GB 설치 파일은 데스크톱 앱 관행에서 상한선** — Steam 게임급. 실용적이지 않음.
2. **대상 사용자 환경** — 유튜브 URL 기반 앱이므로 **이미 인터넷 전제**. 오프라인 지원은 모순.
3. **필요시 v2에서 (C) 옵션 추가 가능** — (A)로 출발해도 (C)로 확장 쉬움. 역방향은 어려움.

**화면 스펙**: 이미 UX_BEHAVIORS.md §인터넷 없음에 와이어프레임 있음. 그대로 구현.

**선택지**: (A) / (B) / (C) → **내 선택: A**

> **Follow-up Q**: 인터넷 필요 구간 정리 — 초기 설치 + 유튜브 다운로드는 필요, MR 추출은 오프라인 가능?
>
> **A**: 정확함. 구간별로:
>
> | 피처                          |    인터넷     | 이유                                      |
> | ----------------------------- | :-----------: | ----------------------------------------- |
> | setup-page (첫 실행)          |    ✅ 필요    | demucs/PyTorch/모델 다운로드              |
> | queue-page (유튜브 URL)       |    ✅ 필요    | yt-dlp 다운로드                           |
> | queue-page (로컬 파일 드래그) |   ❌ 불필요   | 로컬만                                    |
> | process-page (MR 추출)        | ❌ **불필요** | demucs는 완전 로컬 추론. 모델 이미 캐시됨 |
> | player-page (재생/믹싱)       |   ❌ 불필요   | Web Audio API 로컬                        |
> | export (내보내기)             |   ❌ 불필요   | ffmpeg 로컬                               |
>
> → §3.2 NFR "Offline Capability" 추가 완료. `check_internet`은 **setup-page에서만** 호출 (유튜브 다운로드는 yt-dlp 자체 네트워크 에러로 처리).

---

### 10.6 피처 scope 분할

**웹 개발자 비유**: 모노레포 하나에 다 담느냐, 여러 패키지로 쪼개느냐의 트레이드오프.

| 옵션                                          | 장점                                  | 단점                                             | 사이클 수 |
| --------------------------------------------- | ------------------------------------- | ------------------------------------------------ | --------- |
| **(A) `setup-page` 하나로 전부**              | PDCA 사이클 1번. Design 문서 1개.     | Do 세션이 길어짐(4~6시간). Gap analysis 규모 큼. | 1         |
| **(B) `setup-detect` → `setup-install` 분리** | 세션 짧음(2시간 x 2). 조기 검증 가능. | PDCA 오버헤드 2배. 인터페이스 중복 문서화.       | 2         |

**권장: (A) 단일 피처 — 단, Do는 --scope로 3 Phase 분할**

**이유**:

1. **검증 단위 = 피처 단위가 자연스러움** — "첫 실행 UX"는 하나의 사용자 여정. 감지만 구현하고 설치 빠진 중간 상태는 검증 불가.
2. **PDCA `--scope` 기능이 존재** — `/pdca do setup-page --scope phase-1` 로 감지만, `--scope phase-2` 로 설치만 구현 가능. 세션 분할은 가능하되 Plan/Design은 통합.
3. **Design 문서 §Session Guide 자동 생성** — bkit PDCA가 Module Map 자동 생성해줌. (B)는 이 기능 이점 날림.

**Phase 분할 제안 (v0.5 FR-11~15 반영, Design 단계에서 확정):**

| Phase | 예상 시간 | 구현 범위 | 관련 FR / SC |
|---|---|---|---|
| **Phase 1 — Foundation** | ~3h | sidecar 다운로드 스크립트 + `common.rs` 전체 (경로/probing/size/disk 헬퍼) + `check_environment` + UI Skeleton (detecting/ready 상태) | FR-01/02/10/14, FR-15(경계 정의), SC-5/6/7 |
| **Phase 2 — Install Pipeline** | ~3h | `install_dependencies` + Channel 진행률 + 설치 중 UI + 실측/예상 size streaming + health check + rollback | FR-03/04/05/06/09/12/13, SC-1/2/10/11 |
| **Phase 3 — Error/Guard Paths** | ~2h | `check_internet` + `check_disk_space` guard + 4개 에러 상태 UI (error/no-internet/disk-full) + 재시도 + 오류 복사 | FR-07/08/11, SC-3/4/9/12 |

Phase 1에 common.rs를 전부 넣는 이유: Phase 2/3이 이 헬퍼에 의존하므로 foundation에서 완성해야 의존성 역류 방지.

**선택지**: (A) / (B) → **내 선택: A**

> **Follow-up note**: Design 시 scope대로 보완할 점 없게 설계해야 함.
>
> → §9.1 Design 단계 필수 포함 체크리스트에 반영 완료 (Phase별 Interface Contract, 의존성 그래프, SC↔Phase 매핑, 중간 상태 무결성).

---

### 10.7 결정 요약 테이블 (v0.2 확정)

| #   | 결정 항목             | 선택  | 이유 (간단히)                                                                               |
| --- | --------------------- | ----- | ------------------------------------------------------------------------------------------- |
| 1   | Embedded Python 번들  | **A** | python-build-standalone. PyOxidizer는 PyTorch 깨짐. 30MB 증분 허용선.                       |
| 2   | sidecar 바이너리 획득 | **B** | 빌드 스크립트 (beforeBuildCommand). 저장소 150MB+ 커밋 방지, 캐시로 재빌드 빠름.            |
| 3   | demucs 설치 위치      | **B** | `%APPDATA%/com.rhinoty.mr-extractor/venv/`. Program Files 권한 회피. uninstall 삭제는 v1.2. |
| 4   | 진행률 UI             | **A** | 단계별 체크리스트 + 퍼센트. UX_BEHAVIORS.md 스펙 그대로. 체감 대기 최소화.                  |
| 5   | 네트워크 실패 처리    | **A** | 안내 화면 + 재시도. setup/유튜브만 인터넷 전제, MR 추출 이후는 오프라인.                    |
| 6   | 피처 scope 분할       | **A** | 단일 Plan/Design. Do는 `--scope phase-1/2/3` 로 3세션 분할. Design에서 contract 엄밀화.     |

### 10.8 파생 결정 (Follow-up에서 발생)

| 이슈                       | 결정                                                    | Scope            |
| -------------------------- | ------------------------------------------------------- | ---------------- |
| uninstall 시 venv 삭제     | 다이얼로그 방식 (Discord 패턴)                          | **v1.2 이관**    |
| 인터넷 필요 구간           | setup + 유튜브 다운로드만 필요, 이후 피처 오프라인 동작 | 본 피처 NFR 반영 |
| `check_internet` 호출 위치 | setup-page only (다른 페이지는 호출 금지)               | Design에서 강제  |

---

## Version History

| Version | Date       | Changes                                                                                                                                                                                                                                                                                                                                                                                                              | Author   |
| ------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| 0.1     | 2026-04-24 | Initial draft — 6 key decisions pending (§10)                                                                                                                                                                                                                                                                                                                                                                        | rhino-ty |
| 0.2     | 2026-04-24 | 6 decisions confirmed (A/B/B/A/A/A). 파생 결정 3건 반영 (uninstall v1.2 이관, offline capability NFR, check_internet scope 제한). Design 체크리스트 §9.1 추가.                                                                                                                                                                                                                                                       | rhino-ty |
| 0.3     | 2026-04-24 | Plan self-iterate gap fix (9건). ① §5 venv 모순 제거, ② §7.3 빌드/런타임 경로 분리 다이어그램, ③ §3.2 Bundle Size 100→250MB 현실화, ④ §6.1 누락 Resource 7종 추가 (scripts/, common.rs, .gitignore, %APPDATA% runtime), ⑤ FR-11 (디스크 공간), FR-12 (TORCH_HOME) 추가, ⑥ FR-09 health check 명시, ⑦ SC-9/10 추가, ⑧ §5 Risks 3건 추가 (디스크, venv 실패, multi-instance), ⑨ §9.1 Design 세부 엣지 케이스 5건 추가. | rhino-ty |
| 0.4     | 2026-04-24 | Plan↔Ref 교차검증 gap fix (7건). ① `%APPDATA%` 경로 15곳을 실제 Tauri identifier `com.rhinoty.mr-extractor`로 교체 (ref HISTORY.md도 동반 수정), ② §2.1 상태 UI "4가지→6가지" 동기화 (detecting/installing/ready/error/no-internet/disk-full), ③ §10.4 모델 다운로드 시간 ~30초→~1~2분 현실화 (htdemucs_ft = Bag of 4 ~1.3GB), ④ §8.2 Convention 10종 정비 (EnvItem/EnvStatus ref 참조, check_internet URL 명시, ffmpeg PATH 주입 규칙 추가), ⑤ ref COMMANDS.md/SETTINGS.md/UX_BEHAVIORS.md/HISTORY.md 동반 업데이트. | rhino-ty |
| 0.5     | 2026-04-24 | 동적 사이즈 probing 도입 (5건). ① FR-13~15 신규 (실측/예상 표시, 동적 probing, 기본 모델만 설치), ② FR-11 임계값 "3GB 하드코딩→`estimate × 1.5` 동적", ③ §3.2 NFR Disk Space 동적화 + "하드코딩 금지" 룰, ④ §6.2 Future Consumer에 probing 헬퍼 재사용 매핑 (ModelSelector/auto-updater/Settings), ⑤ §8.2 Convention에 probe API 3종 + fallback 상수 1개 명시. ref 동반: UX_BEHAVIORS 설치/완료 화면 breakdown, MODEL_SELECTOR on-demand 플로우, SETTINGS AI 모델 섹션 breakdown. | rhino-ty |
| 0.6     | 2026-04-24 | Fresh-read High gap fix (8건). ① 설치 시간 "2~3분/3~5분/5분" 혼재 → "3~5분 (100Mbps 기준)" 통일, ② Context Anchor RISK "30~80MB" → "~250MB"로 현실화, ③ SC-1에 "100Mbps" 네트워크 조건 명시, ④ EnvStatus 필드 vs estimate_install_size() 중복 해소 (공개 API는 EnvStatus 필드, 내부 헬퍼는 common.rs fn), ⑤ Phase 분할 v0.2→v0.6 재조정 (FR-11~15 반영, ~2/2/1h → ~3/3/2h, Phase별 FR/SC 매핑 테이블), ⑥ §2.1 In Scope에 common.rs 유틸 8종 추가 (경로/probe/size/disk 헬퍼), ⑦ FR-04에 실행 순서 명시 (missing → check_internet → check_disk → install), ⑧ FR-09 health check 2-way → 3-way (venv + demucs import + 모델 파일). | rhino-ty |
