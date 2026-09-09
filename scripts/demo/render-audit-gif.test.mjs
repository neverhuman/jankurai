import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const hub = path.resolve(here, '../..');

test('demo renderer writes bright lossless-palette gifs under the size cap', () => {
  const result = spawnSync(process.execPath, [path.join(here, 'render-audit-gif.mjs')], {
    encoding: 'utf8',
    cwd: hub,
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const receipt = JSON.parse(fs.readFileSync(path.join(hub, 'docs/demo/audit-demo.json'), 'utf8'));
  assert.equal(receipt.cinema.width, 1920);
  assert.equal(receipt.cinema.height, 1080);
  assert.ok(receipt.cinema.bytes < 50 * 1024 * 1024);
  assert.ok(receipt.readme.bytes > 100);
  const cinema = fs.readFileSync(path.join(hub, receipt.cinema.path));
  assert.equal(cinema.subarray(0, 6).toString(), 'GIF89a');
  const readme = fs.readFileSync(path.join(hub, receipt.readme.path));
  assert.equal(readme.subarray(0, 6).toString(), 'GIF89a');
});
