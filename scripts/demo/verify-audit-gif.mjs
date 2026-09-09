#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { PALETTE } from './gif-encode.mjs';
import { decodeGif } from './gif-decode.mjs';
import { verifyEvidence } from './demo-evidence.mjs';
import { PRESETS, MAX_BYTES, RECEIPT } from './demo-catalog.mjs';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function sameColor(actual, expected, label) {
  if (actual[0] !== expected[0] || actual[1] !== expected[1] || actual[2] !== expected[2]) {
    throw new Error(`${label} remapped or dimmed: got ${actual} expected ${expected}`);
  }
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
  const decoded = decodeGif(raw, { includePixels: false });
  if (decoded.width !== output.width || decoded.height !== output.height) throw new Error(`${output.name} geometry`);
  if (decoded.frames.length !== output.frames.length) throw new Error(`${output.name} frame count`);
  decoded.frames.forEach((frame, index) => {
    const expected = output.frames[index];
    if (frame.width !== output.width || frame.height !== output.height) throw new Error(`${output.name}#${index} geometry`);
    if (frame.delay * 10 !== expected.durationMs) throw new Error(`${output.name}#${index} duration`);
    if (frame.rgbSha256 !== expected.rgbSha256) throw new Error(`${output.name}#${index} pixels`);
  });
  console.log(`${output.name}: ${decoded.frames.length} decoded RGB frames, ${raw.length} bytes, ${output.width}x${output.height}`);
}

// Fresh generator output has its capture beside rendered/, while a published
// catalog carries the complete flat capture. Pixel-only checks are explicit.
if (!process.argv.includes('--pixels-only')) verifyEvidence(directory);
