#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { PALETTE } from './gif-encode.mjs';
import { PRESETS, MAX_BYTES, RECEIPT } from './demo-catalog.mjs';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function sameColor(actual, expected, label) {
  if (actual[0] !== expected[0] || actual[1] !== expected[1] || actual[2] !== expected[2]) {
    throw new Error(`${label} remapped or dimmed: got ${actual} expected ${expected}`);
  }
}

function inspect(raw) {
  if (raw.subarray(0, 6).toString() !== 'GIF89a') throw new Error('not GIF89a');
  const width = raw.readUInt16LE(6), height = raw.readUInt16LE(8);
  const packed = raw[10];
  if ((packed & 0x80) === 0) throw new Error('missing global color table');
  const gctCount = 1 << ((packed & 7) + 1);
  if (gctCount !== PALETTE.length) throw new Error(`palette size ${gctCount}`);
  const gct = raw.subarray(13, 13 + gctCount * 3);
  const palette = [];
  for (let i = 0; i < gctCount; i++) {
    const color = [gct[i * 3], gct[i * 3 + 1], gct[i * 3 + 2]];
    sameColor(color, PALETTE[i], `GCT[${i}]`);
    palette.push(color);
  }
  let i = 13 + gctCount * 3;
  if (raw[i] === 0x21 && raw[i + 1] === 0xFF) {
    i += 19;
  }
  const frames = [];
  while (raw[i] !== 0x3B) {
    if (raw[i] === 0x21 && raw[i + 1] === 0xF9) {
      if (raw[i + 3] !== 0x04) throw new Error('graphic control must not fade or dispose');
      const delay = raw[i + 4] | (raw[i + 5] << 8);
      i += 8;
      if (raw[i] !== 0x2C) throw new Error('missing image descriptor');
      const fw = raw.readUInt16LE(i + 5), fh = raw.readUInt16LE(i + 7);
      if (raw[i + 9] & 0x80) throw new Error('local color table would remap the palette');
      i += 11;
      while (raw[i] !== 0) {
        const n = raw[i++];
        i += n;
      }
      i += 1;
      frames.push({ width: fw, height: fh, delay });
      continue;
    }
    throw new Error(`unsupported gif block ${raw[i]}`);
  }
  return { width, height, frames, palette };
}

const directory = process.argv[2];
if (!directory) throw new Error('usage: verify-audit-gif.mjs RENDERED-DIR');
const manifest = JSON.parse(fs.readFileSync(path.join(directory, RECEIPT), 'utf8'));
if (!Array.isArray(manifest.palette) || manifest.palette.length !== PALETTE.length) throw new Error('receipt palette');
manifest.palette.forEach((color, index) => sameColor(color, PALETTE[index], `receipt[${index}]`));
if (manifest.outputs.length !== PRESETS.length) throw new Error('receipt must publish both GIF presets');
PRESETS.forEach((preset, index) => {
  const output = manifest.outputs[index];
  if (output.name !== preset.name || output.width !== preset.width || output.height !== preset.height) {
    throw new Error(`receipt preset ${preset.name}`);
  }
});
for (const output of manifest.outputs) {
  const file = path.join(directory, output.name);
  const raw = fs.readFileSync(file);
  if (raw.length !== output.bytes || raw.length >= MAX_BYTES) throw new Error(`${output.name} size`);
  if (sha256(raw) !== output.sha256) throw new Error(`${output.name} digest`);
  const decoded = inspect(raw);
  if (decoded.width !== output.width || decoded.height !== output.height) throw new Error(`${output.name} geometry`);
  if (decoded.frames.length !== output.frames.length) throw new Error(`${output.name} frame count`);
  decoded.frames.forEach((frame, index) => {
    const expected = output.frames[index];
    if (frame.width !== output.width || frame.height !== output.height) throw new Error(`${output.name}#${index} geometry`);
    if (frame.delay * 10 !== expected.durationMs) throw new Error(`${output.name}#${index} duration`);
  });
  console.log(`${output.name}: ${decoded.frames.length} frames, ${raw.length} bytes, ${output.width}x${output.height}`);
}
