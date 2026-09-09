import fs from 'node:fs';
import path from 'node:path';
import { sha256 } from './audit-recording.mjs';

export const stateName = 'target/jankurai/audit-state.json';

// Reviewed smart-state 1.0 writer contract: the full auditor saves this state
// even with --no-score-history. This is a declared output, not an authored input.
export function captureProducerState(sample, recording, captureDirectory) {
  const r = recording.result, report = r.report;
  const start = recording.startedEpochMs, end = r.completedEpochMs;
  if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start > end
      || !report || typeof report.auditor_version !== 'string'
      || !Array.isArray(report.caps_applied) || !report.caps_applied.every(cap => typeof cap === 'string')
      || !Number.isSafeInteger(report.decision?.hard_findings)
      || !(typeof report.git?.head === 'string' || report.git?.head === null)) {
    throw new Error('cannot bind producer state to execution/report identity');
  }
  for (const name of ['.', 'target', 'target/jankurai']) {
    const stat = fs.lstatSync(path.join(sample, name));
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('preserving redirected producer output directory');
  }
  const file = path.join(sample, stateName);
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  let raw;
  try {
    const before = fs.fstatSync(fd);
    if (!before.isFile() || before.nlink !== 1 || before.size < 1 || before.size > 65536) {
      throw new Error('producer state must be one bounded regular file');
    }
    const buffer = Buffer.alloc(before.size + 1);
    const count = fs.readSync(fd, buffer, 0, buffer.length, 0);
    const after = fs.fstatSync(fd), entry = fs.lstatSync(file);
    if (count !== before.size || after.size !== before.size || entry.isSymbolicLink()
        || entry.dev !== before.dev || entry.ino !== before.ino || after.mtimeMs !== before.mtimeMs) {
      throw new Error('producer state changed while captured');
    }
    raw = buffer.subarray(0, count);
  } finally { fs.closeSync(fd); }
  const state = JSON.parse(raw.toString('utf8'));
  if (!Number.isSafeInteger(state.last_full_scan_at)
      || state.last_full_scan_at < Math.floor(start / 1000)
      || state.last_full_scan_at > Math.floor(end / 1000)) throw new Error('producer state is stale or outside execution');
  const expected = {
    schema_version: '1.0.0',
    last_full_scan_at: state.last_full_scan_at,
    last_full_scan_commit: report.git.head ?? '',
    last_full_hard_findings: report.decision.hard_findings,
    last_full_caps: report.caps_applied,
    last_full_auditor_version: report.auditor_version,
  };
  const pretty = Buffer.from(JSON.stringify(expected, null, 2));
  if (!raw.equals(pretty) && !raw.equals(Buffer.concat([pretty, Buffer.from('\n')]))) {
    throw new Error('producer state does not match the supported writer/report contract');
  }
  const preservedName = 'producer-audit-state.json';
  fs.writeFileSync(path.join(captureDirectory, preservedName), raw, { flag: 'wx' });
  return { name: stateName, sha256: sha256(raw), bytes: raw.length,
    preservedName, contract: 'jankurai smart-state1.0.0',
    executableSha256: recording.identity.executableSha256,
    reportSha256: r.reportSha256, startedEpochMs: start, completedEpochMs: end };
}
