// Independent GIF89a decoder. This file must not import the encoder LZW table.
import { createHash } from 'node:crypto';
import { PALETTE } from './gif-encode.mjs';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function decodeLzw(minCode, data) {
  const clear = 1 << minCode, eoi = clear + 1;
  let codeSize = minCode + 1, next = eoi + 1, pos = 0, buf = 0, bits = 0;
  const table = new Map(), pixels = [];
  let previous = null;
  const read = () => {
    while (bits < codeSize) {
      if (pos >= data.length) throw new Error('truncated LZW stream');
      buf |= data[pos++] << bits;
      bits += 8;
    }
    const code = buf & ((1 << codeSize) - 1);
    buf >>= codeSize;
    bits -= codeSize;
    return code;
  };
  while (true) {
    const code = read();
    if (code === clear) {
      table.clear();
      codeSize = minCode + 1;
      next = eoi + 1;
      previous = null;
      continue;
    }
    if (code === eoi) return pixels;
    let entry;
    if (code < clear) entry = [code];
    else if (table.has(code)) entry = table.get(code);
    else if (code === next && previous) entry = previous.concat(previous[0]);
    else throw new Error('invalid LZW code');
    pixels.push(...entry);
    if (previous && next < 4096) {
      table.set(next, previous.concat(entry[0]));
      next += 1;
      if (next === (1 << codeSize) && codeSize < 12) codeSize += 1;
    }
    previous = entry;
  }
}

export function decodeGif(raw) {
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
    if (color[0] !== PALETTE[i][0] || color[1] !== PALETTE[i][1] || color[2] !== PALETTE[i][2]) {
      throw new Error(`GCT[${i}] remapped or dimmed`);
    }
    palette.push(color);
  }
  let i = 13 + gctCount * 3;
  if (raw[i] === 0x21 && raw[i + 1] === 0xFF) i += 19;
  const frames = [];
  while (raw[i] !== 0x3B) {
    if (raw[i] !== 0x21 || raw[i + 1] !== 0xF9) throw new Error(`unsupported gif block ${raw[i]}`);
    if (raw[i + 3] !== 0x04) throw new Error('graphic control must not fade or dispose');
    const delay = raw[i + 4] | (raw[i + 5] << 8);
    i += 8;
    if (raw[i] !== 0x2C) throw new Error('missing image descriptor');
    const fw = raw.readUInt16LE(i + 5), fh = raw.readUInt16LE(i + 7);
    if (raw[i + 9] & 0x80) throw new Error('local color table would remap the palette');
    i += 10;
    const minCode = raw[i++];
    const compressed = [];
    while (raw[i] !== 0) {
      const n = raw[i++];
      compressed.push(...raw.subarray(i, i + n));
      i += n;
    }
    i += 1;
    const indexes = decodeLzw(minCode, Buffer.from(compressed));
    if (indexes.length !== fw * fh) throw new Error('decoded frame size');
    const rgb = Buffer.alloc(indexes.length * 3);
    for (let p = 0; p < indexes.length; p++) {
      const color = palette[indexes[p]];
      if (!color) throw new Error('decoded index outside palette');
      rgb.set(color, p * 3);
    }
    frames.push({ width: fw, height: fh, delay, rgb, rgbSha256: sha256(rgb) });
  }
  return { width, height, palette, frames };
}
