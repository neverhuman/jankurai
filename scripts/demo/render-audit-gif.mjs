#!/usr/bin/env node
// Live-audit GIF producer. Requires `jankurai` on PATH or JANKURAI_BIN.
// Glyphs come from the committed Liberation Mono atlas. No badge/success fallback.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const hub = path.resolve(here, '../..');
const outDir = path.join(hub, 'docs/demo');
const atlas = JSON.parse(fs.readFileSync(path.join(here, 'font-atlas.json'), 'utf8'));
const phases = [
  'resolve changed paths',
  'load audit mode',
  'scan repository',
  'apply score policy',
  'apply mode and baseline',
  'render artifacts',
  'write JSON and Markdown',
  'score ready',
];

const PALETTE = [
  [11, 18, 36],
  [248, 250, 255],
  [0, 229, 255],
  [61, 255, 110],
  [255, 61, 154],
  [255, 209, 102],
  [255, 122, 24],
  [255, 51, 85],
  [77, 140, 255],
  [255, 255, 255],
  [20, 40, 72],
  [0, 255, 184],
  [255, 240, 31],
  [190, 80, 255],
  [40, 80, 140],
  [255, 90, 200],
];

function liveReport() {
  const bin = process.env.JANKURAI_BIN || 'jankurai';
  const found = spawnSync(bin, ['--version'], { encoding: 'utf8' });
  if (found.status !== 0) throw new Error('jankurai is required to render the demo GIF; set JANKURAI_BIN');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-demo-'));
  const json = path.join(tmp, 'repo-score.json');
  const md = path.join(tmp, 'repo-score.md');
  fs.mkdirSync(outDir, { recursive: true });
  const started = Date.now();
  const result = spawnSync(bin, [
    'audit', hub, '--full', '--mode', 'advisory', '--no-score-history',
    '--json', json, '--md', md,
  ], {
    encoding: 'utf8',
    env: { ...process.env, JANKURAI_COLOR: 'always', JANKURAI_PROGRESS: 'always', FORCE_COLOR: '1' },
  });
  if (!fs.existsSync(json)) {
    throw new Error(`audit wrote no report\n${result.stderr || result.stdout}`);
  }
  const report = JSON.parse(fs.readFileSync(json, 'utf8'));
  if (typeof report.score !== 'number') throw new Error('report.score is not a number');
  const passed = report.decision?.passed === true && report.score >= 85 && (report.decision?.hard_findings ?? 1) === 0;
  return {
    score: report.score,
    raw: typeof report.raw_score === 'number' ? report.raw_score : report.score,
    findings: Array.isArray(report.findings) ? report.findings.length : 0,
    passed,
    elapsed_ms: Date.now() - started,
    version: (found.stdout || '').trim(),
    source: 'live',
  };
}

function screen(cols, rows) {
  return Array.from({ length: rows }, () => Array.from({ length: cols }, () => ({ ch: ' ', fg: 1, bg: 0 })));
}

function put(buf, x, y, text, fg, bg = 0) {
  if (y < 0 || y >= buf.length) return;
  for (let i = 0; i < text.length; i++) {
    const cx = x + i;
    if (cx < 0 || cx >= buf[0].length) continue;
    buf[y][cx] = { ch: text[i], fg, bg };
  }
}

function frame(cols, rows, done, total, report) {
  const buf = screen(cols, rows);
  const barW = Math.min(40, cols - 16);
  const ratio = done / total;
  const filled = Math.round(barW * ratio);
  const bar = '#'.repeat(filled) + '-'.repeat(barW - filled);
  const pct = String(Math.round(ratio * 100)).padStart(3, ' ');
  put(buf, 2, 1, '+' + '='.repeat(cols - 6) + '+', 2);
  put(buf, 2, 2, '|  JANKURAI AUDIT' + ' '.repeat(Math.max(0, cols - 24)) + '|', 2);
  put(buf, 2, 3, '|  live score / ownership / proof' + ' '.repeat(Math.max(0, cols - 40)) + '|', 4);
  put(buf, 2, 4, '+' + '='.repeat(cols - 6) + '+', 2);
  put(buf, 4, 6, `scoring repository  ${pct}%`, 3);
  put(buf, 4, 7, `[${bar}]`, done >= total ? 3 : 2);
  for (let i = 0; i < phases.length; i++) {
    const mark = i < done ? '+' : i === done ? '>' : '.';
    const fg = i < done ? 3 : i === done ? 5 : 8;
    put(buf, 4, 9 + i, `${mark}  ${phases[i]}`, fg);
  }
  if (done >= total) {
    const mark = report.passed ? 'PASS' : 'FAIL';
    const style = report.passed ? 3 : 7;
    put(buf, 4, 19, '+---------- score ----------+', style);
    put(buf, 4, 20, `|  ${String(report.score).padStart(3, ' ')}/100   raw ${String(report.raw).padStart(3, ' ')}   ${mark}  |`, style);
    put(buf, 4, 21, `|  findings ${String(report.findings).padStart(3, ' ')}   floor  85      |`, 5);
    put(buf, 4, 22, '+---------------------------+', style);
  }
  return buf;
}

