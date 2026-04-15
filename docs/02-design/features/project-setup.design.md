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
| **WHY** | 실행 가능한 앱이 없음 -- 스캐폴딩으로 개발 시작점 확보 |
| **WHO** | 개발자 (rhino-ty) |
| **RISK** | Tauri v2 + Svelte 5 조합의 호환성 문제 |
| **SUCCESS** | `pnpm tauri dev` -> 다크 테마 앱 실행 + 6개 페이지 전환 동작 |
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

**Selected**: Option C -- Pragmatic -- **Rationale**: 6개 페이지 규모에 feature-based는 과잉, flat은 확장 시 혼란. pages/components/lib 분리가 CLAUDE.md 설계와도 일치.

---

### 2.1 Component Diagram

```
+----------------------------------------------+
|  Svelte Frontend (WebView)                    |
|  +---------+  +----------+  +-------------+  |
|  | pages/  |  |components|  |   lib/      |  |
|  | 6 pages |  | widgets  |  | commands.ts |  |
|  +----+----+  +----+-----+  | stores.ts   |  |
|       |            |        | types.ts    |  |
|       +------------+---+----+             |  |
|                        |  invoke / Channel|  |
+------------------------+-----------------+   |
|  Tauri Core (Rust)     |                  |  |
|  +---------------------v--------------+  |  |
|  |  commands/                          |  |  |
|  |  setup.rs | youtube.rs | separate.rs|  |  |
|  |  video.rs | export.rs              |  |  |
|  +---------------------+--------------+  |  |
|                        |  subprocess      |  |
|  +---------------------v--------------+  |  |
|  |  sidecar: ffmpeg, yt-dlp           |  |  |
|  |  embedded: python + demucs         |  |  |
|  +------------------------------------+  |  |
+------------------------------------------+  |
```

### 2.2 Data Flow

```
사용자 입력 -> Svelte (UI) -> invoke() -> Rust Command
                                           |
                               +-----------+
                               |           |
                          Channel<T>    Result<T>
                          (진행률)      (완료/에러)
                               |           |
                               v           v
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
| Rust commands | tauri_plugin_store | 설정/히스토리 저장 |
| sidecar | Tauri bundle config | ffmpeg/yt-dlp 번들 |

---

## 3. Data Model

### 3.1 앱 상태 타입 (src/lib/types.ts)

이 피처에서는 **라우팅 + 환경 상태 타입만** 정의. 나머지는 이후 피처에서 추가.

```typescript
// 페이지 라우팅
export type PageName = 'setup' | 'queue' | 'process' | 'player' | 'history' | 'settings';

// 환경 상태 (SetupPage용 -- 이후 setup-page 피처에서 확장)
export type EnvItemStatus = 'ready' | 'missing' | 'installing' | 'error';

