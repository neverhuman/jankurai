import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import { parsePhase, reportSummary, validateRecording, outcome } from './audit-recording.mjs';
import { raster, timeline } from './render-audit-gif.mjs';

const report = () => ({ score: 46, raw_score: 46, findings: [{}],
  decision: { status: 'fail', passed: false, minimum_score: 85, hard_findings: 0, soft_findings: 1 } });
const recording = () => ({ schema: 1, kind: 'synthetic-test', identity: { executableSha256: '0'.repeat(64) },
  events: [{ atMs: 0, position: 0, label: 'starting audit process' },
    { atMs: 125, position: 3, label: 'scan repository' }],
  result: { atMs: 345, exitCode: 1, signal: null, error: null, report: report(), reportSha256: '0'.repeat(64) } });

test('released and updated forced progress formats are observed without invented phases', () => {
  assert.deepEqual(parsePhase('\x1b[1;38;5;141m| [==========------------------] 3/8 scan repository\x1b[0m'), { position: 3, label: 'scan repository', total: 8 });
  assert.deepEqual(parsePhase('⠇ █████░░░░ 38%  3/8  scan repository'), { position: 3, label: 'scan repository', total: 8 });
  assert.deepEqual(parsePhase('⠋ ████████████████████████████████ 100%   8/8  score 36 raw 36 findings 22'),
    { position: 8, label: 'score 36 raw 36 findings 22', total: 8 });
  assert.equal(parsePhase('[progress] scoring repository'), null);
  // Bright TUI scorecard uses "36/100"; that must not look like a progress phase.
  assert.equal(parsePhase('│   36/100   raw 36    FAIL      │'), null);
  assert.equal(parsePhase('score=36 raw=36 caps=10 findings=22'), null);
  assert.throws(() => parsePhase('| [==] 3/9 scan repository'), /unsupported/);
  assert.throws(() => parsePhase('| [==] 3/8 invented scan'), /unsupported/);
});

test('successor seven-phase auditor timeline is observed without invented labels', () => {
  assert.deepEqual(parsePhase('| [==========------------------] 3/7 scan repository'),
    { position: 3, label: 'scan repository', total: 7 });
  assert.deepEqual(parsePhase('⠇ ████████░░ 57%  4/7  apply mode and baseline'),
    { position: 4, label: 'apply mode and baseline', total: 7 });
  assert.deepEqual(parsePhase('⠋ ████████████████████████████████ 100%   7/7  score 36 raw 36 findings 22'),
    { position: 7, label: 'score 36 raw 36 findings 22', total: 7 });
  assert.throws(() => parsePhase('| [==] 4/7 apply score policy'), /unsupported/);
  assert.throws(() => parsePhase('| [==] 3/7 invented scan'), /unsupported/);
  const r = recording();
  r.events = [
    { atMs: 0, position: 0, label: 'starting audit process' },
    { atMs: 100, position: 3, label: 'scan repository', total: 7 },
    { atMs: 140, position: 4, label: 'apply mode and baseline', total: 7 },
    { atMs: 200, position: 7, label: 'score 36 raw 36 findings 22', total: 7 },
  ];
  assert.equal(validateRecording(r, { allowSynthetic: true }).events.at(-1).position, 7);
  r.events.push({ atMs: 210, position: 8, label: 'score 36 raw 36 findings 22', total: 7 });
  assert.throws(() => validateRecording(r, { allowSynthetic: true }), /invalid phase timeline/);
});

test('failed policy and process outcomes never become PASS', () => {
  const r = recording(); assert.equal(outcome(r).passed, false);
  r.result.report.score = r.result.report.raw_score = 99;
  r.result.report.decision = { status: 'pass', passed: true, minimum_score: 85, hard_findings: 0, soft_findings: 0 };
  assert.equal(outcome(r).passed, false, 'exit failure preserved at high score');
  r.result.exitCode = 0; assert.equal(outcome(r).passed, true);
  r.result.signal = 'SIGTERM'; assert.equal(outcome(r).passed, false);
  r.result.signal = null; r.result.report.decision.hard_findings = 1; assert.equal(outcome(r).passed, false);
  r.result.report.decision.hard_findings = 0; r.result.report.decision.ratchet = { passed: false }; assert.equal(outcome(r).passed, false);
  r.result.report.decision.ratchet = null; r.result.report.decision.minimum_score = 100; assert.equal(outcome(r).passed, false);
});

test('missing, malformed and contradictory reports fail closed', () => {
  for (const score of [null, '91', NaN, -1, 101]) assert.throws(() => reportSummary({ ...report(), score }));
  const bad = report(); delete bad.decision.hard_findings; assert.throws(() => reportSummary(bad));
  bad.decision.hard_findings = 0; bad.decision.passed = true; assert.throws(() => reportSummary(bad), /contradictory/);
  const r = recording(); r.kind = 'recorded-process'; r.result.exitCode = 0; r.result.report = null;
  assert.throws(() => validateRecording(r), /lacks report/);
});

test('synthetic fixtures require explicit opt in and retain their visible label', () => {
  assert.throws(() => validateRecording(recording()), /synthetic/);
  assert.equal(validateRecording(recording(), { allowSynthetic: true }).kind, 'synthetic-test');
  const synthetic = recording(), actual = structuredClone(synthetic); actual.kind = 'recorded-process';
  const state = { atMs: 0, events: synthetic.events.slice(0, 1), finished: false };
  assert.notDeepEqual(raster(synthetic, state, 1).pixels, raster(actual, state, 1).pixels);
});

test('timeline uses captured elapsed time with only disclosed GIF quantization and final hold', () => {
  const frames = timeline(recording());
  assert.equal(frames.reduce((sum, f) => sum + f.delay, 0), 235);
  assert.equal(frames.at(-1).delay, 200);
  assert(frames.some(f => f.atMs === 200 && f.events.at(-1).position === 3 && !f.finished), 'clock advances without inventing phase progress');
  const r = recording(); r.events.push({ atMs: 126, position: 4, label: 'apply score policy' });
  assert.equal(timeline(r).length, frames.length, 'same GIF tick is coalesced, not stretched');
  r.events[1].atMs = -1; assert.throws(() => validateRecording(r, { allowSynthetic: true }));
});

test('glyphs contain readable shapes rather than interchangeable filled rectangles', () => {
  const font = JSON.parse(fs.readFileSync(new URL('./audit-mono.json', import.meta.url)));
  for (const face of Object.values(font.faces)) {
    assert.notDeepEqual(face.glyphs.A, face.glyphs.B);
    assert.notDeepEqual(face.glyphs.P, face.glyphs.F);
    assert(face.glyphs.A.some(row => row !== 0 && row !== 2 ** face.width - 1));
    assert(face.glyphs[' '].every(row => row === 0));
  }
  assert.equal(font.faces['4'].width, 40, 'native64px title face');
});

test('both output dimensions and every palette index are exact', () => {
  const r = recording(), state = { atMs: 345, events: r.events, finished: true };
  for (const scale of [1, 2]) {
    const frame = raster(r, state, scale);
    assert.equal(frame.width, 960 * scale); assert.equal(frame.height, 540 * scale);
    assert.equal(frame.pixels.length, frame.width * frame.height);
    assert([...new Set(frame.pixels)].every(v => v >= 0 && v < 16));
    assert(frame.pixels.includes(7), 'FAIL uses bright red');
  }
});
