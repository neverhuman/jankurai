#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function inspect(raw) {
  if (raw.subarray(0, 6).toString() !== 'GIF89a') throw new Error('not GIF89a');
  const width = raw.readUInt16LE(6), height = raw.readUInt16LE(8);
  let i = 13 + 48 + 19;
  const frames = [];
  while (raw[i] !== 0x3B) {
    if (raw[i] === 0x21 && raw[i + 1] === 0xF9) {
      const delay = raw[i + 4] | (raw[i + 5] << 8);
      i += 8;
      if (raw[i] !== 0x2C) throw new Error('missing image descriptor');
      const fw = raw.readUInt16LE(i + 5), fh = raw.readUInt16LE(i + 7);
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
  return { width, height, frames };
}

const directory = process.argv[2];
if (!directory) throw new Error('usage: verify-audit-gif.mjs RENDERED-DIR');
const manifest = JSON.parse(fs.readFileSync(path.join(directory, 'audit-demo.json'), 'utf8'));
for (const output of manifest.outputs) {
  const file = path.join(directory, output.name);
  const raw = fs.readFileSync(file);
  if (raw.length !== output.bytes || raw.length >= 50_000_000) throw new Error(`${output.name} size`);
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
