import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import TOML from '@iarna/toml';

export const readToml = file => TOML.parse(fs.readFileSync(file, 'utf8'));
export const exists = file => fs.existsSync(file);
export const isLink = file => { try { return fs.lstatSync(file).isSymbolicLink(); } catch { return false; } };
export function run(args, { cwd, env = process.env, capture = false, check = true, input } = {}) {
  const result = spawnSync(String(args[0]), args.slice(1).map(String), { cwd, env, input, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
    stdio: capture || input !== undefined ? ['pipe', 'pipe', 'pipe'] : 'inherit' });
  if (result.error) throw result.error;
  if (check && result.status !== 0) throw new Error(`command failed (${result.status}): ${args.join(' ')}\n${result.stderr ?? ''}`);
  return capture && check ? result.stdout.trim() : result;
}
export const git = (directory, args, options = {}) => run(['git', '-C', directory, ...args], options);
export const gitText = (directory, ...args) => git(directory, args, { capture: true });
export function atomicWrite(file, content) {
  const temporary = `${file}.${process.pid}.tmp`;
  let created = false;
  try {
    const fd = fs.openSync(temporary, 'wx', 0o644);
    created = true;
    try { fs.writeFileSync(fd, content); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
    fs.renameSync(temporary, file);
  } finally { if (created && exists(temporary)) fs.unlinkSync(temporary); }
}
export function clean(directory) {
  if (gitText(directory, 'status', '--porcelain')) throw new Error(`${path.basename(directory)}: dirty checkout; obtain a stopped-head handoff`);
  const gd = gitText(directory, 'rev-parse', '--absolute-git-dir');
  for (const marker of ['index.lock', 'MERGE_HEAD', 'CHERRY_PICK_HEAD', 'rebase-merge', 'rebase-apply']) {
    if (exists(path.join(gd, marker))) throw new Error(`${directory}: Git operation in progress (${marker})`);
  }
}
export function operation(hub, action) {
  const lock = path.join(gitText(hub, 'rev-parse', '--absolute-git-dir'), 'family-operation');
  try { fs.mkdirSync(lock); } catch (error) {
    if (error.code === 'EEXIST') throw new Error('another family command owns this checkout; obtain a stopped-head handoff if interrupted');
    throw error;
  }
  try {
    fs.writeFileSync(path.join(lock, 'owner.json'), JSON.stringify({ pid: process.pid, host: os.hostname() }));
    return action();
  } finally { fs.rmSync(lock, { recursive: true }); }
}
export function temporaryCI(parent, prefix, action) {
  fs.mkdirSync(parent, { recursive: true });
  const directory = fs.mkdtempSync(path.join(parent, prefix));
  try { return action(directory); } finally { fs.rmSync(directory, { recursive: true, force: true }); }
}
export function buildEnvironment() {
  const env = { ...process.env };
  for (const key of ['GH_TOKEN', 'GITHUB_TOKEN', 'FAMILY_AUTOMATION_TOKEN']) delete env[key];
  return env;
}
