# Project Setup Design Document

> **Summary**: Tauri v2 + Svelte 5 + Tailwind CSS 프로젝트 스캐폴딩 상세 설계
>
> **Project**: MR Extractor
> **Version**: 0.1.0
> **Author**: rhino-ty
> **Date**: 2026-04-15
> **Status**: Draft
> **Planning Doc**: [project-setup.plan.md](../../01-plan/features/project-setup.plan.md)

---

## Context Anchor

| Key | Value |
|-----|-------|
| **WHY** | 실행 가능한 앱이 없음 — 스캐폴딩으로 개발 시작점 확보 |
| **WHO** | 개발자 (rhino-ty) |
| **RISK** | Tauri v2 + Svelte 5 조합의 호환성 문제 |
| **SUCCESS** | `pnpm tauri dev` → 다크 테마 앱 실행 + 6개 페이지 전환 동작 |
| **SCOPE** | Phase 1: 스캐폴딩 + 플러그인, Phase 2: 테마 + 라우팅 셸, Phase 3: Rust 모듈 구조 + sidecar |

---

## 1. Overview

### 1.1 Design Goals

- Tauri v2 + Svelte 5 + Tailwind CSS 프로젝트 기반 확립
- 모든 Tauri 플러그인 사전 등록 (이후 기능에서 바로 사용)
- 다크 테마 전용 디자인 시스템 (CSS 변수)
- 페이지 라우팅 셸 (6개 빈 페이지 + 전환 애니메이션)
- Rust 커맨드 모듈 구조 + sidecar 설정

### 1.2 Design Principles

- 최소 구현: 셸만 만들고 기능은 이후 Plan에서 추가
- CLAUDE.md 구조와 1:1 매칭
- Tauri v2 패턴 준수 (capabilities, plugin 등록, Channel API)

---

## 2. Architecture Options

### 2.0 Architecture Comparison

| Criteria | Option A: Flat | Option B: Feature-based | Option C: Pragmatic |
|----------|:-:|:-:|:-:|
| **Approach** | 모든 파일 flat | 기능별 폴더 | pages/components/lib 분리 |
| **New Files** | ~15 | ~25 | ~20 |
| **Complexity** | Low | High | Medium |
| **Maintainability** | Low | High | High |
| **Effort** | Low | High | Medium |
| **Risk** | 나중에 리팩토링 필요 | 초기 보일러플레이트 과다 | 균형 |
| **Recommendation** | 프로토타입 | 대규모 앱 | **이 프로젝트에 적합** |

**Selected**: Option C — Pragmatic — **Rationale**: 6개 페이지 규모에 feature-based는 과잉, flat은 확장 시 혼란. pages/components/lib 분리가 CLAUDE.md 설계와도 일치.

---

## 2.1 Component Diagram

```
┌──────────────────────────────────────────────┐
│  Svelte Frontend (WebView)                    │
│  ┌─────────┐  ��──────────┐  ┌─────────────┐ │
│  │ pages/  │  │components│  │   lib/      │ │
│  │ 6 pages │  │ widgets  │  │ commands.ts │ │
│  └────┬────┘  └────┬─────┘  │ stores.ts  │ │
│       │            │        │ types.ts   │ │
│       └────────────┴───┬────┘            │ │
│                        │  invoke / Channel│ │
├────────────────────────┼─────────────────┤ │
│  Tauri Core (Rust)     │                  │ │
│  ┌─────────────────────▼──────────────┐  │ │
│  │  commands/                          │  │ │
│  │  setup.rs | youtube.rs | separate.rs│  │ │
│  │  video.rs | export.rs              │  │ │
│  └─────────────────────┬──────────────┘  │ │
│                        │  subprocess      │ │
│  ┌─────────────────────▼──────────────┐  │ │
│  │  sidecar: ffmpeg, yt-dlp           │  │ │
│  │  embedded: python + demucs         │  │ │
│  └────────────────────────────────────┘  │ │
└──────────────────────────────────────────┘ │
```

### 2.2 Data Flow

