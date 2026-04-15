# Project Setup Planning Document

> **Summary**: Tauri v2 + Svelte 5 + Tailwind CSS 프로젝트 스캐폴딩 및 기본 구조 확립
>
> **Project**: MR Extractor
> **Author**: rhino-ty
> **Date**: 2026-04-15
> **Status**: Draft

---

## Executive Summary

| Perspective            | Content                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------ |
| **Problem**            | 프로젝트가 문서만 존재하고 실행 가능한 코드가 없음. Tauri + Svelte 기반 앱 셸이 필요 |
| **Solution**           | Tauri v2 프로젝트 스캐폴딩 + 플러그인 설치 + 다크 테마 + 페이지 라우팅 셸 구성       |
| **Function/UX Effect** | `pnpm tauri dev` 실행 시 다크 테마 앱이 뜨고 빈 페이지 전환이 동작하는 상태          |
| **Core Value**         | 이후 모든 기능 구현의 기반. 프로젝트 구조/규칙/도구 체인 확립                        |

---

## Context Anchor

| Key         | Value                                                                                      |
| ----------- | ------------------------------------------------------------------------------------------ |
| **WHY**     | 실행 가능한 앱이 없음 — 스캐폴딩으로 개발 시작점 확보                                      |
| **WHO**     | 개발자 (rhino-ty)                                                                          |
| **RISK**    | Tauri v2 + Svelte 5 조합의 호환성 문제                                                     |
| **SUCCESS** | `pnpm tauri dev` → 다크 테마 앱 실행 + 6개 페이지 전환 동작                                |
| **SCOPE**   | Phase 1: 스캐폴딩 + 플러그인, Phase 2: 테마 + 라우팅 셸, Phase 3: Rust 모듈 구조 + sidecar |

---

## 1. Overview

### 1.1 Purpose

Tauri v2 + Svelte 5 + Tailwind CSS 기반 데스크탑 앱의 초기 프로젝트 구조를 수립한다.
이후 모든 기능(SetupPage, QueuePage, ProcessPage, PlayerPage 등)은 이 셸 위에 구현된다.

### 1.2 Background

- PyQt6에서 Tauri v2로 스택 전환 결정 (라이선스 + UI 퀄리티 + Web Audio)
- 기존 Python 파일은 모두 삭제됨
- 문서(CLAUDE.md, references)는 Tauri 기준으로 이미 재작성 완료

### 1.3 Related Documents

- [CLAUDE.md](../../../CLAUDE.md) — 프로젝트 개요 + 스택
- [docs/ROADMAP.md](../../ROADMAP.md) — MVP 로드맵
- [docs/references/COMMANDS.md](../../references/COMMANDS.md) — Rust 커맨드 스펙

---

## 2. Scope

### 2.1 In Scope

- [ ] `pnpm create tauri-app` 으로 Svelte 5 + TS 프로젝트 생성
- [ ] Tailwind CSS v4 설치 및 설정
- [ ] Tauri 플러그인 설치 (shell, fs, dialog, notification, store, global-shortcut, window-state, process)
- [ ] Rust 플러그인 등록 (`lib.rs`에 `.plugin()`)
- [ ] Capabilities 권한 파일 (`capabilities/default.json`)
- [ ] 다크 테마 CSS 변수 (`--bg`, `--surface`, `--accent` 등)
- [ ] 페이지 라우팅 셸 (App.svelte + 6개 빈 페이지 컴포넌트)
- [ ] Svelte transition 기반 페이지 전환 애니메이션
- [ ] Rust 커맨드 모듈 구조 (`src-tauri/src/commands/`)
- [ ] sidecar 설정 (ffmpeg, yt-dlp 번들 경로)
- [ ] Embedded Python 번들 경로 설정
- [ ] `src/lib/` 구조 (commands.ts, stores.ts, types.ts)
- [ ] `.gitignore` Tauri 빌드 아티팩트 추가

### 2.2 Out of Scope

- 각 페이지의 실제 UI 구현 (이후 Plan)
- demucs/yt-dlp/ffmpeg subprocess 로직
- Web Audio API 믹서, wavesurfer.js
- 히스토리/설정 데이터 관리

---

## 3. Requirements

### 3.1 Functional Requirements

