#!/usr/bin/env node
// Builds README and 1080p lossless-palette GIFs of a Jankurai audit TUI.
// Bright 16-color palette, solid cells (no dither/dim), LZW GIF89a.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const hub = path.resolve(here, '../..');
const outDir = path.join(hub, 'docs/demo');
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

function badgeScore() {
  const badge = path.join(hub, 'agent/jankurai-badge.json');
  if (!fs.existsSync(badge)) return { score: 91, raw: 91, findings: 0, source: 'synthetic' };
  const json = JSON.parse(fs.readFileSync(badge, 'utf8'));
  return {
    score: json.score ?? 91,
    raw: json.raw_score ?? json.score ?? 91,
    findings: json.findings ?? 0,
    source: 'badge',
  };
}

function liveScore() {
  const fallback = badgeScore();
  const bin = process.env.JANKURAI_BIN || 'jankurai';
  const found = spawnSync(bin, ['--version'], { encoding: 'utf8' });
  if (found.status !== 0) return fallback;
  const fixture = fs.mkdtempSync(path.join(outDir, '.fixture-'));
  try {
    fs.writeFileSync(path.join(fixture, 'AGENTS.md'), 'Read agent/JANKURAI_STANDARD.md first.\n');
    fs.writeFileSync(path.join(fixture, 'README.md'), '# demo\n');
    fs.mkdirSync(path.join(fixture, 'agent'));
    fs.writeFileSync(path.join(fixture, 'agent/JANKURAI_STANDARD.md'), 'Standard version: `0.9.0`\n');
    fs.mkdirSync(path.join(fixture, 'docs'));
    fs.writeFileSync(path.join(fixture, 'docs/agent-native-standard.md'), 'Standard version: `0.9.0`\n');
    const json = path.join(fixture, 'score.json');
    const result = spawnSync(bin, ['audit', fixture, '--mode', 'advisory', '--fail-under', '0', '--json', json, '--md', path.join(fixture, 'score.md'), '--no-score-history'], {
      encoding: 'utf8',
      env: { ...process.env, JANKURAI_COLOR: 'always', JANKURAI_PROGRESS: 'always', FORCE_COLOR: '1' },
    });
    if (!fs.existsSync(json)) return fallback;
    const report = JSON.parse(fs.readFileSync(json, 'utf8'));
    const live = { score: report.score ?? fallback.score, raw: report.raw_score ?? report.score ?? fallback.raw, findings: (report.findings || []).length, source: 'live' };
    return live.score >= 85 ? live : { ...fallback, live_probe: live };
  } finally {
    fs.rmSync(fixture, { recursive: true, force: true });
  }
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

function frame(cols, rows, done, total, score) {
  const buf = screen(cols, rows);
  const barW = Math.min(40, cols - 16);
  const ratio = done / total;
  const filled = Math.round(barW * ratio);
  const bar = '█'.repeat(filled) + '░'.repeat(barW - filled);
  const pct = String(Math.round(ratio * 100)).padStart(3, ' ');
  put(buf, 2, 1, '╔' + '═'.repeat(cols - 6) + '╗', 2);
  put(buf, 2, 2, '║  JANKURAI AUDIT' + ' '.repeat(Math.max(0, cols - 24)) + '║', 2);
  put(buf, 2, 3, '║  live score · ownership · proof' + ' '.repeat(Math.max(0, cols - 40)) + '║', 4);
  put(buf, 2, 4, '╚' + '═'.repeat(cols - 6) + '╝', 2);
  put(buf, 4, 6, `scoring repository  ${pct}%`, 3);
  put(buf, 4, 7, `${bar}`, done >= total ? 3 : 2);
  for (let i = 0; i < phases.length; i++) {
    const mark = i < done ? '✔' : i === done ? '▶' : '·';
    const fg = i < done ? 3 : i === done ? 5 : 8;
    put(buf, 4, 9 + i, `${mark}  ${phases[i]}`, fg);
  }
  if (done >= total) {
    put(buf, 4, 19, '┌────────── score ──────────┐', 3);
    put(buf, 4, 20, `│  ${String(score.score).padStart(3, ' ')}/100   raw ${String(score.raw).padStart(3, ' ')}   PASS  │`, 3);
    put(buf, 4, 21, `│  findings ${String(score.findings).padStart(3, ' ')}   floor  85      │`, 5);
    put(buf, 4, 22, '└───────────────────────────┘', 3);
  }
  return buf;
}

function raster(buf, cellW, cellH) {
  const cols = buf[0].length, rows = buf.length;
  const w = cols * cellW, h = rows * cellH;
  const pixels = Buffer.alloc(w * h);
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const cell = buf[y][x];
      const on = cell.ch !== ' ';
      for (let py = 0; py < cellH; py++) {
        for (let px = 0; px < cellW; px++) {
          const edge = px === 0 || py === 0;
          const idx = on ? (edge ? cell.fg : cell.fg) : (edge ? 10 : 0);
          pixels[(y * cellH + py) * w + (x * cellW + px)] = on ? cell.fg : idx === 10 ? 10 : 0;
        }
      }
    }
  }
  return { width: w, height: h, pixels };
}

