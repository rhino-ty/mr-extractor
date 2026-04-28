#!/usr/bin/env node
// Design Ref: §11.1 — Phase 1 빌드 토대. beforeBuildCommand/beforeDevCommand가 이걸 호출.
// Plan SC-5 — `pnpm tauri build` 성공, 설치 파일에 ffmpeg/yt-dlp/python 포함.
//
// 동작:
//   1. Windows x64 전용 sidecar (ffmpeg, ffprobe, yt-dlp) → src-tauri/binaries/{name}-{triple}.exe
//   2. Embedded Python (python-build-standalone) → src-tauri/binaries/python/
//   3. 이미 존재하면 skip (캐시)
//   4. 네트워크 필요, Node 20+ 내장 fetch 사용 (zero-install)
//
// 제약:
//   - 크기/SHA256 검증은 Phase 1에서 기본 수준. Security Plan §7 언급대로 강화는 v1.x.
//   - 현 스코프: x86_64-pc-windows-msvc. macOS/Linux 확장은 §2.2 Out of Scope.

import { createHash } from 'node:crypto';
import { createWriteStream, existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import { chmod, mkdir, readFile, rename, rm, unlink, writeFile } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const BINARIES_DIR = join(ROOT, 'src-tauri', 'binaries');
const PYTHON_DIR = join(BINARIES_DIR, 'python');
const CACHE_DIR = join(BINARIES_DIR, '.cache');

const TARGET_TRIPLE = 'x86_64-pc-windows-msvc';
const EXE = '.exe';

// Plan §10.2 — 빌드 스크립트 자동화. 버전 한 곳에서 관리 (옵션 B 결정).
const VERSIONS = {
  // FFmpeg: BtbN shared builds. ffmpeg + ffprobe 포함 zip
  // https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest
  ffmpeg: {
    url: 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip',
    archive: 'ffmpeg-win64.zip',
    kind: 'zip',
    extracts: [
      // inside zip: ffmpeg-master-latest-win64-gpl/bin/{ffmpeg,ffprobe}.exe
      { match: /\/bin\/ffmpeg\.exe$/, out: `ffmpeg-${TARGET_TRIPLE}${EXE}` },
      { match: /\/bin\/ffprobe\.exe$/, out: `ffprobe-${TARGET_TRIPLE}${EXE}` },
    ],
  },
  // yt-dlp: 정적 exe
  'yt-dlp': {
    url: 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe',
    kind: 'direct',
    out: `yt-dlp-${TARGET_TRIPLE}${EXE}`,
  },
  // python-build-standalone: 이식성 좋은 Python (Plan §10.1 결정)
  // Release tag format: YYYYMMDD. "latest" 태그가 리다이렉트되는 경우를 위해 명시적 버전 권장.
  // 아래는 최신 안정 버전 예시 — 업그레이드 시 이 값 하나만 변경.
  python: {
    // indygreg python-build-standalone. 3.11.9 + shared-install_only
    url: 'https://github.com/indygreg/python-build-standalone/releases/download/20240415/cpython-3.11.9+20240415-x86_64-pc-windows-msvc-install_only.tar.gz',
    archive: 'python-win64.tar.gz',
    kind: 'tar.gz',
    // inside: python/ (root dir in tarball)
    extractTo: 'python',
  },
};

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

function log(msg) {
  process.stdout.write(`[setup:binaries] ${msg}\n`);
}

function warn(msg) {
  process.stderr.write(`[setup:binaries] ⚠ ${msg}\n`);
}

async function ensureDir(path) {
  if (!existsSync(path)) {
    await mkdir(path, { recursive: true });
  }
}

// 0-byte placeholder를 "있다"로 오인하는 회귀 방지. (Analysis G-I1)
function isUsable(path) {
  if (!existsSync(path)) return false;
  try {
    return statSync(path).size > 0;
  } catch {
    return false;
  }
}

async function download(url, destPath) {
  log(`download ${url}`);
  const res = await fetch(url, { redirect: 'follow' });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} for ${url}`);
  }
  if (!res.body) {
    throw new Error(`empty body for ${url}`);
  }
  await ensureDir(dirname(destPath));
  await pipeline(res.body, createWriteStream(destPath));
}

async function sha256(path) {
  const hash = createHash('sha256');
  const data = await readFile(path);
  hash.update(data);
  return hash.digest('hex');
}

// ─────────────────────────────────────────────
// Archive extraction (zip, tar.gz via native tools)
// ─────────────────────────────────────────────

function run(cmd, args, opts = {}) {
  return new Promise((res, rej) => {
    const child = spawn(cmd, args, { stdio: 'inherit', shell: false, ...opts });
    child.on('error', rej);
    child.on('exit', (code) => (code === 0 ? res() : rej(new Error(`${cmd} exited ${code}`))));
  });
}

// Windows 환경에서 tar 선택 우선순위:
//   1. Git for Windows의 GNU tar 1.35는 .zip 미지원 + "D:\path"를 host:path로 오인 → 회피
//   2. Windows 10+ 내장 BSD tar (System32\tar.exe)는 zip + tar.gz 모두 지원
function nativeTar() {
  if (process.platform === 'win32') {
    const sysRoot = process.env.SystemRoot ?? 'C:\\Windows';
    return join(sysRoot, 'System32', 'tar.exe');
  }
  return 'tar';
}

async function extractZip(archivePath, destDir) {
  await ensureDir(destDir);
  // BSD tar는 zip 지원. cwd로 archive dir 지정해서 GNU tar의 "D:" 오인 회피와 동등 안전.
  await run(nativeTar(), ['-xf', basename(archivePath), '-C', destDir], { cwd: dirname(archivePath) });
}

async function extractTarGz(archivePath, destDir) {
  await ensureDir(destDir);
  await run(nativeTar(), ['-xzf', basename(archivePath), '-C', destDir], { cwd: dirname(archivePath) });
}

// Recursively list files under a dir
function listFiles(dir, acc = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const s = statSync(full);
    if (s.isDirectory()) listFiles(full, acc);
    else acc.push(full);
  }
  return acc;
}

// ─────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────

async function fetchDirect(spec, outName) {
  const outPath = join(BINARIES_DIR, outName);
  if (isUsable(outPath)) {
    log(`skip (cached): ${outName}`);
    return;
  }
  if (existsSync(outPath)) {
    warn(`removing 0-byte placeholder: ${outName}`);
    await unlink(outPath);
  }
  const tmp = join(CACHE_DIR, basename(spec.url));
  if (!isUsable(tmp)) {
    if (existsSync(tmp)) await unlink(tmp);
    await download(spec.url, tmp);
  }
  await rename(tmp, outPath);
  log(`ready: ${outName}`);
}

async function fetchZip(spec) {
  // Check if all expected outputs exist AND have content
  const allUsable = spec.extracts.every((e) => isUsable(join(BINARIES_DIR, e.out)));
  if (allUsable) {
    log(`skip (cached): ${spec.extracts.map((e) => e.out).join(', ')}`);
    return;
  }

  // 0-byte placeholder 정리
  for (const e of spec.extracts) {
    const outPath = join(BINARIES_DIR, e.out);
    if (existsSync(outPath) && !isUsable(outPath)) {
      warn(`removing 0-byte placeholder: ${e.out}`);
      await unlink(outPath);
    }
  }

  const archivePath = join(CACHE_DIR, spec.archive);
  const extractDir = join(CACHE_DIR, basename(spec.archive, '.zip'));

  if (!isUsable(archivePath)) {
    if (existsSync(archivePath)) await unlink(archivePath);
    await download(spec.url, archivePath);
  }
  if (!existsSync(extractDir)) {
    await extractZip(archivePath, extractDir);
  }

  // Find matching files inside extracted dir
  const files = listFiles(extractDir);
  for (const ex of spec.extracts) {
    const outPath = join(BINARIES_DIR, ex.out);
    if (isUsable(outPath)) continue;
    const hit = files.find((f) => ex.match.test(f.replace(/\\/g, '/')));
    if (!hit) {
      throw new Error(`not found in archive: ${ex.match}`);
    }
    await ensureDir(dirname(outPath));
    await rename(hit, outPath);
    log(`ready: ${ex.out}`);
  }
}

async function fetchPython(spec) {
  const pythonExe = join(PYTHON_DIR, 'python.exe');
  if (isUsable(pythonExe)) {
    log(`skip (cached): python/`);
    return;
  }

  // 0-byte placeholder 정리 (python.exe 단독으로 만들어진 깨진 상태)
  if (existsSync(PYTHON_DIR) && !isUsable(pythonExe)) {
    warn(`removing broken python/ (python.exe is empty)`);
    await rm(PYTHON_DIR, { recursive: true, force: true });
  }

  const archivePath = join(CACHE_DIR, spec.archive);
  const extractTmp = join(CACHE_DIR, 'python-extract');

  if (!isUsable(archivePath)) {
    if (existsSync(archivePath)) await unlink(archivePath);
    await download(spec.url, archivePath);
  }

  if (existsSync(extractTmp)) {
    await rm(extractTmp, { recursive: true, force: true });
  }
  await ensureDir(extractTmp);
  await extractTarGz(archivePath, extractTmp);

  // python-build-standalone tarball root is `python/`
  const rootPython = join(extractTmp, 'python');
  if (!existsSync(rootPython)) {
    throw new Error(`expected python/ inside archive, got ${readdirSync(extractTmp).join(', ')}`);
  }

  // Move into final location
  if (existsSync(PYTHON_DIR)) {
    await rm(PYTHON_DIR, { recursive: true, force: true });
  }
  await rename(rootPython, PYTHON_DIR);
  log(`ready: python/ (python.exe at ${pythonExe})`);
}

// ─────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────

async function main() {
  if (process.platform !== 'win32') {
    warn(`Non-Windows platform (${process.platform}). Skipping sidecar fetch — MR Extractor Phase 1 targets Windows only.`);
    return;
  }

  await ensureDir(BINARIES_DIR);
  await ensureDir(CACHE_DIR);

  try {
    await fetchZip(VERSIONS.ffmpeg);
  } catch (e) {
    warn(`ffmpeg: ${e.message}`);
    throw e;
  }

  try {
    await fetchDirect(VERSIONS['yt-dlp'], VERSIONS['yt-dlp'].out);
  } catch (e) {
    warn(`yt-dlp: ${e.message}`);
    throw e;
  }

  try {
    await fetchPython(VERSIONS.python);
  } catch (e) {
    warn(`python: ${e.message}`);
    throw e;
  }

  log('all sidecars ready');
}

main().catch((err) => {
  warn(err.stack ?? err.message);
  process.exit(1);
});
