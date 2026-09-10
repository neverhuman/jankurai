import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { temporaryCI } from './family-lib.mjs';
import { preserveCandidateDiagnostics } from './family-diagnostics.mjs';

function fixture(t) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'family-diagnostics-')));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}

test('separate failed runs retain exact bytes and hashes without overwriting prior diagnostics', t => {
  const root = fixture(t), parent = path.join(root, 'diagnostics');
  const saved = [];
  for (const bytes of [Buffer.from('{"decision":{"passed":false}}\n'), Buffer.from('{truncated')]) {
    const source = fs.mkdtempSync(path.join(root, 'source-'));
    fs.mkdirSync(path.join(source, 'jankurai/.jankurai'), { recursive: true });
    fs.writeFileSync(path.join(source, 'jankurai/.jankurai/repo-score.json'), bytes);
    const directory = preserveCandidateDiagnostics(source, parent, new Error('audit failed'));
    saved.push(directory);
    const manifest = JSON.parse(fs.readFileSync(path.join(directory, 'manifest.json')));
    const report = manifest.files.find(file => file.source === 'jankurai/.jankurai/repo-score.json');
    assert.equal(report.sha256, createHash('sha256').update(bytes).digest('hex'));
    assert.deepEqual(fs.readFileSync(path.join(directory, report.file)), bytes);
  }
  assert.notEqual(saved[0], saved[1]);
  assert.equal(fs.readdirSync(parent).length, 2);
});

test('redirected candidate report prevents cleanup and never copies an external sentinel', t => {
  const root = fixture(t), sentinel = path.join(root, 'sentinel');
  fs.writeFileSync(sentinel, 'external user bytes');
  let candidate;
  assert.throws(() => temporaryCI(root, 'source-', directory => {
    candidate = directory;
    fs.mkdirSync(path.join(directory, 'jankurai/.jankurai'), { recursive: true });
    fs.symlinkSync(sentinel, path.join(directory, 'jankurai/.jankurai/repo-score.json'));
    throw new Error('candidate failed');
  }, { onFailure: (directory, error) => preserveCandidateDiagnostics(directory, path.join(root, 'diagnostics'), error) }), /source retained/);
  assert.equal(fs.existsSync(candidate), true);
  assert.equal(fs.readFileSync(sentinel, 'utf8'), 'external user bytes');
  const destination = path.join(root, 'diagnostics');
  for (const entry of fs.readdirSync(destination)) assert.deepEqual(fs.readdirSync(path.join(destination, entry)), []);
});

test('diagnostic destination failure retains the complete candidate and original error', t => {
  const root = fixture(t), destination = path.join(root, 'diagnostics');
  fs.writeFileSync(destination, 'existing user file');
  let candidate;
  assert.throws(() => temporaryCI(root, 'source-', directory => {
    candidate = directory;
    fs.writeFileSync(path.join(directory, 'candidate.lock'), 'selected candidate');
    throw new Error('original validation failure');
  }, { onFailure: (directory, error) => preserveCandidateDiagnostics(directory, destination, error) }), error => {
    assert.equal(error.errors[0].message, 'original validation failure');
    return /source retained/.test(error.message);
  });
  assert.equal(fs.readFileSync(path.join(candidate, 'candidate.lock'), 'utf8'), 'selected candidate');
  assert.equal(fs.readFileSync(destination, 'utf8'), 'existing user file');
});