function raster(buf) {
  const cw = atlas.width, ch = atlas.height;
  const cols = buf[0].length, rows = buf.length;
  const width = cols * cw, height = rows * ch;
  const pixels = Buffer.alloc(width * height);
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const cell = buf[y][x];
      const glyph = atlas.glyphs[cell.ch] || atlas.glyphs[' '];
      for (let py = 0; py < ch; py++) {
        const row = glyph[py] || 0;
        for (let px = 0; px < cw; px++) {
          const on = ((row >> px) & 1) === 1;
          pixels[(y * ch + py) * width + (x * cw + px)] = on ? cell.fg : 0;
        }
      }
    }
  }
  return { width, height, pixels };
}

function lzw(indexes, minCode) {
  const clear = 1 << minCode, eoi = clear + 1;
  let codeSize = minCode + 1, next = eoi + 1;
  const dict = new Map(), out = [];
  let buf = 0, bits = 0;
  const emit = code => {
    buf |= code << bits;
    bits += codeSize;
    while (bits >= 8) {
      out.push(buf & 255);
      buf >>= 8;
      bits -= 8;
    }
  };
  emit(clear);
  let w = String(indexes[0]);
  for (let i = 1; i < indexes.length; i++) {
    const k = String(indexes[i]), wk = `${w},${k}`;
    if (dict.has(wk)) { w = wk; continue; }
    emit(w.includes(',') ? dict.get(w) : Number(w));
    if (next <= 4095) {
      dict.set(wk, next);
      if (next === 1 << codeSize && codeSize < 12) codeSize += 1;
      next += 1;
    } else {
      emit(clear); dict.clear(); codeSize = minCode + 1; next = eoi + 1;
    }
    w = k;
  }
  emit(w.includes(',') ? dict.get(w) : Number(w));
  emit(eoi);
  if (bits) out.push(buf & 255);
  return Buffer.from(out);
}

function gif(frames, width, height, delay) {
  const parts = [Buffer.from('GIF89a')];
  const header = Buffer.alloc(7);
  header.writeUInt16LE(width, 0);
  header.writeUInt16LE(height, 2);
  header[4] = 0xF0 | 3;
  parts.push(header);
  const table = Buffer.alloc(48);
  for (let i = 0; i < 16; i++) table.set(PALETTE[i], i * 3);
  parts.push(table);
  parts.push(Buffer.from([0x21, 0xFF, 0x0B]));
  parts.push(Buffer.from('NETSCAPE2.0'));
  parts.push(Buffer.from([0x03, 0x01, 0x00, 0x00, 0x00]));
  for (const pixels of frames) {
    parts.push(Buffer.from([0x21, 0xF9, 0x04, 0x04, delay & 255, (delay >> 8) & 255, 0x00, 0x00]));
    const desc = Buffer.alloc(10);
    desc[0] = 0x2C;
    desc.writeUInt16LE(width, 5);
    desc.writeUInt16LE(height, 7);
    parts.push(desc);
    const packed = lzw(pixels, 4);
    parts.push(Buffer.from([4]));
    for (let i = 0; i < packed.length; i += 255) {
      const slice = packed.subarray(i, Math.min(i + 255, packed.length));
      parts.push(Buffer.from([slice.length]), slice);
    }
    parts.push(Buffer.from([0]));
  }
  parts.push(Buffer.from([0x3B]));
  return Buffer.concat(parts);
}

function scale(pixels, srcW, srcH, destW, destH) {
  const out = Buffer.alloc(destW * destH);
  for (let y = 0; y < destH; y++) {
    const sy = Math.min(srcH - 1, Math.floor(y * srcH / destH));
    for (let x = 0; x < destW; x++) {
      const sx = Math.min(srcW - 1, Math.floor(x * srcW / destW));
      out[y * destW + x] = pixels[sy * srcW + sx];
    }
  }
  return out;
}

function renderPreset(name, destW, destH, report) {
  const frames = [];
  for (let done = 0; done <= phases.length; done++) {
    const rastered = raster(frame(72, 24, done, phases.length, report));
    frames.push(scale(rastered.pixels, rastered.width, rastered.height, destW, destH));
  }
  const encoded = gif(frames, destW, destH, 18);
  const file = path.join(outDir, name);
  fs.writeFileSync(file, encoded);
  return { file, bytes: encoded.length, width: destW, height: destH };
}

const report = liveReport();
const readme = renderPreset('audit-readme.gif', 960, 540, report);
const cinema = renderPreset('audit-1080p.gif', 1920, 1080, report);
if (cinema.bytes >= 50 * 1024 * 1024) throw new Error(`1080p GIF is ${cinema.bytes} bytes`);
const receipt = { report, readme: { path: 'docs/demo/audit-readme.gif', ...readme }, cinema: { path: 'docs/demo/audit-1080p.gif', ...cinema } };
delete receipt.readme.file;
delete receipt.cinema.file;
fs.writeFileSync(path.join(outDir, 'audit-demo.json'), JSON.stringify(receipt, null, 2) + '\n');
console.log(JSON.stringify(receipt, null, 2));
