# Commands (Rust Backend)

## Purpose

`src-tauri/src/commands/` Rust 커맨드 상세 스펙.
Tauri `invoke`로 프론트엔드에서 호출. subprocess + Channel로 진행률 스트리밍.

## Current State

미구현. 스펙 문서만 존재.

## Current Rules

- 모든 subprocess는 Rust 측에서 실행 (프론트엔드 shell 직접 호출 금지)
- 진행률은 `Channel<T>` API로 스트리밍
- 에러는 `Result<T, String>` 반환
- Shell scope에 허용된 명령어만 실행 가능

---

## setup.rs — 환경 감지 + 자동 설치

### 감지

```rust
#[derive(Clone, serde::Serialize)]
struct EnvItem {
    label: String,            // 사용자에게 보이는 이름
    status: EnvItemStatus,    // ready, missing, installing, error
    version: Option<String>,
}

#[derive(Clone, serde::Serialize)]
enum EnvItemStatus { Ready, Missing, Installing, Error }

#[derive(Clone, serde::Serialize)]
struct EnvStatus {
    items: Vec<EnvItem>,      // 순서대로 표시
    all_ready: bool,
}

#[tauri::command]
async fn check_environment(app: tauri::AppHandle) -> Result<EnvStatus, String> {
    // 1. ffmpeg sidecar → "오디오 변환 도구"
    // 2. yt-dlp sidecar → "유튜브 다운로더"
    // 3. embedded python → "실행 환경"
    // 4. demucs pip check → "음원 분리 엔진"
    // 5. htdemucs_ft 모델 캐시 → "AI 모델"
}
```

### 자동 설치

```rust
#[tauri::command]
async fn install_dependencies(
    app: tauri::AppHandle,
    on_progress: Channel<InstallProgress>,
) -> Result<(), String> {
    // 1. Embedded Python 확인 (sidecar에 포함)
    // 2. pip install demucs (가장 오래 걸림 — PyTorch 포함)
    //    → on_progress: { step: "음원 분리 엔진 설치 중...", percent: 45 }
    // 3. 모델 사전 다운로드 (htdemucs_ft)
    //    → on_progress: { step: "AI 모델 다운로드 중...", percent: 80 }
}

#[derive(Clone, serde::Serialize)]
struct InstallProgress {
    step: String,       // 사용자에게 보이는 현재 단계
    percent: u32,       // 0~100
}
```

### 사용자에게 보이는 이름 매핑

| 내부 | 표시 텍스트 |
|---|---|
| ffmpeg sidecar | 오디오 변환 도구 |
| yt-dlp sidecar | 유튜브 다운로더 |
| embedded python | 실행 환경 |
| `pip install demucs` | 음원 분리 엔진 |
| 모델 다운로드 | AI 모델 |

### 인터넷 확인

설치 시작 전 네트워크 연결 확인. 없으면 안내 화면 표시.
```rust
#[tauri::command]
async fn check_internet() -> Result<bool, String> {
    // https://pypi.org 또는 https://dl.fbaipublicfiles.com 에 HEAD 요청
}
```

## youtube.rs — yt-dlp 다운로드

```rust
#[tauri::command]
async fn download_youtube(
    app: tauri::AppHandle,
    url: String,
    out_dir: String,
    on_progress: Channel<DownloadProgress>,
) -> Result<String, String> {
    // yt-dlp subprocess
    // stdout에서 진행률 파싱 → on_progress.send()
    // 완료 시 파일 경로 반환
}
```

## video.rs — ffmpeg 오디오 추출

```rust
#[tauri::command]
async fn extract_audio(
    app: tauri::AppHandle,
    video_path: String,
    out_dir: String,
    on_progress: Channel<u32>,
) -> Result<String, String> {
    // ffprobe로 총 재생시간 사전 확인
    // ffmpeg -i video -vn -acodec pcm_s16le -ar 44100 -ac 2 -y out.wav
    // stderr에서 time=HH:MM:SS 파싱 → 진행률 계산
}
```

처리 체이닝:
```
extract_audio → separate_audio → 완료
진행바: 0~20% 추출 / 20~100% 분리
```

## separate.rs — demucs 분리

```rust
#[tauri::command]
async fn separate_audio(
    app: tauri::AppHandle,
    file_path: String,
    model: String,       // htdemucs, htdemucs_ft, htdemucs_6s
    out_dir: String,
    on_progress: Channel<u32>,
) -> Result<SeparationResult, String> {
    // python -m demucs -n {model} --out {out_dir} {file_path}
    // stdout/stderr에서 tqdm 진행률 파싱
    // 완료 후 결과 경로 glob 탐색 (모델명 하드코딩 금지)
}

#[derive(Clone, serde::Serialize)]
struct SeparationResult {
    vocals: String,
    drums: String,
    bass: String,
    other: String,
    // htdemucs_6s 시 추가
    piano: Option<String>,
    guitar: Option<String>,
}
```

### GPU 메모리 옵션

| VRAM | 권장 |
|---|---|
| 3GB 미만 | `--device cpu` |
| 3~6GB | `--segment 7` |
| 7GB+ | 기본값 |

## export.rs — 믹스 내보내기 + 피치 시프트

```rust
#[tauri::command]
async fn export_mix(
    app: tauri::AppHandle,
    stems: Vec<StemConfig>,  // [{path, volume, muted}]
    output_path: String,
    format: String,          // wav, mp3, flac
    bitrate: Option<u32>,    // mp3용 (320)
    semitones: Option<i32>,  // 피치 시프트 (-12 ~ +12, None=원본)
) -> Result<(), String> {
    // ffmpeg로 스템 믹싱 + 포맷 변환
    // 피치 시프트 시: ffmpeg rubberband 필터 사용
    // -af "rubberband=pitch={ratio}" where ratio = 2^(semitones/12)
}
```

### 피치 시프트 전략

| 용도 | 방법 | 위치 |
|---|---|---|
| 실시간 미리듣기 | Tone.js `PitchShift` | 프론트엔드 (Web Audio) |
| 내보내기 | ffmpeg `rubberband` 필터 | Rust (subprocess) |

프론트엔드 실시간 피치:
```typescript
import * as Tone from 'tone';
const pitchShift = new Tone.PitchShift({ pitch: semitones });
// source → pitchShift → destination
```

ffmpeg 내보내기:
```bash
ffmpeg -i input.wav -af "rubberband=pitch={2^(semitones/12)}" output.wav
```

## 프론트엔드 래퍼 (src/lib/commands.ts)

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';

export async function separateAudio(
    filePath: string,
    model: string,
    outDir: string,
    onProgress: (percent: number) => void
) {
    const channel = new Channel<number>();
    channel.onmessage = onProgress;
    return invoke<SeparationResult>('separate_audio', {
        filePath, model, outDir, onProgress: channel
    });
}
```

## Related Docs

- [FILE_FORMATS.md](FILE_FORMATS.md) — 입력 포맷
- [MODEL_SELECTOR.md](MODEL_SELECTOR.md) — 모델 선택
