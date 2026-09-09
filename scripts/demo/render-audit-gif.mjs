#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { PALETTE, gif } from './gif-encode.mjs';
import { sha256, validateRecording, outcome } from './audit-recording.mjs';

const fontBytes = fs.readFileSync(new URL('./audit-mono.json', import.meta.url));
const FONT = JSON.parse(fontBytes);
const ascii = text => String(text).replace(/[^\x20-\x7e]/g, '?');

export function raster(recording, state, scale) {
  const width = 960 * scale, height = 540 * scale, pixels = Buffer.alloc(width * height);
  const rect = (x, y, w, h, color) => {
    for (let row = y * scale; row < (y + h) * scale && row < height; row++) {
      if (row < 0) continue;
      pixels.fill(color, row * width + Math.max(0, x * scale), row * width + Math.min(width, (x + w) * scale));
    }
  };
  const text = (x, y, value, color = 1, size = 1) => {
    const face = FONT.faces[String(scale * size)];
    for (const ch of ascii(value)) {
      if (x + FONT.width * size > 930) break;
      const glyph = face.glyphs[ch];
      for (let row = 0; row < face.height; row++) for (let col = 0; col < face.width; col++) {
        // The widest glyph is40px, so avoid32-bit bitwise truncation.
        if (Math.floor(glyph[row] / (2 ** col)) % 2) pixels[(y * scale + row) * width + x * scale + col] = color;
      }
      x += FONT.width * size;
    }
  };
  rect(24, 24, 912, 2, 2); rect(24, 514, 912, 2, 2);
  text(48, 45, 'JANKURAI / AUDIT', 2, 2);
  const synthetic = recording.kind === 'synthetic-test';
  text(48, 96, synthetic ? 'SYNTHETIC TEST - NOT AN AUDIT RECORDING' : 'RECORDED AUDIT / OBSERVED PHASES', synthetic ? 7 : 4);
  text(48, 130, '$ jankurai audit ' + path.basename(recording.identity.repository || 'repository') + ' --full --mode standard', 1);
  rect(48, 164, 864, 1, 8);
  const last = state.events.at(-1);
  const result = outcome(recording);
  const color = state.finished ? (result.passed ? 3 : 7) : 2;
  const activity = ['|', '/', '-', '\\'][Math.floor(state.atMs / 100) % 4];
  text(48, 182, state.finished ? 'AUDIT PROCESS FINISHED' : `[${activity}] ${last.label.toUpperCase()}`, color);
  text(738, 182, (state.atMs / 1000).toFixed(2) + 's elapsed', 5);
  rect(48, 217, 864, 8, 10); rect(48, 217, Math.round(864 * last.position / 8), 8, color);
  const rows = state.events.slice(-6);
  rows.forEach((event, i) => text(48, 243 + i * 27,
    `${(event.atMs / 1000).toFixed(2).padStart(7)}s  ${String(event.position).padStart(1)}/8  ${event.label}`,
    i === rows.length - 1 ? 5 : 11));
  rect(48, 416, 864, 1, 8);
  if (state.finished) {
    const s = result.summary;
    text(48, 433, (result.passed ? 'PASS' : 'FAIL') + (s ? `  ${s.score}/100` : '  NO REPORT'), color, 2);
    if (s) text(520, 438, `raw ${s.raw} / findings ${s.findings} / floor ${s.floor}`, 5);
    else text(520, 438, 'No valid result. No fallback.', 7);
    text(520, 466, `process exit: ${recording.result.exitCode ?? recording.result.signal ?? 'error'}`, 1);
  } else {
    text(48, 433, 'AUDIT RUNNING', 2, 2);
    text(520, 438, 'Score appears after process exit.', 1);
  }
  text(48, 491, '1x timeline / GIF 10ms ticks / final hold 2s / exact palette', 1);
  return { pixels, width, height };
}

export function timeline(recording) {
  const end = Math.round(recording.result.atMs / 10);
  //10fps for normal demos; at most300 clock updates for long real recordings.
  const interval = Math.max(10, Math.ceil(end / 300));
  const ticks = new Set([0, end, ...recording.events.map(e => Math.round(e.atMs / 10))]);
  for (let tick = 0; tick < end; tick += interval) ticks.add(tick);
  const ordered = [...ticks].sort((a, b) => a - b);
  return ordered.map((tick, i) => ({
    events: recording.events.filter(e => Math.round(e.atMs / 10) <= tick),
    atMs: tick === end ? recording.result.atMs : tick * 10,
    finished: tick === end,
    delay: i + 1 < ordered.length ? ordered[i + 1] - tick : 200,
  }));
}

export function render(recordingBytes, outDir, options = {}) {
  const recording = validateRecording(JSON.parse(recordingBytes), options);
  fs.mkdirSync(outDir); // Never silently replace prior artifacts.
  const states = timeline(recording), outputs = [];
  for (const [name, scale] of [['audit-readme.gif', 1], ['audit-1080p.gif', 2]]) {
    const expectedFrames = [], width = 960 * scale, height = 540 * scale;
    function* frames() {
      for (const state of states) {
        const frame = { ...raster(recording, state, scale), delay: state.delay };
        const rgb = Buffer.alloc(frame.pixels.length * 3);
        for (let i = 0; i < frame.pixels.length; i++) rgb.set(PALETTE[frame.pixels[i]], i * 3);
        expectedFrames.push({ durationMs: frame.delay * 10, rgbSha256: sha256(rgb) });
        yield frame;
      }
    }
    const encoded = gif(frames(), width, height);
    if (encoded.length >= 50_000_000) throw new Error('GIF must be smaller than 50 MB');
    fs.writeFileSync(path.join(outDir, name), encoded, { flag: 'wx' });
    outputs.push({ name, width, height, bytes: encoded.length, sha256: sha256(encoded), frames: expectedFrames });
  }
  const manifest = { schema: 1, kind: recording.kind, recordingSha256: sha256(recordingBytes),
    fontSha256: sha256(fontBytes), palette: PALETTE, timing: '1x observed timeline, rounded to 10ms; final hold 2000ms',
    clockFrameIntervalMs: Math.max(10, Math.ceil(Math.round(recording.result.atMs / 10) / 300)) * 10,
    measuredDurationMs: recording.result.atMs, outcome: outcome(recording), outputs };
  fs.writeFileSync(path.join(outDir, 'audit-demo.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [input, output, option] = process.argv.slice(2);
  if (!input || !output) throw new Error('usage: render-audit-gif.mjs recording.json /new-output-directory [--allow-synthetic-test]');
  render(fs.readFileSync(input), output, { allowSynthetic: option === '--allow-synthetic-test' });
}
