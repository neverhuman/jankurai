export const PALETTE = [[11, 18, 36], [248, 250, 255], [0, 229, 255], [61, 255, 110], [255, 61, 154], [255, 209, 102], [255, 122, 24], [255, 51, 85], [77, 140, 255], [255, 255, 255], [20, 40, 72], [0, 255, 184], [255, 240, 31], [190, 80, 255], [40, 80, 140], [255, 90, 200]];
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

export function gif(frames, width, height) {
  const parts = [Buffer.from('GIF89a')];
  let encodedBytes = 0;
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
  for (const frame of frames) {
    const { pixels, delay } = frame;
    parts.push(Buffer.from([0x21, 0xF9, 0x04, 0x04, delay & 255, (delay >> 8) & 255, 0x00, 0x00]));
    const desc = Buffer.alloc(10);
    desc[0] = 0x2C;
    desc.writeUInt16LE(width, 5);
    desc.writeUInt16LE(height, 7);
    desc[9] = 0;
    parts.push(desc);
    const packed = lzw(pixels, 4);
    encodedBytes += packed.length + Math.ceil(packed.length / 255) + 32;
    if (encodedBytes >= 50_000_000) throw new Error('GIF must be smaller than50 MB');
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