```
사용자 입력 → Svelte (UI) → invoke() → Rust Command
                                          │
                              ┌───────────┤
                              │           │
                         Channel<T>    Result<T>
                         (진행률)      (완료/에러)
                              │           │
                              ▼           ▼
                         Svelte UI    Svelte UI
                         (프로그레스)  (결과 표시)
```

### 2.3 Dependencies

| Component | Depends On | Purpose |
|-----------|-----------|---------|
| Svelte pages | lib/commands.ts | Tauri invoke 래퍼 |
| lib/commands.ts | @tauri-apps/api/core | IPC |
| Rust commands | tauri_plugin_shell | subprocess 실행 |
| Rust commands | tauri_plugin_fs | 파일 시스템 |
| sidecar | Tauri bundle config | ffmpeg/yt-dlp 번들 |

---

## 3. Data Model

### 3.1 앱 상태 타입 (src/lib/types.ts)

```typescript
// 페이지 라우팅
type PageName = 'setup' | 'queue' | 'process' | 'player' | 'history' | 'settings';

// 큐 아이템 (이후 QueuePage에서 사용)
interface QueueItem {
  id: string;
  type: 'youtube' | 'audio' | 'video';
  source: string;       // URL 또는 파일 경로
  title: string;
  status: 'pending' | 'processing' | 'done' | 'error';
}

// 환경 상태
interface EnvItem {
  label: string;
  status: 'ready' | 'missing' | 'installing' | 'error';
  version?: string;
}

// 분리 결과
interface SeparationResult {
  vocals: string;
  drums: string;
  bass: string;
  other: string;
  piano?: string;   // htdemucs_6s
  guitar?: string;  // htdemucs_6s
}
```

---

## 4. API Specification (Tauri Commands)

이 피처에서는 실제 로직 없이 **빈 커맨드 구조만** 생성.

### 4.1 Command List

| Command | File | Description | 실제 구현 시기 |
|---------|------|-------------|--------------|
| `check_environment` | setup.rs | 환경 감지 | setup-page Plan |
| `install_dependencies` | setup.rs | 자동 설치 | setup-page Plan |
| `download_youtube` | youtube.rs | yt-dlp | queue-page Plan |
| `extract_audio` | video.rs | ffmpeg | process Plan |
| `separate_audio` | separate.rs | demucs | process Plan |
| `export_mix` | export.rs | 내보내기 | player Plan |

### 4.2 Placeholder Command 예시

```rust
// src-tauri/src/commands/setup.rs
#[tauri::command]
pub async fn check_environment() -> Result<String, String> {
    Ok("not implemented".into())
}
```

---

## 5. UI/UX Design

### 5.1 Screen Layout

```
┌────────────────────────────────────────────────┐
│  MR Extractor          [🕐 히스토리] [⚙ 설정]  │  ← 헤더 (공통)
├────────────────────────────────────────────────┤
│                                                │
│              페이지 콘텐츠 영역                  │
│         (Svelte transition:fade 200ms)          │
│                                                │
└────────────────────────────────────────────────┘
```

### 5.2 User Flow (this feature)

```
앱 시작 → SetupPage (빈 셸) → QueuePage (빈 셸)
             ↕                    ↕
        SettingsPage          HistoryPage
```

### 5.3 Component List (this feature)

| Component | Location | Responsibility |
|-----------|----------|----------------|
| App.svelte | src/ | 루트 + 페이지 라우팅 + 공통 헤더 |
| SetupPage.svelte | src/pages/ | 빈 셸 (placeholder 텍스트) |
| QueuePage.svelte | src/pages/ | 빈 셸 |
| ProcessPage.svelte | src/pages/ | 빈 셸 |
| PlayerPage.svelte | src/pages/ | 빈 셸 |
| HistoryPage.svelte | src/pages/ | 빈 셸 |
| SettingsPage.svelte | src/pages/ | 빈 셸 |

### 5.4 Page UI Checklist

#### App.svelte (공통 셸)
- [ ] 헤더: 앱 타이틀 "MR Extractor"
- [ ] 헤더: 히스토리 버튼 (🕐)
- [ ] 헤더: 설정 버튼 (⚙)
- [ ] 페이지 전환: fade 애니메이션 (200ms)
- [ ] 다크 테마: `--bg` 배경색 적용

