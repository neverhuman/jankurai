import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { sha256 } from './audit-recording.mjs';
import { cleanupSample } from './sample-cleanup.mjs';

for (const mutation of ['none', 'edit', 'unknown-file', 'unknown-directory', 'missing', 'symlink']) {
  test(`owned sample cleanup: ${mutation}`, () => {
    const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-demo-cleanup-')));
    try {
      fs.writeFileSync(path.join(directory, 'source.rs'), 'original');
      const expected = [{ name: 'source.rs', sha256: sha256('original') }];
      if (mutation === 'edit') fs.writeFileSync(path.join(directory, 'source.rs'), 'changed');
      if (mutation === 'unknown-file') fs.writeFileSync(path.join(directory, 'unique.txt'), 'retain');
      if (mutation === 'unknown-directory') fs.mkdirSync(path.join(directory, 'unknown'));
      if (mutation === 'missing') fs.unlinkSync(path.join(directory, 'source.rs'));
      if (mutation === 'symlink') fs.symlinkSync('source.rs', path.join(directory, 'link'));
      if (mutation === 'none') { cleanupSample(directory, expected); assert.equal(fs.existsSync(directory), false); }
      else { assert.throws(() => cleanupSample(directory, expected), /preserving/); assert(fs.existsSync(directory)); }
    } finally {
      // These fixtures are authored by this test. Unlink their known symlink
      // explicitly before removing the automatically disposed fixture.
      const link = path.join(directory, 'link');
      if (fs.existsSync(link)) fs.unlinkSync(link);
      if (fs.existsSync(directory)) fs.rmSync(directory, { recursive: true });
    }
  });
}
