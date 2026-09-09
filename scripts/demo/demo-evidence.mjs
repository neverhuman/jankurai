import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { sha256, validateRecording, outcome } from './audit-recording.mjs';
import { CAPTURE_FILES, MAX_BYTES } from './demo-catalog.mjs';

// Consistency of a presentation capture; this does not create trusted CI evidence.
export function verifyEvidence(directory) {
  directory = path.resolve(directory);
  const exists = file => { try { fs.lstatSync(file); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; } };
  const plainDirectory = target => {
    for (let current = path.resolve(target); ; current = path.dirname(current)) {
      const stat = fs.lstatSync(current);
      if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('refusing redirected evidence directory');
      if (current === path.dirname(current)) break;
    }
  };
  plainDirectory(directory);
  const flat = Object.keys(CAPTURE_FILES).some(name => exists(path.join(directory, name)));
  if (!flat && path.basename(directory) !== 'rendered') throw new Error('missing flat capture or supported rendered/ layout');
  const capture = flat ? directory : path.join(path.dirname(directory), 'recording');
  plainDirectory(capture);
  const read = name => {
    const file = CAPTURE_FILES[name] ? path.join(capture, flat ? name : CAPTURE_FILES[name]) : path.join(directory, name);
    const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
    try {
      const before = fs.fstatSync(fd);
      if (!before.isFile() || before.nlink !== 1 || before.size >= MAX_BYTES) throw new Error('refusing nonregular/linked/oversized evidence');
      const raw = fs.readFileSync(fd), after = fs.fstatSync(fd), entry = fs.lstatSync(file);
      if (raw.length !== before.size || after.mtimeMs !== before.mtimeMs || after.ctimeMs !== before.ctimeMs
          || entry.dev !== before.dev || entry.ino !== before.ino || entry.isSymbolicLink()) throw new Error('evidence changed while read');
      return raw;
    } finally { fs.closeSync(fd); }
  };
  const manifest = JSON.parse(read('audit-demo.json'));
  const raw = read('audit-recording.json');
  const recording = validateRecording(JSON.parse(raw));
  const inputs = JSON.parse(read('sample-inputs.json'));
  assert.equal(manifest.recordingSha256, sha256(raw));
  assert.equal(inputs.recordingSha256, sha256(raw));
  assert.equal(inputs.sourceSha256, sha256(JSON.stringify(inputs.inventory)));
  assert.equal(manifest.kind, 'recorded-process');
  assert.equal(manifest.measuredDurationMs, recording.result.atMs);
  assert.deepEqual(manifest.outcome, outcome(recording));
  const reportBytes = read('report.json');
  assert.equal(recording.result.reportSha256, sha256(reportBytes));
  assert.deepEqual(recording.result.report, JSON.parse(reportBytes));
  assert.equal(recording.stdoutSha256, sha256(read('stdout.txt')));
  assert.equal(recording.stderrSha256, sha256(read('stderr.txt')));
  assert.equal(inputs.producerOutputs.length, 1);
  const producer = inputs.producerOutputs[0], stateBytes = read('producer-audit-state.json');
  assert.equal(producer.name, 'target/jankurai/audit-state.json');
  assert.equal(producer.preservedName, 'producer-audit-state.json');
  assert.equal(producer.sha256, sha256(stateBytes));
  assert.equal(producer.bytes, stateBytes.length);
  assert.equal(producer.executableSha256, recording.identity.executableSha256);
  assert.equal(producer.reportSha256, recording.result.reportSha256);
  assert.equal(producer.startedEpochMs, recording.startedEpochMs);
  assert.equal(producer.completedEpochMs, recording.result.completedEpochMs);
  return recording;
}
