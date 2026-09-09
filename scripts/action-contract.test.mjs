import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { enforceReport, parseFloor, parseReport } from './action-gate.mjs';

const script = fileURLToPath(new URL('./run-action-audit.sh', import.meta.url));
const fingerprint = 'sha256:' + '0'.repeat(64);
function report(score = 85, mode = 'standard') {
  return { score, raw_score: score, scope: { mode: 'full' }, decision: {
    status: mode === 'advisory' ? 'advisory' : 'pass', minimum_score: 85,
    passed: true, hard_findings: 0, soft_findings: 0,
  }};
}
function ratchet(score) {
  return { baseline_score: score, score_delta: 0, allowed_drop: 0, passed: true,
    baseline_report_fingerprint: fingerprint, baseline_input_fingerprint: fingerprint,
    baseline_policy_fingerprint: fingerprint, new_caps: [], new_hard_findings: [], policy_changed: false };
}
const cases = [
  ['score84 fails', report(84), '85', 'standard', false],
  ['score85 passes', report(85), '85', 'standard', true],
  ['score89 fails floor90', report(89), '90', 'standard', false],
  ['score90 passes floor90', report(90), '90', 'standard', true],
  ['higher policy remains', { ...report(89), decision: { ...report(89).decision, minimum_score: 90 } }, '85', 'standard', false],
  ['zero retains policy', report(84), '0', 'standard', false],
  ['contradictory policy90 fails', { ...report(89), policy: { minimum_score: 90 } }, '85', 'standard', false],
  ['consistent policy85 passes', { ...report(89), policy: { minimum_score: 85 } }, '85', 'standard', true],
  ['invalid policy floor fails', { ...report(99), policy: { minimum_score: '85' } }, '85', 'standard', false],
  ['null present policy fails', { ...report(99), policy: null }, '85', 'standard', false],
  ['string score fails', { ...report(), score: '99' }, '85', 'standard', false],
  ['out of range fails', report(200), '85', 'standard', false],
  ['missing decision fails', { score: 99, raw_score: 99, scope: { mode: 'full' } }, '85', 'standard', false],
  ['hard failure fails', { ...report(), decision: { ...report().decision, hard_findings: 1 } }, '85', 'standard', false],
  ['null hard count fails', { ...report(), decision: { ...report().decision, hard_findings: null } }, '85', 'standard', false],
  ['false hard count fails', { ...report(), decision: { ...report().decision, hard_findings: false } }, '85', 'standard', false],
  ['failed decision fails', { ...report(), decision: { ...report().decision, passed: false } }, '85', 'standard', false],
  ['contradictory status fails', { ...report(), decision: { ...report().decision, status: 'fail' } }, '85', 'standard', false],
  ['changed scope fails', { ...report(), scope: { mode: 'changed' } }, '85', 'standard', false],
  ['advisory cannot hide policy failure', { ...report(99, 'advisory'), decision: { ...report(99, 'advisory').decision, passed: false } }, '85', 'advisory', false],
  ['valid advisory passes', report(99, 'advisory'), '90', 'advisory', true],
  ['ratchet required', report(99), '85', 'ratchet', false],
  ['ratchet false fails', { ...report(99), decision: { ...report(99).decision, ratchet: { ...ratchet(99), passed: false } } }, '85', 'ratchet', false],
  ['valid ratchet passes', { ...report(99), decision: { ...report(99).decision, ratchet: ratchet(99) } }, '85', 'ratchet', true],
  ['invalid ratchet delta fails', { ...report(99), decision: { ...report(99).decision, ratchet: { ...ratchet(99), score_delta: 1 } } }, '85', 'ratchet', false],
];
for (const [name, data, floor, mode, passes] of cases) {
  test(name, () => passes ? assert.doesNotThrow(() => enforceReport(data, floor, mode)) : assert.throws(() => enforceReport(data, floor, mode)));
}

test('input format is bounded and duplicate or malformed JSON fails', () => {
  for (const value of ['', '85.0', '1e2', '-1', '101', 'null', 'true', ' 85', '085', '$(exit 0)']) assert.throws(() => parseFloor(value));
  for (const value of ['0', '85', '90', '100']) assert.equal(parseFloor(value), Number(value));
  assert.throws(() => parseReport('{"score": 84, "score": 99}'), /duplicate/);
  assert.throws(() => parseReport('{"decision": {"passed":false,"pass\\u0065d":true}}'), /duplicate/);
  assert.throws(() => parseReport('{"score": 99'));
  assert.deepEqual(parseReport(JSON.stringify(report())), report());
});

