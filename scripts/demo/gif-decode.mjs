// Independent GIF89a decoder. This file must not import the encoder LZW table.
import { createHash } from 'node:crypto';
import { PALETTE } from './gif-encode.mjs';
import { MAX_BYTES } from './demo-catalog.mjs';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function decodeLzw(minCode, data, pixelLimit) {
  if (minCode !== 4) throw new Error('unsupported LZW code size');
  const clear = 1 << minCode, eoi = clear + 1;
  let codeSize = minCode + 1, next = eoi + 1, pos = 0, buf = 0, bits = 0;
  const table = new Map(), pixels = new Uint8Array(pixelLimit);
  let pixelCount = 0, first = true;
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
    if (first && code !== clear) throw new Error('missing initial LZW clear');
    first = false;
    if (code === clear) {
      table.clear();
      codeSize = minCode + 1;
      next = eoi + 1;
      previous = null;
      continue;
    }
    if (code === eoi) {
      if (pixelCount !== pixelLimit || pos !== data.length || buf !== 0) throw new Error('decoded size or trailing LZW data');
      return pixels;
    }
    let entry;
    if (code < clear) entry = [code];
    else if (table.has(code)) entry = table.get(code);
    else if (code === next && previous) entry = previous.concat(previous[0]);
    else throw new Error('invalid LZW code');
    if (pixelCount + entry.length > pixelLimit) throw new Error('decoded pixels exceed frame bounds');
    pixels.set(entry, pixelCount); pixelCount += entry.length;
    if (previous && next < 4096) {
      table.set(next, previous.concat(entry[0]));
      next += 1;
      if (next === (1 << codeSize) && codeSize < 12) codeSize += 1;
    }
    previous = entry;
  }
}

export function decodeGif(raw, { includePixels = true } = {}) {
  if (raw.length < 80 || raw.length >= MAX_BYTES || raw.subarray(0, 6).toString() !== 'GIF89a') throw new Error('not GIF89a');
  const width = raw.readUInt16LE(6), height = raw.readUInt16LE(8);
  if (width < 1 || height < 1 || width * height > 1920 * 1080) throw new Error('unsupported canvas bounds');
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
  if (raw[i] === 0x21 && raw[i + 1] === 0xFF) {
    if (raw.subarray(i, i + 19).toString('hex') !== '21ff0b4e45545343415045322e300301000000') throw new Error('unsupported loop extension');
    i += 19;
  }
  const frames = [];
  let retainedPixels = 0;
  while (i < raw.length && raw[i] !== 0x3B) {
    if (frames.length >= 512 || i + 19 >= raw.length) throw new Error('truncated or excessive frame data');
    if (raw[i] !== 0x21 || raw[i + 1] !== 0xF9) throw new Error(`unsupported gif block ${raw[i]}`);
    if (raw[i + 2] !== 4 || raw[i + 7] !== 0) throw new Error('invalid graphic control');
    if (raw[i + 3] !== 0x04) throw new Error('graphic control must not fade or dispose');
    const delay = raw[i + 4] | (raw[i + 5] << 8);
    i += 8;
    if (raw[i] !== 0x2C) throw new Error('missing image descriptor');
    const fw = raw.readUInt16LE(i + 5), fh = raw.readUInt16LE(i + 7);
    if (raw.readUInt16LE(i + 1) !== 0 || raw.readUInt16LE(i + 3) !== 0) throw new Error('unsupported frame offset');
    if (fw !== width || fh !== height) throw new Error('unsupported partial frame');
    if (raw[i + 9] !== 0) throw new Error('unsupported interlace/local table/image flags');
    i += 10;
    const minCode = raw[i++];
    const compressed = [];
    while (raw[i] !== 0) {
      if (i >= raw.length) throw new Error('truncated image data');
      const n = raw[i++];
      if (i + n >= raw.length) throw new Error('truncated image subblock');
      compressed.push(...raw.subarray(i, i + n));
      i += n;
    }
    i += 1;
    const indexes = decodeLzw(minCode, Buffer.from(compressed), fw * fh);
    if (indexes.length !== fw * fh) throw new Error('decoded frame size');
    const rgb = Buffer.alloc(indexes.length * 3);
    for (let p = 0; p < indexes.length; p++) {
      const color = palette[indexes[p]];
      if (!color) throw new Error('decoded index outside palette');
      rgb.set(color, p * 3);
    }
    retainedPixels += includePixels ? indexes.length : 0;
    if (retainedPixels > 100_000_000) throw new Error('retained pixel budget exceeded');
    frames.push({ width: fw, height: fh, delay, ...(includePixels ? { rgb } : {}), rgbSha256: sha256(rgb) });
  }
  if (i !== raw.length - 1 || raw[i] !== 0x3B) throw new Error('missing trailer or trailing GIF bytes');
  return { width, height, palette, frames };
}
