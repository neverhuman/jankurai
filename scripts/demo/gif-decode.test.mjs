import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { decodeGif } from './gif-decode.mjs';
import { PALETTE } from './gif-encode.mjs';
import { PUBLIC_DIR, PUBLIC_FILES, RECEIPT } from './demo-catalog.mjs';

const hub = path.resolve(fileURLToPath(new URL('../..', import.meta.url)));
const catalog = path.join(hub, PUBLIC_DIR);
const verify = fileURLToPath(new URL('./verify-audit-gif.mjs', import.meta.url));

test('independent decoder matches committed RGB frame hashes', () => {
  const receipt = JSON.parse(fs.readFileSync(path.join(catalog, RECEIPT), 'utf8'));
  for (const output of receipt.outputs) {
    const decoded = decodeGif(fs.readFileSync(path.join(catalog, output.name)));
    decoded.frames.forEach((frame, index) => {
      assert.equal(frame.rgbSha256, output.frames[index].rgbSha256, `${output.name}#${index}`);
      assert.equal(frame.delay * 10, output.frames[index].durationMs);
    });
  }
});

function mutateAndVerify(mutate) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-gif-neg-'));
  try {
    for (const name of PUBLIC_FILES) {
      fs.copyFileSync(path.join(catalog, name), path.join(dir, name));
    }
    mutate(dir);
    const result = spawnSync(process.execPath, [verify, dir], { encoding: 'utf8', timeout: 60000 });
    assert.equal(result.signal, null); assert.equal(result.error, undefined);
    assert.notEqual(result.status, 0, result.stdout + result.stderr);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

test('wrong expected pixel hash fails closed', () => {
  mutateAndVerify(dir => {
    const receipt = JSON.parse(fs.readFileSync(path.join(dir, RECEIPT), 'utf8'));
    receipt.outputs[0].frames[0].rgbSha256 = '0'.repeat(64);
    fs.writeFileSync(path.join(dir, RECEIPT), JSON.stringify(receipt, null, 2));
  });
});

test('wrong palette fails closed', () => {
  mutateAndVerify(dir => {
    const raw = Buffer.from(fs.readFileSync(path.join(dir, 'audit-readme.gif')));
    raw[13] = (PALETTE[0][0] + 40) & 255;
    fs.writeFileSync(path.join(dir, 'audit-readme.gif'), raw);
    const receipt = JSON.parse(fs.readFileSync(path.join(dir, RECEIPT), 'utf8'));
    receipt.outputs[0].sha256 = createHash('sha256').update(raw).digest('hex');
    receipt.outputs[0].bytes = raw.length;
    fs.writeFileSync(path.join(dir, RECEIPT), JSON.stringify(receipt, null, 2));
  });
});

test('corrupt LZW bytes fail closed', () => {
  mutateAndVerify(dir => {
    const raw = Buffer.from(fs.readFileSync(path.join(dir, 'audit-readme.gif')));
    // Corrupt only compressed LZW bytes, preserving all GIF headers.
    raw[100] = 0xff; raw[101] = 0xff;
    fs.writeFileSync(path.join(dir, 'audit-readme.gif'), raw);
    const receipt = JSON.parse(fs.readFileSync(path.join(dir, RECEIPT), 'utf8'));
    receipt.outputs[0].sha256 = createHash('sha256').update(raw).digest('hex');
    receipt.outputs[0].bytes = raw.length;
    fs.writeFileSync(path.join(dir, RECEIPT), JSON.stringify(receipt, null, 2));
  });
});