export interface EnvItem {
  label: string;
  status: EnvItemStatus;
  version?: string;
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

#[tauri::command]
pub async fn install_dependencies() -> Result<String, String> {
    Ok("not implemented".into())
}
```

모든 커맨드는 `pub async fn` + `Result<T, String>` 패턴.

---

## 5. UI/UX Design

### 5.1 Screen Layout

```
+------------------------------------------------+
|  [<- 뒤로]  MR Extractor  [history] [settings] |  <- 헤더 (공통)
+------------------------------------------------+
|                                                |
|              페이지 콘텐츠 영역                  |
|         (Svelte transition:fade 200ms)          |
|                                                |
+------------------------------------------------+
```

- **뒤로 버튼**: PlayerPage -> QueuePage 등 이전 페이지로 돌아갈 때 표시
- SetupPage/QueuePage(메인)에서는 뒤로 버튼 숨김

### 5.2 User Flow (this feature)

```
앱 시작 -> SetupPage (빈 셸) -> QueuePage (빈 셸)
              |                    |
         SettingsPage          HistoryPage
                                   |
                              PlayerPage -> QueuePage (뒤로)
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
- [ ] 헤더: 뒤로 버튼 (<-) -- 특정 페이지에서만 표시
- [ ] 헤더: 히스토리 버튼
- [ ] 헤더: 설정 버튼
- [ ] 페이지 전환: fade 애니메이션 (200ms)
- [ ] 다크 테마: `--bg` 배경색 적용
- [ ] 최소 창 크기: 900x600

#### 각 Page (빈 셸)
- [ ] 페이지 이름 표시 (예: "Setup Page")
- [ ] 다크 테마 배경 + 텍스트 색상
- [ ] 페이지 식별 가능한 아이콘 또는 이모지

---

## 6. Error Handling

### 6.1 Rust Command 에러

```rust
// 모든 커맨드는 Result<T, String> 반환
#[tauri::command]
pub async fn some_command() -> Result<SomeType, String> {
    some_operation().map_err(|e| e.to_string())
}
```

### 6.2 프론트엔드 에러

```typescript
import { invoke } from '@tauri-apps/api/core';

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
| 3 | 뒤로 버튼 클릭 | 이전 페이지로 돌아감 |
| 4 | 6개 페이지 순회 | 모든 페이지 렌더링 |
| 5 | `pnpm tauri build` | 에러 없이 빌드 성공 |
| 6 | Rust 컴파일 | 경고 0개 |
| 7 | TypeScript | 에러 0개 |

---

## 9. Clean Architecture

### 9.1 Layer Structure

| Layer | Responsibility | Location |
|-------|---------------|----------|
| **Presentation** | Svelte 페이지/컴포넌트 | `src/pages/`, `src/components/` |
| **Application** | Tauri invoke 래퍼, 상태 관리 | `src/lib/commands.ts`, `src/lib/stores.ts` |
| **Domain** | 타입 정의 | `src/lib/types.ts` |
| **Infrastructure** | Rust 커맨드, subprocess, 파일 I/O | `src-tauri/src/commands/` |

### 9.2 Dependency Rules

```
Svelte Pages -> lib/commands.ts -> @tauri-apps/api/core -> Rust Commands
                lib/stores.ts     (invoke / Channel)      -> subprocess
                lib/types.ts                               -> filesystem
```

---

## 10. Coding Convention Reference

### 10.1 Naming Conventions

| Target | Rule | Example |
|--------|------|---------|
| Svelte 컴포넌트 | PascalCase | `SetupPage.svelte`, `DropZone.svelte` |
| TS 함수 | camelCase | `separateAudio()`, `checkEnvironment()` |
| TS 타입 | PascalCase | `PageName`, `EnvItem` |
| Rust 함수 | snake_case | `check_environment`, `separate_audio` |
| Rust 구조체 | PascalCase | `EnvStatus`, `InstallProgress` |
| CSS 변수 | kebab-case | `--bg`, `--surface`, `--accent` |

### 10.2 Import Order (Svelte/TS)

```typescript
// 1. Svelte
import { onMount } from 'svelte';
// 2. Tauri
import { invoke } from '@tauri-apps/api/core';
// 3. lib (relative path -- 바닐라 Svelte는 $lib 미지원)
import { separateAudio } from '../lib/commands';
import { page } from '../lib/stores';
// 4. types
import type { PageName } from '../lib/types';
// 5. components
import FileCard from '../components/FileCard.svelte';
```

> **Note**: 바닐라 Svelte(SvelteKit 아닌)에서는 `$lib` alias가 기본 제공되지 않음.
> Vite alias 설정으로 `$lib -> src/lib` 매핑 추가하거나, 상대 경로 사용.
> 이 프로젝트에서는 **Vite alias 설정** 방식 채택:
> ```typescript
> // vite.config.ts
> resolve: { alias: { '$lib': path.resolve('./src/lib') } }
> ```

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

### 11.1 스캐폴딩 전략 (기존 프로젝트 병합)

> **중요**: 현재 `docs/` 폴더가 이미 존재하는 상태.
> `pnpm create tauri-app`은 빈 디렉토리에서 실행해야 함.

**방법: 임시 디렉토리에 생성 후 병합**

```bash
# 1. 임시 디렉토리에 스캐폴딩
cd /tmp
pnpm create tauri-app mr-temp --template svelte-ts

# 2. 생성된 파일을 기존 프로젝트로 복사
#    (docs/, CLAUDE.md, README.md, .git 등 기존 파일은 보존)
cp -r mr-temp/src mr-temp/src-tauri mr-temp/package.json ... D:/DEV/mr_extractor/

# 3. 임시 디렉토리 삭제
rm -rf mr-temp
```

또는 직접 수동 설정 (package.json + Cargo.toml + 설정 파일 직접 작성).

### 11.2 File Structure

```
mr_extractor/
+-- package.json
+-- pnpm-lock.yaml
+-- svelte.config.js
+-- vite.config.ts             # $lib alias 포함
+-- tsconfig.json
+-- src/
|   +-- app.css                # @tailwind + CSS 변수 (Tailwind v4: @import "tailwindcss")
|   +-- main.ts                # Svelte 마운트
|   +-- App.svelte             # 루트 + 헤더 + 라우팅
|   +-- lib/
|   |   +-- commands.ts        # invoke 래퍼 (빈 구조)
|   |   +-- stores.ts          # page 상태
|   |   +-- types.ts           # PageName, EnvItem
|   +-- pages/
|   |   +-- SetupPage.svelte
|   |   +-- QueuePage.svelte
|   |   +-- ProcessPage.svelte
|   |   +-- PlayerPage.svelte
|   |   +-- HistoryPage.svelte
|   |   +-- SettingsPage.svelte
|   +-- components/            # (빈 디렉토리)
+-- src-tauri/
|   +-- Cargo.toml
|   +-- tauri.conf.json
|   +-- capabilities/
|   |   +-- default.json
|   +-- build.rs
|   +-- icons/
|   +-- binaries/              # sidecar (빈 -- 이후 바이너리 추가)
|   +-- src/
|       +-- main.rs
|       +-- lib.rs             # Builder + 플러그인 등록 + 커맨드 등록
|       +-- commands/
|           +-- mod.rs
|           +-- setup.rs       # placeholder
|           +-- youtube.rs     # placeholder
|           +-- separate.rs    # placeholder
|           +-- video.rs       # placeholder
|           +-- export.rs      # placeholder
+-- docs/                      # 기존 문서 유지
```

### 11.3 Tailwind 버전 결정

**Tailwind v4 (CSS-first 설정)** 채택:
- `tailwind.config.ts` 불필요 -- CSS에서 직접 설정
- `src/app.css`에 `@import "tailwindcss"` + `@theme` 블록으로 커스텀 색상 정의

```css
/* src/app.css */
@import "tailwindcss";

@theme {
  --color-bg: #0d0d1a;
  --color-surface: #16162e;
  --color-border: #2a2a4e;
  --color-accent: #7c5cfc;
  --color-success: #3dd68c;
  --color-warn: #f0a500;
  --color-danger: #e05c5c;
  --color-muted: #8888aa;
}
```

### 11.4 Implementation Order

1. [ ] 스캐폴딩 (임시 디렉토리 방식 또는 수동 설정)
2. [ ] Tailwind CSS v4 설치 + `app.css` 다크 테마 설정
3. [ ] Vite alias 설정 (`$lib -> src/lib`)
4. [ ] Tauri 플러그인 8개 설치 (`pnpm tauri add shell fs dialog notification store global-shortcut window-state process`)
5. [ ] `src-tauri/src/lib.rs` -- 플러그인 등록 (`.plugin()` 8개)
6. [ ] `src-tauri/capabilities/default.json` -- 권한 작성
7. [ ] `tauri.conf.json` -- identifier: `com.rhinoty.mr-extractor`, 윈도우 크기, sidecar 경로
8. [ ] `src/lib/types.ts` -- `PageName`, `EnvItem` 타입
9. [ ] `src/lib/stores.ts` -- `page` 상태 (Svelte 5 runes)
10. [ ] `src/App.svelte` -- 헤더 + 뒤로 버튼 + 라우팅 셸 + transition:fade
11. [ ] `src/pages/*.svelte` -- 6개 빈 페이지
12. [ ] `src/lib/commands.ts` -- invoke 래퍼 빈 구조
13. [ ] `src-tauri/src/commands/*.rs` -- 6개 placeholder 커맨드
14. [ ] `src-tauri/src/commands/mod.rs` -- re-export
15. [ ] `.gitignore` -- node_modules, target/, dist/ 추가
16. [ ] `pnpm tauri dev` 실행 확인
17. [ ] `pnpm tauri build` 빌드 확인

### 11.5 Session Guide

#### Module Map

| Module | Scope Key | Description | Estimated Turns |
|--------|-----------|-------------|:---------------:|
| Tauri 스캐폴딩 + 플러그인 + 설정 | `module-1` | create-tauri-app + 플러그인 + capabilities + tauri.conf.json + Vite alias | 15-20 |
| 프론트엔드 셸 | `module-2` | Tailwind v4 + CSS 변수 + App.svelte + 6 pages + lib/ (types, stores, commands) | 15-20 |
| Rust 백엔드 셸 | `module-3` | commands/ 모듈 6개 placeholder + mod.rs + lib.rs 등록 + .gitignore | 10-15 |

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
| 0.2 | 2026-04-15 | 검증 반영: ASCII 다이어그램, $lib alias, 스캐폴딩 전략, Tailwind v4, 뒤로 버튼, store 의존성 | rhino-ty |