#### 각 Page (빈 셸)
- [ ] 페이지 이름 표시 (예: "Setup Page")
- [ ] 다크 테마 배경 + 텍스트 색상

---

## 6. Error Handling

### 6.1 Rust Command 에러

```rust
// 모든 커맨드는 Result<T, String> 반환
#[tauri::command]
async fn some_command() -> Result<SomeType, String> {
    some_operation().map_err(|e| e.to_string())
}
```

### 6.2 프론트엔드 에러

```typescript
try {
    const result = await invoke<string>('command_name');
} catch (error) {
    // error는 Rust의 Err(String)
    console.error(error);
}
```

---

## 7. Security Considerations

- [x] Tauri capabilities: 최소 권한 원칙 (필요한 플러그인 권한만)
- [x] Shell scope: 허용된 명령어만 실행 가능
- [ ] CSP 설정 (기본값 사용, 필요 시 조정)

---

## 8. Test Plan

### 8.1 Test Scope

이 피처는 프로젝트 셋업이므로 자동화 테스트 대신 수동 확인.

| Type | Target | Tool | Phase |
|------|--------|------|-------|
| Manual | `pnpm tauri dev` 실행 확인 | 수동 | Do |
| Manual | 6개 페이지 전환 확인 | 수동 | Do |
| Manual | `pnpm tauri build` 성공 확인 | 수동 | Do |

### 8.2 수동 테스트 체크리스트

| # | 테스트 | 기대 결과 |
|---|--------|----------|
| 1 | `pnpm tauri dev` 실행 | 다크 테마 윈도우 표시 |
| 2 | 헤더 버튼 클릭 | 페이지 전환 동작 + fade 애니메이션 |
| 3 | 6개 페이지 순회 | 모든 페이지 렌더링 |
| 4 | `pnpm tauri build` | 에러 없이 빌드 성공 |
| 5 | Rust 컴파일 | 경고 0개 |
| 6 | TypeScript | 에러 0개 |

---

## 9. Clean Architecture

### 9.1 Layer Structure

| Layer | Responsibility | Location |
|-------|---------------|----------|
| **Presentation** | Svelte 페이지/컴포넌트 | `src/pages/`, `src/components/` |
| **Application** | Tauri invoke ��퍼, 상태 관리 | `src/lib/commands.ts`, `src/lib/stores.ts` |
| **Domain** | 타입 정의 | `src/lib/types.ts` |
| **Infrastructure** | Rust 커맨드, subprocess, 파일 I/O | `src-tauri/src/commands/` |

### 9.2 Dependency Rules

```
Svelte Pages → lib/commands.ts → @tauri-apps/api/core → Rust Commands
                lib/stores.ts     (invoke / Channel)      → subprocess
                lib/types.ts                               → filesystem
```

---

## 10. Coding Convention Reference

### 10.1 Naming Conventions

| Target | Rule | Example |
|--------|------|---------|
| Svelte 컴포넌트 | PascalCase | `SetupPage.svelte`, `DropZone.svelte` |
| TS 함수 | camelCase | `separateAudio()`, `checkEnvironment()` |
| TS 타입 | PascalCase | `QueueItem`, `SeparationResult` |
| Rust 함수 | snake_case | `check_environment`, `separate_audio` |
| Rust 구조체 | PascalCase | `EnvStatus`, `InstallProgress` |
| CSS 변수 | kebab-case | `--bg`, `--surface`, `--accent` |

### 10.2 Import Order (Svelte/TS)

```typescript
// 1. Svelte
import { onMount } from 'svelte';
// 2. Tauri
import { invoke } from '@tauri-apps/api/core';
// 3. lib
import { separateAudio } from '$lib/commands';
import { page } from '$lib/stores';
// 4. types
import type { QueueItem } from '$lib/types';
// 5. components
import FileCard from '../components/FileCard.svelte';
```

### 10.3 This Feature's Conventions

