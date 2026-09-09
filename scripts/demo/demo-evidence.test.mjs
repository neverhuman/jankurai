import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { verifyEvidence } from './demo-evidence.mjs';
import { CAPTURE_FILES, RECEIPT } from './demo-catalog.mjs';

const catalog = fileURLToPath(new URL('../../docs/demo/', import.meta.url));
// These deliberately altered copies never replace the public audit recording.
function capturedFixture(run) {
  const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-capture-control-')));
  try {
    for (const name of [RECEIPT, ...Object.keys(CAPTURE_FILES)]) fs.copyFileSync(path.join(catalog, name), path.join(directory, name));
    run(directory);
  } finally { fs.rmSync(directory, { recursive: true, force: true }); }
}

test('the committed capture has a complete matching digest chain', () => capturedFixture(directory => {
  assert.equal(verifyEvidence(directory).kind, 'recorded-process');
}));

test('fresh GIF receipt cannot retain a modified raw report', () => capturedFixture(directory => {
  fs.appendFileSync(path.join(directory, 'report.json'), '\n');
  assert.throws(() => verifyEvidence(directory));
}));

test('stale authored inventory cannot accompany the current recording', () => capturedFixture(directory => {
  const file = path.join(directory, 'sample-inputs.json'), inputs = JSON.parse(fs.readFileSync(file));
  inputs.recordingSha256 = '0'.repeat(64); fs.writeFileSync(file, JSON.stringify(inputs));
  assert.throws(() => verifyEvidence(directory));
}));

test('missing producer state refuses the capture', () => capturedFixture(directory => {
  fs.unlinkSync(path.join(directory, 'producer-audit-state.json'));
  assert.throws(() => verifyEvidence(directory));
}));


function freshFixture(run) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-fresh-capture-control-')));
  const rendered = path.join(root, 'rendered'), recording = path.join(root, 'recording');
  fs.mkdirSync(rendered); fs.mkdirSync(recording);
  fs.copyFileSync(path.join(catalog, RECEIPT), path.join(rendered, RECEIPT));
  for (const [name, original] of Object.entries(CAPTURE_FILES)) fs.copyFileSync(path.join(catalog, name), path.join(recording, original));
  try { run({ root, rendered, recording }); }
  finally { fs.rmSync(root, { recursive: true, force: true }); }
}

test('fresh rendered/ resolves its exact matching recording/ sibling', () => freshFixture(({ rendered }) => {
  assert.equal(verifyEvidence(rendered).kind, 'recorded-process');
}));

test('partial flat capture cannot silently fall back to a fresh sibling', () => freshFixture(({ rendered }) => {
  fs.writeFileSync(path.join(rendered, 'stdout.txt'), '');
  assert.throws(() => verifyEvidence(rendered));
}));

test('arbitrary directory names cannot nominate a sibling recording', () => freshFixture(({ root, rendered }) => {
  const renamed = path.join(root, 'arbitrary'); fs.renameSync(rendered, renamed);
  assert.throws(() => verifyEvidence(renamed), /supported rendered/);
}));

test('symlinked fresh capture directory is refused', () => freshFixture(({ root, rendered, recording }) => {
  const retained = path.join(root, 'outside-recording'); fs.renameSync(recording, retained); fs.symlinkSync(retained, recording);
  assert.throws(() => verifyEvidence(rendered), /redirected evidence/);
  assert.ok(fs.existsSync(path.join(retained, 'recording.json')));
}));

test('symlinked fresh report is refused', () => freshFixture(({ root, rendered, recording }) => {
  const file = path.join(recording, 'report.json'), retained = path.join(root, 'outside-report.json');
  fs.renameSync(file, retained); fs.symlinkSync(retained, file);
  assert.throws(() => verifyEvidence(rendered)); assert.ok(fs.existsSync(retained));
}));
