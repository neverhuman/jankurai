import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash } from 'node:crypto';
import { gif, PALETTE } from './gif-encode.mjs';
import { decodeGif } from './gif-decode.mjs';

// Synthetic image-format controls, never an audit recording.
const width = 16, height = 16;
const indexes = Uint8Array.from({ length: width * height }, (_, i) => (Math.floor(i / width) * 3 + i % width) % 16);
const original = gif([{ pixels: indexes, delay: 7 }], width, height);
const descriptor = 80 + 8;
const expected = Buffer.from(Array.from(indexes).flatMap(index => PALETTE[index]));
const expectedHash = createHash('sha256').update(expected).digest('hex');

function rejects(mutate, pattern) {
  const raw = Buffer.from(original); mutate(raw); assert.throws(() => decodeGif(raw), pattern);
}

test('independent decoder matches known nonsymmetric pixels and timing', () => {
  const decoded = decodeGif(original);
  assert.deepEqual(decoded.frames[0].rgb, expected);
  assert.equal(decoded.frames[0].rgbSha256, expectedHash);
  assert.equal(decoded.frames[0].delay, 7);
});

test('hash-only verification keeps identical pixels without retaining RGB buffers', () => {
  const frame = decodeGif(original, { includePixels: false }).frames[0];
  assert.equal(frame.rgbSha256, expectedHash); assert.equal(frame.rgb, undefined);
});

for (const [name, offset] of [['left', 1], ['top', 3]]) {
  test(`unsupported ${name} offset is rejected`, () => rejects(raw => { raw[descriptor + offset] = 1; }, /offset/));
}
test('interlace is rejected instead of hashing the wrong displayed order', () => rejects(raw => { raw[descriptor + 9] = 0x40; }, /interlace/));

test('every truncated prefix terminates with an error', { timeout: 2000 }, () => {
  for (let length = 0; length < original.length; length++) assert.throws(() => decodeGif(original.subarray(0, length)), `prefix ${length}`);
});

test('unsupported LZW minimum code size is rejected', () => rejects(raw => { raw[descriptor + 10] = 255; }, /code size/));
test('oversized declared canvas is refused before decompression', () => rejects(raw => { raw.writeUInt16LE(65535, 6); raw.writeUInt16LE(65535, 8); }, /bounds/));
test('partial frame is refused before decompression', () => rejects(raw => { raw.writeUInt16LE(1, descriptor + 5); }, /partial frame/));
test('LZW expansion cannot exceed its declared frame', () => rejects(raw => {
  raw.writeUInt16LE(1, 6); raw.writeUInt16LE(1, 8); raw.writeUInt16LE(1, descriptor + 5); raw.writeUInt16LE(1, descriptor + 7);
}, /exceed frame bounds/));
test('unsupported application bytes are rejected', () => rejects(raw => { raw[65] ^= 1; }, /loop extension/));
test('trailing data after the GIF trailer is rejected', () => assert.throws(() => decodeGif(Buffer.concat([original, Buffer.from([0])])), /trailing GIF/));