function runAudit(t, { data = report(99), outcome = 0, behavior = 'report', floor = '85', mode = 'standard' } = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-action-test-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const root = path.join(dir, 'repo with spaces $(literal)'); fs.mkdirSync(root);
  const runner = path.join(dir, 'runner'); fs.mkdirSync(runner);
  fs.mkdirSync(path.join(root, 'target/jankurai'), { recursive: true });
  const stale = path.join(root, 'target/jankurai/repo-score.json'); fs.writeFileSync(stale, JSON.stringify(report(99)));
  const executable = path.join(dir, 'controlled-auditor');
  fs.writeFileSync(executable, `#!${process.execPath}\n` + `
import fs from 'node:fs';
const args = process.argv.slice(2);
fs.writeFileSync(process.env.ARGUMENTS, JSON.stringify(args));
const target = args[args.indexOf('--json') + 1];
if (process.env.BEHAVIOR === 'report') fs.writeFileSync(target, process.env.REPORT);
if (process.env.BEHAVIOR === 'symlink') fs.symlinkSync(process.env.STALE, target);
if (process.env.BEHAVIOR === 'oversized') { fs.writeFileSync(target, '{}'); fs.truncateSync(target, 32 * 1024 * 1024 + 1); }
process.exit(Number(process.env.OUTCOME));
`, { mode: 0o700 });
  const argsFile = path.join(dir, 'arguments.json'); const outputs = path.join(dir, 'outputs');
  const result = spawnSync('bash', [script, executable, root, mode, path.join(root, 'baseline.json'), floor], {
    cwd: root, encoding: 'utf8', env: {
      PATH: process.env.PATH, HOME: dir, RUNNER_TEMP: runner, GITHUB_OUTPUT: outputs,
      GITHUB_STEP_SUMMARY: path.join(dir, 'summary'), ARGUMENTS: argsFile,
      REPORT: typeof data === 'string' ? data : JSON.stringify(data), OUTCOME: String(outcome), BEHAVIOR: behavior, STALE: stale,
    },
  });
  return { result, args: fs.existsSync(argsFile) ? JSON.parse(fs.readFileSync(argsFile)) : null,
    outputs: fs.existsSync(outputs) ? fs.readFileSync(outputs, 'utf8') : '', runner, root, stale };
}

test('actual shell writes a unique full report and leaves stale repository data untouched', t => {
  const run = runAudit(t);
  assert.equal(run.result.status, 0, run.result.stderr);
  assert.equal(run.args[1], run.root);
  assert.ok(run.args.includes('--full'));
  assert.ok(!run.args.includes('--fail-under'));
  const file = run.args[run.args.indexOf('--json') + 1];
  assert.ok(file.startsWith(run.runner + path.sep));
  assert.equal(fs.statSync(path.dirname(file)).mode & 0o777, 0o700);
  assert.equal(JSON.parse(fs.readFileSync(run.stale)).score, 99);
  assert.match(run.outputs, /report-json=/);
});

test('a previous score cannot satisfy an auditor that emits no report', t => {
  const run = runAudit(t, { behavior: 'missing' });
  assert.notEqual(run.result.status, 0);
  assert.match(run.result.stderr, /ENOENT/);
});

test('a symlink report is rejected', t => {
  const run = runAudit(t, { behavior: 'symlink' });
  assert.notEqual(run.result.status, 0);
  assert.match(run.result.stderr, /regular file/);
});

test('actual shell preserves a failing auditor exit even with a passing report', t => {
  assert.equal(runAudit(t, { outcome: 7 }).result.status, 7);
});

test('actual shell rejects low and malformed fresh reports', t => {
  for (const data of [report(84), { ...report(), score: '99' }, '{"score":99']) assert.notEqual(runAudit(t, { data }).result.status, 0);
});

test('invalid floor prevents execution; ratchet baseline remains an argument', t => {
  const invalid = runAudit(t, { floor: '85; exit 0' });
  assert.notEqual(invalid.result.status, 0); assert.equal(invalid.args, null);
  const valid = runAudit(t, { mode: 'ratchet', data: { ...report(99), decision: { ...report(99).decision, ratchet: ratchet(99) } } });
  assert.equal(valid.result.status, 0, valid.result.stderr); assert.ok(valid.args.includes('--baseline'));
});

test('actual shell rejects an oversized report before parsing it', t => {
  const run = runAudit(t, { behavior: 'oversized' });
  assert.notEqual(run.result.status, 0);
  assert.match(run.result.stderr, /32 MiB size limit/);
});

test('actual shell rejects contradictory policy floors', t => {
  const run = runAudit(t, { data: { ...report(89), policy: { minimum_score: 90 } } });
  assert.notEqual(run.result.status, 0);
  assert.match(run.result.stderr, /policy and decision score floors disagree/);
});