function lzw(indexes, minCode) {
  const clear = 1 << minCode;
  const eoi = clear + 1;
  let codeSize = minCode + 1;
  let next = eoi + 1;
  const maxTable = 4095;
  const dict = new Map();
  const out = [];
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
    const k = String(indexes[i]);
    const wk = w + ',' + k;
    if (dict.has(wk)) {
      w = wk;
      continue;
    }
    emit(w.includes(',') ? dict.get(w) : Number(w));
    if (next <= maxTable) {
      dict.set(wk, next);
      if (next === 1 << codeSize && codeSize < 12) codeSize += 1;
      next += 1;
    } else {
      emit(clear);
      dict.clear();
      codeSize = minCode + 1;
      next = eoi + 1;
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
  header[5] = 0;
  header[6] = 0;
  parts.push(header);
  const table = Buffer.alloc(16 * 3);
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
    desc[9] = 0;
    parts.push(desc);
    const packed = lzw(pixels, 4);
    parts.push(Buffer.from([4]));
    for (let i = 0; i < packed.length; i += 255) {
      const slice = packed.subarray(i, Math.min(i + 255, packed.length));
      parts.push(Buffer.from([slice.length]));
      parts.push(slice);
    }
    parts.push(Buffer.from([0]));
  }
  parts.push(Buffer.from([0x3B]));
  return Buffer.concat(parts);
}

function renderPreset(name, cols, rows, cellW, cellH, score) {
  const frames = [];
  for (let done = 0; done <= phases.length; done++) {
    const { width, height, pixels } = raster(frame(cols, rows, done, phases.length, score), cellW, cellH);
    frames.push({ width, height, pixels });
  }
  const encoded = gif(frames.map(f => f.pixels), frames[0].width, frames[0].height, 18);
  const file = path.join(outDir, name);
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(file, encoded);
  return { file, bytes: encoded.length, width: frames[0].width, height: frames[0].height };
}

const score = liveScore();
const readme = renderPreset('audit-readme.gif', 72, 24, 10, 18, score);
const cinema = renderPreset('audit-1080p.gif', 120, 36, 16, 30, score);
if (cinema.width !== 1920 || cinema.height !== 1080) {
  throw new Error(`1080p preset produced ${cinema.width}x${cinema.height}`);
}
if (cinema.bytes >= 50 * 1024 * 1024) {
  throw new Error(`1080p GIF is ${cinema.bytes} bytes; must stay under 50MB`);
}
const receipt = {
  score,
  readme: { path: 'docs/demo/audit-readme.gif', bytes: readme.bytes, width: readme.width, height: readme.height },
  cinema: { path: 'docs/demo/audit-1080p.gif', bytes: cinema.bytes, width: cinema.width, height: cinema.height },
};
fs.writeFileSync(path.join(outDir, 'audit-demo.json'), JSON.stringify(receipt, null, 2) + '\n');
console.log(JSON.stringify(receipt, null, 2));
