import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { sha256 } from './audit-recording.mjs';
import { captureProducerState, stateName } from './producer-state.mjs';
import { cleanupSample } from './sample-cleanup.mjs';

function fixture(run) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-state-test-')));
  const sample = path.join(root, 'sample'), capture = path.join(root, 'recording');
  fs.mkdirSync(path.join(sample, 'target/jankurai'), { recursive: true }); fs.mkdirSync(capture);
  fs.writeFileSync(path.join(sample, 'source.rs'), 'authored input');
  const inputs = [{ name: 'source.rs', sha256: sha256('authored input') }];
  const report = { auditor_version: '1.7.0', git: { head: null }, decision: { hard_findings: 8 }, caps_applied: ['cap-one'] };
  const recording = { identity: { executableSha256: 'a'.repeat(64) }, startedEpochMs: 100000,
    result: { report, reportSha256: 'b'.repeat(64), completedEpochMs: 104900 } };
  const state = { schema_version: '1.0.0', last_full_scan_at: 104, last_full_scan_commit: '',
    last_full_hard_findings: 8, last_full_caps: ['cap-one'], last_full_auditor_version: '1.7.0' };
  const file = path.join(sample, stateName);
  fs.writeFileSync(file, JSON.stringify(state, null, 2));
  const symlinks = [];
  try { run({ root, sample, capture, inputs, recording, state, file, symlinks }); }
  finally {
    for (const link of symlinks) { try { fs.unlinkSync(link); } catch (e) { if (e.code !== 'ENOENT') throw e; } }
    fs.rmSync(root, { recursive: true });
  }
}

test('real writer-shaped state is preserved outside inputs and permits exact inventory cleanup', () => fixture(f => {
  const original = fs.readFileSync(f.file);
  const output = captureProducerState(f.sample, f.recording, f.capture);
  assert.equal(output.name, stateName); assert.equal(output.sha256, sha256(original));
  assert.deepEqual(fs.readFileSync(path.join(f.capture, output.preservedName)), original);
  cleanupSample(f.sample, [...f.inputs, output]); assert.equal(fs.existsSync(f.sample), false);
  assert(fs.existsSync(path.join(f.capture, output.preservedName)));
}));

for (const [name, mutate] of [
  ['old schema', s => { s.schema_version = '0.9'; }],
  ['wrong tool version', s => { s.last_full_auditor_version = '9.9.9'; }],
  ['wrong commit', s => { s.last_full_scan_commit = 'deadbeef'; }],
  ['wrong hard count', s => { s.last_full_hard_findings = 0; }],
  ['wrong caps', s => { s.last_full_caps = []; }],
  ['unknown key', s => { s.unrecognized = 'retain'; }],
  ['stale', s => { s.last_full_scan_at = 99; }],
  ['future', s => { s.last_full_scan_at = 105; }],
]) test(`producer state rejects ${name}`, () => fixture(f => {
  mutate(f.state); fs.writeFileSync(f.file, JSON.stringify(f.state, null, 2));
  assert.throws(() => captureProducerState(f.sample, f.recording, f.capture));
  assert.throws(() => cleanupSample(f.sample, f.inputs)); assert(fs.existsSync(f.file));
}));

for (const [name, content] of [['malformed', '{'], ['oversized', ' '.repeat(65537)],
  ['duplicate key', '{"schema_version":"1.0.0","schema_version":"1.0.0","last_full_scan_at":104}']]) {
  test(`producer state rejects ${name}`, () => fixture(f => {
    fs.writeFileSync(f.file, content); assert.throws(() => captureProducerState(f.sample, f.recording, f.capture));
    assert(fs.existsSync(f.file));
  }));
}

test('producer state rejects file symlink and preserves its outside target', () => fixture(f => {
  const outside = path.join(f.root, 'outside.json'); fs.renameSync(f.file, outside);
  fs.symlinkSync(outside, f.file); f.symlinks.push(f.file);
  assert.throws(() => captureProducerState(f.sample, f.recording, f.capture)); assert(fs.existsSync(outside));
}));
test('producer state rejects symlinked ancestor', () => fixture(f => {
  const outside = path.join(f.root, 'outside'); fs.renameSync(path.join(f.sample, 'target'), outside);
  fs.symlinkSync(outside, path.join(f.sample, 'target')); f.symlinks.push(path.join(f.sample, 'target'));
  assert.throws(() => captureProducerState(f.sample, f.recording, f.capture));
}));
test('producer state rejects hard links', () => fixture(f => {
  fs.linkSync(f.file, path.join(f.root, 'outside.json'));
  assert.throws(() => captureProducerState(f.sample, f.recording, f.capture));
}));
test('producer state requires recorded execution window', () => fixture(f => {
  delete f.recording.startedEpochMs;
  assert.throws(() => captureProducerState(f.sample, f.recording, f.capture));
}));
test('producer state does not replace earlier preserved evidence', () => fixture(f => {
  captureProducerState(f.sample, f.recording, f.capture);
  assert.throws(() => captureProducerState(f.sample, f.recording, f.capture), /EEXIST/);
}));
for (const mutation of ['modified state', 'extra file', 'extra directory']) {
  test(`known producer output does not authorize cleanup of ${mutation}`, () => fixture(f => {
    const output = captureProducerState(f.sample, f.recording, f.capture);
    if (mutation === 'modified state') fs.appendFileSync(f.file, '\n');
    if (mutation === 'extra file') fs.writeFileSync(path.join(f.sample, 'target/unique.txt'), 'retain');
    if (mutation === 'extra directory') fs.mkdirSync(path.join(f.sample, 'target/unknown'));
    assert.throws(() => cleanupSample(f.sample, [...f.inputs, output])); assert(fs.existsSync(f.sample));
  }));
}
