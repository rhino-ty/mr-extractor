# Quick Reference

## 커맨드

```bash
# 개발
pnpm tauri dev          # Tauri + Svelte HMR
pnpm dev                # 프론트엔드만

# 빌드
pnpm tauri build        # → src-tauri/target/release/bundle/

# 플러그인 추가
pnpm tauri add shell
pnpm tauri add fs
pnpm tauri add dialog
pnpm tauri add notification
pnpm tauri add store
pnpm tauri add global-shortcut
```

## CLI 도구 (Rust에서 subprocess)

```bash
# demucs 분리
python -m demucs -n htdemucs_ft --out <tmp_dir> <input_file>
python -m demucs -n htdemucs_ft --segment 7 --out <tmp_dir> <input_file>  # GPU 메모리 부족
python -m demucs -n htdemucs_ft --two-stems=vocals --out <tmp_dir> <input_file>  # 노래방

# yt-dlp 다운로드
yt-dlp -f bestaudio --extract-audio --audio-format mp3 --audio-quality 320k <url>

# ffmpeg 영상→오디오
ffmpeg -i <video> -vn -acodec pcm_s16le -ar 44100 -ac 2 -y <output.wav>

# ffprobe 메타데이터
ffprobe -v quiet -print_format json -show_format -show_streams <file>
```

⚠ demucs 결과 경로에 모델명 하드코딩 금지 → glob 패턴 탐색

## 앱 데이터 경로

```
출력 폴더:     ~/Desktop/MR Extractor/
앱 데이터:     %APPDATA%/com.mr-extractor.app/
히스토리:      %APPDATA%/com.mr-extractor.app/history.json
설정:          %APPDATA%/com.mr-extractor.app/settings.json
모델 캐시:     ~/.cache/torch/hub/checkpoints/
```

## 테마 색상

```
--bg: #0d0d1a     --surface: #16162e   --border: #2a2a4e   --accent: #7c5cfc
--success: #3dd68c  --warn: #f0a500    --danger: #e05c5c   --muted: #8888aa
```

## 단축키 (주요)

| 키 | 동작 |
|---|---|
| Space | 재생/일시정지 |
| ←/→ | 5초 이동 |
| Shift+←/→ | 30초 이동 |
| ↑/↓ | 볼륨 ±5 |
| M | 전체 뮤트 |
| Ctrl+O | 파일 열기 |
| Ctrl+H | 히스토리 |
| Ctrl+, | 설정 |
| Esc | 뒤로/닫기 |

## Tauri IPC 패턴

```typescript
// 요청-응답
import { invoke } from '@tauri-apps/api/core';
const result = await invoke<string>('command_name', { arg1: 'value' });

// 진행률 스트리밍
import { invoke, Channel } from '@tauri-apps/api/core';
const ch = new Channel<{ percent: number }>();
ch.onmessage = (p) => console.log(p.percent);
await invoke('long_task', { onProgress: ch });
```
