#!/usr/bin/env node
// Normal CI: audit an explicitly authored sample, then verify its recorded GIF.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { sha256, validateRecording, outcome } from './audit-recording.mjs';
import { render } from './render-audit-gif.mjs';
import { cleanupSample } from './sample-cleanup.mjs';

const [auditor, destination] = process.argv.slice(2);
if (!auditor || !path.isAbsolute(auditor) || !destination) throw new Error('usage: generate-demo.mjs /absolute/qualified-auditor /new-output-directory');
const here = path.dirname(fileURLToPath(import.meta.url)), root = path.resolve(destination);
fs.mkdirSync(root);
const sample = path.join(root, 'sample-repository'); fs.mkdirSync(sample);
fs.mkdirSync(path.join(sample, 'src'));
const inventory = [];
const write = (name, content) => {
  fs.writeFileSync(path.join(sample, name), content, { flag: 'wx' });
  inventory.push({ name, sha256: sha256(content) });
};
write('README.md', '# Sample repository\n\nAn intentionally incomplete authored sample for a real Jankurai audit.\nThe displayed score and failures are measured, never substituted.\n');
write('Cargo.toml', '[package]\nname="audit-demo-sample"\nversion="0.1.0"\nedition="2021"\n');
write('src/lib.rs', 'pub fn sample() -> u64 { 42 }\n');
for (let index = 0; index < 2000; index++) {
  const functions = Array.from({ length: 32 }, (_, row) => `pub fn value_${index}_${row}(x: u64) -> u64 { x.wrapping_add(${index * 32 + row}) }`).join('\n') + '\n';
  write(`src/sample_${String(index).padStart(4, '0')}.rs`, functions);
}
const capture = path.join(root, 'recording');
try {
  const execution = spawnSync(process.execPath, [path.join(here, 'record-audit.mjs'), auditor, sample, capture], { stdio: 'inherit' });
  const recordingBytes = fs.readFileSync(path.join(capture, 'recording.json'));
  const recording = validateRecording(JSON.parse(recordingBytes));
  const result = outcome(recording);
  if (execution.signal || recording.result.signal || recording.result.error || !result.summary
      || ![0, 1].includes(recording.result.exitCode)
      || execution.status !== recording.result.exitCode
      || (recording.result.exitCode === 0) !== result.summary.policyPassed
      || result.passed || recording.result.exitCode !== 1) {
    throw new Error('sample audit did not complete with a consistent real policy outcome');
  }
  // This job verifies an honest demo; an explicitly displayed sample policy FAIL
  // is allowed. Missing execution/report and contradictory outcomes remain fatal.
  const rendered = path.join(root, 'rendered');
  render(recordingBytes, rendered);
  const verify = spawnSync(process.env.DEMO_PYTHON || 'python3', [path.join(here, 'verify-audit-gif.py'), rendered], { stdio: 'inherit' });
  if (verify.status !== 0) throw new Error('independent decoded pixel verification failed');
  fs.writeFileSync(path.join(capture, 'sample-inputs.json'), JSON.stringify({
    description: 'Real audit of an authored sample repository; policy failures are preserved.',
    inventory, sourceSha256: sha256(JSON.stringify(inventory)),
    recordingSha256: sha256(recordingBytes), expectedSampleOutcome: 'FAIL', measuredOutcome: result,
  }, null, 2) + '\n', { flag: 'wx' });
} finally {
  for (const extra of ['target', '.jankurai']) {
    const ephemeral = path.join(sample, extra);
    if (fs.existsSync(ephemeral) && !fs.lstatSync(ephemeral).isSymbolicLink()) {
      fs.rmSync(ephemeral, { recursive: true, force: true });
    }
  }
  cleanupSample(sample, inventory);
}
