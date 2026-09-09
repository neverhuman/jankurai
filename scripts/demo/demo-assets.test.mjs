import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { PALETTE } from './gif-encode.mjs';
import { PRESETS, PUBLIC_DIR, MAX_BYTES, RECEIPT } from './demo-catalog.mjs';

const hub = path.resolve(fileURLToPath(new URL('../..', import.meta.url)));
const catalog = path.join(hub, PUBLIC_DIR);

test('README embeds the preview and links the committed 1080p GIF', () => {
  const readme = fs.readFileSync(path.join(hub, 'README.md'), 'utf8');
  const catalogReadme = fs.readFileSync(path.join(catalog, 'README.md'), 'utf8');
  assert.match(readme, /!\[Jankurai audit\]\(docs\/demo\/audit-readme\.gif\)/);
  assert.match(readme, /\[1920×1080 full-resolution GIF\]\(docs\/demo\/audit-1080p\.gif\)/);
  assert.match(catalogReadme, /audit-readme\.gif/);
  assert.match(catalogReadme, /audit-1080p\.gif/);
  assert.match(catalogReadme, /docs\/demo\/audit-1080p\.gif/);
});

test('committed catalog GIFs are exact-palette, sized, and under 50 MB', () => {
  const verify = spawnSync(process.execPath, [fileURLToPath(new URL('./verify-audit-gif.mjs', import.meta.url)), catalog], {
    encoding: 'utf8',
  });
  assert.equal(verify.status, 0, verify.stderr || verify.stdout);
  const receipt = JSON.parse(fs.readFileSync(path.join(catalog, RECEIPT), 'utf8'));
  assert.deepEqual(receipt.palette, PALETTE);
  PRESETS.forEach((preset, index) => {
    const output = receipt.outputs[index];
    const raw = fs.readFileSync(path.join(catalog, preset.name));
    assert.equal(output.name, preset.name);
    assert.equal(output.width, preset.width);
    assert.equal(output.height, preset.height);
    assert.ok(raw.length < MAX_BYTES, `${preset.name} must stay under 50 MB`);
    assert.equal(raw.subarray(0, 6).toString(), 'GIF89a');
  });
});