| ID    | Requirement                                                               | Priority | Status  |
| ----- | ------------------------------------------------------------------------- | -------- | ------- |
| FR-01 | `pnpm tauri dev` 실행 시 다크 테마 윈도우가 뜸                            | High     | Pending |
| FR-02 | 6개 페이지(Setup, Queue, Process, Player, History, Settings) 간 전환 동작 | High     | Pending |
| FR-03 | 페이지 전환 시 fade/slide 애니메이션 적용                                 | Medium   | Pending |
| FR-04 | Tauri 플러그인 8개 설치 및 Rust 등록 완료                                 | High     | Pending |
| FR-05 | Capabilities 권한 파일에 필요한 권한 모두 정의                            | High     | Pending |
| FR-06 | Rust 커맨드 모듈 구조 (`commands/` 디렉토리) 생성                         | Medium   | Pending |
| FR-07 | sidecar 경로 설정 (ffmpeg, yt-dlp)                                        | Medium   | Pending |
| FR-08 | Embedded Python 경로 설정                                                 | Medium   | Pending |
| FR-09 | `src/lib/commands.ts`에 Tauri invoke 래퍼 기본 구조                       | Medium   | Pending |
| FR-10 | `pnpm tauri build` 성공 (빌드 에러 없음)                                  | High     | Pending |

### 3.2 Non-Functional Requirements

| Category    | Criteria                                | Measurement Method |
| ----------- | --------------------------------------- | ------------------ |
| Performance | 앱 시작 시간 < 3초 (빈 셸 기준)         | 수동 측정          |
| Bundle Size | Tauri 빌드 결과물 < 15MB (sidecar 제외) | 빌드 후 파일 크기  |
| DX          | `pnpm tauri dev` HMR 동작               | 개발 중 확인       |

---

## 4. Success Criteria

### 4.1 Definition of Done

- [ ] `pnpm tauri dev` 실행 시 다크 테마 윈도우 표시
- [ ] 6개 페이지 전환 동작 (애니메이션 포함)
- [ ] 모든 Tauri 플러그인 등록 완료 (Rust + capabilities)
- [ ] Rust 커맨드 모듈 빈 구조 생성
- [ ] `pnpm tauri build` 에러 없이 성공
- [ ] sidecar 경로 설정 완료

### 4.2 Quality Criteria

- [ ] TypeScript 에러 0개
- [ ] Rust 컴파일 경고 0개
- [ ] 빌드 성공

---

## 5. Risks and Mitigation

| Risk                             | Impact | Likelihood | Mitigation                                                 |
| -------------------------------- | ------ | ---------- | ---------------------------------------------------------- |
| Tauri v2 + Svelte 5 호환성 문제  | High   | Low        | 공식 템플릿 사용, 최신 버전 확인                           |
| Tailwind v4 + Svelte 설정 충돌   | Medium | Medium     | Vite 플러그인 순서 확인, 공식 문서 따르기                  |
| sidecar 바이너리 플랫폼별 네이밍 | Medium | Medium     | Windows(x86_64-pc-windows-msvc) 먼저, 나중에 크로스 플랫폼 |
| Embedded Python 번들 크기        | Low    | Low        | python-build-standalone 사용, 최소 구성                    |
| Rust 컴파일 시간 (첫 빌드)       | Low    | High       | 첫 빌드만 느림, 이후 증분 빌드                             |

---

## 6. Impact Analysis

### 6.1 Changed Resources

| Resource      | Type      | Change Description                              |
| ------------- | --------- | ----------------------------------------------- |
| 프로젝트 전체 | 신규 생성 | Tauri + Svelte + Tailwind 프로젝트 스캐폴딩     |
| .gitignore    | Config    | Tauri 빌드 아티팩트, node_modules, target/ 추가 |
| CLAUDE.md     | Doc       | 필요 시 실제 구조에 맞게 미세 조정              |

### 6.2 Current Consumers

신규 프로젝트이므로 기존 코드 소비자 없음.

### 6.3 Verification

- [ ] 기존 문서(docs/)가 새 프로젝트 구조와 일치하는지 확인
- [ ] .gitignore에 민감 파일 포함 여부 확인

---

## 7. Architecture Considerations

### 7.1 Project Level Selection