| Item | Convention |
|------|-----------|
| 컴포넌트 네이밍 | PascalCase `.svelte` |
| 상태 관리 | Svelte 5 runes (`$state`, `$derived`) |
| 페이지 라우팅 | 상태 변수 + `{#if}` 분기 |
| Rust 에러 | `Result<T, String>` |
| CSS | Tailwind 유틸리티 + CSS 변수 |

---

## 11. Implementation Guide

### 11.1 File Structure

```
mr_extractor/
├── package.json
├── pnpm-lock.yaml
├── svelte.config.js
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.ts        # 또는 Tailwind v4 CSS-first 설정
├── src/
│   ├── app.css               # Tailwind imports + CSS 변수
│   ├── main.ts               # Svelte 마운트
│   ├── App.svelte            # 루트 + 헤더 + 라우팅
│   ├── lib/
│   │   ├── commands.ts       # invoke 래퍼 (빈 구조)
│   │   ├── stores.ts         # page 상태 등
│   │   └── types.ts          # TypeScript 타입
│   ├── pages/
│   │   ├── SetupPage.svelte
│   │   ├── QueuePage.svelte
│   │   ├── ProcessPage.svelte
│   │   ├── PlayerPage.svelte
│   │   ├── HistoryPage.svelte
│   │   └── SettingsPage.svelte
│   └── components/           # (빈 디렉토리 — 이후 추가)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── build.rs
│   ├── icons/
│   ├── binaries/             # sidecar (ffmpeg, yt-dlp) — 빈 디렉토리
│   └── src/
│       ├── main.rs
│       ├── lib.rs            # Builder + 플러그인 등록 + 커맨드 등록
│       └── commands/
│           ├── mod.rs
│           ��── setup.rs      # placeholder
│           ├── youtube.rs    # placeholder
│           ├── separate.rs   # placeholder
│           ├── video.rs      # placeholder
│           └── export.rs     # placeholder
└── docs/                     # 기존 문서 유지
```

### 11.2 Implementation Order

1. [ ] `pnpm create tauri-app` 스캐폴딩 (svelte-ts 템플릿)
2. [ ] Tailwind CSS 설치 + 다크 테마 CSS 변수 설정
3. [ ] Tauri 플러그인 8개 설치 (`pnpm tauri add`)
4. [ ] `src-tauri/src/lib.rs` 플러그인 등록
5. [ ] `src-tauri/capabilities/default.json` 권한 작성
6. [ ] `tauri.conf.json` 앱 설정 (이름, 크기, identifier, sidecar)
7. [ ] `src/lib/types.ts` 타입 정의
8. [ ] `src/lib/stores.ts` 페이지 상태
9. [ ] `src/app.css` 다크 테마 CSS 변수
10. [ ] `src/App.svelte` 헤더 + 라우팅 셸 + transition
11. [ ] `src/pages/*.svelte` 6개 빈 페이지
12. [ ] `src/lib/commands.ts` invoke 래퍼 빈 구조
13. [ ] `src-tauri/src/commands/*.rs` placeholder 커맨드
14. [ ] `pnpm tauri dev` 실행 확인
15. [ ] `pnpm tauri build` 빌드 확인
16. [ ] `.gitignore` Tauri 아티팩트 추가

### 11.3 Session Guide

#### Module Map

| Module | Scope Key | Description | Estimated Turns |
|--------|-----------|-------------|:---------------:|
| Tauri 스캐폴딩 + 플러그인 | `module-1` | create-tauri-app + 플러그인 설치 + 설정 | 15-20 |
| 프론트엔드 셸 | `module-2` | Tailwind + CSS 변수 + App.svelte + pages + lib | 15-20 |
| Rust 백엔드 셸 | `module-3` | commands 모듈 + sidecar 설정 + .gitignore | 10-15 |

#### Recommended Session Plan

| Session | Phase | Scope | Turns |
|---------|-------|-------|:-----:|
| Session 1 (현재) | Plan + Design | 전체 | 완료 |
| Session 2 | Do | `--scope module-1,module-2,module-3` | 40-55 |
| Session 3 | Check + Report | 전체 | 10-15 |

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 0.1 | 2026-04-15 | Initial draft | rhino-ty |
