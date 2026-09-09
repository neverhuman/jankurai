#!/usr/bin/env node
// Records observed audit phases; does not manufacture execution evidence.
import fs from 'node:fs';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { parsePhase, sha256, reportSummary } from './audit-recording.mjs';

const [binArg, repoArg, outputArg] = process.argv.slice(2);
if (!binArg || !repoArg || !outputArg) throw new Error('usage: record-audit.mjs /absolute/jankurai /repository /new-recording-directory');
if (!path.isAbsolute(binArg)) throw new Error('auditor path must be absolute');
const bin = fs.realpathSync(binArg), repo = fs.realpathSync(repoArg), output = path.resolve(outputArg);
if (output === repo || output.startsWith(repo + path.sep)) throw new Error('recording output must be outside audited source');
fs.mkdirSync(output); // Existing output is never reused or removed.
const reportFile = path.join(output, 'report.json');
const argv = ['audit', repo, '--full', '--mode', 'standard', '--no-score-history',
  '--json', reportFile, '--md', path.join(output, 'report.md')];
const git = (...args) => {
  const r = spawnSync('git', ['--no-optional-locks', '-C', repo, ...args], { encoding: 'utf8' });
  return r.status === 0 ? r.stdout.trim() : null;
};
const discoveredRoot = git('rev-parse', '--show-toplevel');
const ownsGit = discoveredRoot !== null && fs.realpathSync(discoveredRoot) === repo;
const sourceGit = (...args) => ownsGit ? git(...args) : null;
const identity = { executableSha256: sha256(fs.readFileSync(bin)), executable: bin,
  repository: repo, commit: sourceGit('rev-parse', 'HEAD'), tree: sourceGit('rev-parse', 'HEAD^{tree}'),
  statusBefore: sourceGit('status', '--porcelain=v1', '--untracked-files=all'), argv };
const events = [{ atMs: 0, position: 0, label: 'starting audit process' }];
const startedEpochMs = Date.now();
const started = performance.now();
const now = () => Math.round(performance.now() - started);
let bytes = 0, error = null, stderr = '', stdout = '', pending = '';
const env = { ...process.env, JANKURAI_PROGRESS: 'always', JANKURAI_COLOR: 'always' };
delete env.JANKURAI_DEMO; // No producer-side artificial pauses.
const child = spawn(bin, argv, { cwd: repo, env, stdio: ['ignore', 'pipe', 'pipe'], detached: process.platform !== 'win32' });
const stop = reason => {
  if (error) return;
  error = reason;
  try { process.platform === 'win32' ? child.kill('SIGKILL') : process.kill(-child.pid, 'SIGKILL'); } catch { /* Already exited. */ }
};
const timer = setTimeout(() => stop('audit timed out after 600 seconds'), 600000);
const line = text => {
  try {
    const phase = parsePhase(text);
    if (!phase) return;
    if (phase.position < events.at(-1).position) return stop('audit phase moved backwards');
    events.push({ atMs: now(), ...phase });
  } catch (e) { stop(e.message); }
};
const capture = (stream, value) => {
  bytes += Buffer.byteLength(value);
  if (bytes > 8 * 1024 * 1024) return stop('combined audit output exceeds 8 MiB');
  if (stream === 'stdout') { stdout += value; process.stdout.write(value); return; }
  stderr += value; process.stderr.write(value); pending += value;
  const parts = pending.split(/[\r\n]/); pending = parts.pop(); parts.forEach(line);
};
child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
child.stdout.on('data', chunk => capture('stdout', chunk));
child.stderr.on('data', chunk => capture('stderr', chunk));
child.on('error', e => { error = String(e.message); });
child.on('close', (exitCode, signal) => {
  const completedAtMs = now();
  clearTimeout(timer); if (pending) line(pending);
  let report = null, reportSha256 = null;
  try {
    const st = fs.lstatSync(reportFile);
    if (!st.isFile() || st.isSymbolicLink() || st.size > 32 * 1024 * 1024) throw new Error('report is not a bounded fresh regular file');
    const raw = fs.readFileSync(reportFile); report = JSON.parse(raw); reportSummary(report); reportSha256 = sha256(raw);
  } catch (e) { error ||= String(e.message); report = null; }
  if (!events.some(e => e.position === 3)) error ||= 'auditor emitted no supported scan phase';
  if (sha256(fs.readFileSync(bin)) !== identity.executableSha256) error ||= 'auditor changed during recording';
  identity.statusAfter = sourceGit('status', '--porcelain=v1', '--untracked-files=all');
  fs.writeFileSync(path.join(output, 'stdout.txt'), stdout, { flag: 'wx' });
  fs.writeFileSync(path.join(output, 'stderr.txt'), stderr, { flag: 'wx' });
  const recording = { schema: 1, kind: 'recorded-process', identity, events, startedEpochMs,
    result: { atMs: completedAtMs, completedEpochMs: Date.now(), exitCode, signal, error, report, reportSha256 },
    stdoutSha256: sha256(stdout), stderrSha256: sha256(stderr) };
  fs.writeFileSync(path.join(output, 'recording.json'), JSON.stringify(recording, null, 2) + '\n', { flag: 'wx' });
  // Artifact generation must not turn an execution failure into green CI.
  process.exitCode = error || signal || exitCode !== 0 ? 1 : 0;
});