| Level          | Characteristics         | Recommended For       | Selected |
| -------------- | ----------------------- | --------------------- | :------: |
| **Starter**    | Simple structure        | Static sites          |    ☐     |
| **Dynamic**    | Feature-based modules   | Web apps, SaaS MVPs   |    ☑     |
| **Enterprise** | Strict layer separation | Complex architectures |    ☐     |

### 7.2 Key Architectural Decisions

| Decision         | Options                       | Selected          | Rationale                               |
| ---------------- | ----------------------------- | ----------------- | --------------------------------------- |
| App Framework    | Tauri v2 / Electron / Flutter | Tauri v2          | 가벼운 번들, MIT 라이선스, Rust 백엔드  |
| Frontend         | Svelte 5 / React / Vue        | Svelte 5          | 공식 Tauri 템플릿, 컴파일러 기반 경량   |
| Styling          | Tailwind CSS v4               | Tailwind CSS v4   | 유틸리티 기반, 다크 테마 쉬움           |
| Language         | TypeScript                    | TypeScript        | 타입 안전성, Tauri invoke 타입 생성     |
| Package Manager  | pnpm                          | pnpm              | 빠른 설치, 디스크 효율적                |
| State Management | Svelte 5 runes ($state)       | Svelte 5 runes    | 프레임워크 내장, 추가 라이브러리 불필요 |
| Routing          | SPA 상태 변수                 | 상태 변수 + {#if} | 6개 페이지에 라이브러리 과잉            |
| Audio            | Web Audio API + Tone.js       | Web Audio API     | 브라우저 내장 오디오 처리               |
| Waveform         | wavesurfer.js                 | wavesurfer.js     | 파형 시각화 표준 라이브러리             |

### 7.3 Clean Architecture Approach

```
Selected Level: Dynamic

Folder Structure:
src/
  lib/
    commands.ts          # Tauri invoke 래퍼
    stores.ts            # Svelte stores (앱 상태)
    types.ts             # TypeScript 타입
  pages/                 # 6개 페이지 컴포넌트
  components/            # 재사용 위젯
  App.svelte             # 루트 + 라우팅

src-tauri/
  src/
    main.rs              # 데스크탑 진입점
    lib.rs               # 앱 빌더 + 플러그인 등록
    commands/
      mod.rs             # 커맨드 모듈 re-export
      setup.rs           # 환경 감지
      youtube.rs         # yt-dlp
      separate.rs        # demucs
      video.rs           # ffmpeg
      export.rs          # 내보내기
    history.rs           # 히스토리 관리
  capabilities/
    default.json         # 권한 정의
  binaries/              # sidecar (ffmpeg, yt-dlp)
  python/                # Embedded Python
```

---

## 8. Convention Prerequisites

### 8.1 Existing Project Conventions

- [x] `CLAUDE.md` has coding conventions section
- [ ] ESLint configuration — 설정 필요
- [ ] Prettier configuration — 설정 필요
- [x] TypeScript configuration — Tauri 템플릿에서 생성

### 8.2 Conventions to Define/Verify

| Category             | Current State         | To Define                              | Priority |
| -------------------- | --------------------- | -------------------------------------- | :------: |
| **Naming**           | CLAUDE.md에 정의      | Svelte: PascalCase, Rust: snake_case   |   High   |
| **Folder structure** | CLAUDE.md에 정의      | pages/, components/, lib/, commands/   |   High   |
| **Import order**     | missing               | Svelte → lib → components → types      |  Medium  |
| **Theme**            | CLAUDE.md에 색상 정의 | CSS 변수 `--bg`, `--surface` 등        |   High   |
| **Error handling**   | missing               | Rust: Result<T, String>, TS: try/catch |  Medium  |

### 8.3 Environment Variables Needed

해당 없음 (데스크탑 앱 — 환경 변수 대신 Tauri Store 사용)

---

## 9. Next Steps

1. [ ] Design 문서 작성 (`project-setup.design.md`)
2. [ ] 구현 시작 (`pnpm create tauri-app`)
3. [ ] 이후 기능 Plan (setup-page, queue-page 등)

---

## Version History

| Version | Date       | Changes       | Author   |
| ------- | ---------- | ------------- | -------- |
| 0.1     | 2026-04-15 | Initial draft | rhino-ty |
