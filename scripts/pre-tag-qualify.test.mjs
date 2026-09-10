import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { SUCCESS_MARKER, qualifyPreTag } from './pre-tag-qualify.mjs';

test('pre-tag-qualify fails closed without evidence and rejects release.yml masquerade', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'pre-tag-qualify-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  assert.throws(() => qualifyPreTag(path.join(root, 'missing')), /missing/);
  fs.mkdirSync(path.join(root, 'empty'));
  assert.throws(() => qualifyPreTag(path.join(root, 'empty')), /missing release-services result/);
  const ok = path.join(root, 'ok');
  fs.mkdirSync(ok);
  fs.writeFileSync(path.join(ok, 'result.txt'), `${SUCCESS_MARKER}\n`);
  assert.deepEqual(qualifyPreTag(ok).ok, true);
  fs.writeFileSync(path.join(ok, 'identity.txt'),
    'cert-identity=https://github.com/neverhuman/jankurai/.github/workflows/release.yml@refs/heads/main\n');
  assert.throws(() => qualifyPreTag(ok), /refusing release\.yml identity/);
});
