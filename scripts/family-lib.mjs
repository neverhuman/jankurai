import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import TOML from '@iarna/toml';

export { operation, acquire, inspect, finish, rollback } from './family-operation.mjs';

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

function syncDirectory(directory) {
  const fd = fs.openSync(directory, fs.constants.O_RDONLY);
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}

/** Optional expected prior identity: { sha256, dev, ino, size? }. */
export function atomicWrite(file, content, { expected } = {}) {
  if (expected) {
    const st = fs.lstatSync(file);
    if (st.isSymbolicLink() || !st.isFile()) throw new Error(`${file}: refusing non-regular destination`);
    const bytes = fs.readFileSync(file);
    const sha256 = createHash('sha256').update(bytes).digest('hex');
    if (sha256 !== expected.sha256 || st.dev !== expected.dev || st.ino !== expected.ino
        || (expected.size != null && st.size !== expected.size)) {
      throw new Error(`${file}: destination identity changed before write`);
    }
  }
  const temporary = `${file}.${process.pid}.tmp`;
  let created = false;
  try {
    const fd = fs.openSync(temporary, 'wx', 0o644);
    created = true;
    try { fs.writeFileSync(fd, content); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
    fs.renameSync(temporary, file);
    syncDirectory(path.dirname(path.resolve(file)));
  } finally { if (created && exists(temporary)) fs.unlinkSync(temporary); }
}
export function clean(directory) {
  const dirty = gitText(directory, 'status', '--porcelain');
  if (dirty) throw new Error(`${path.basename(directory)}: dirty checkout; obtain a stopped-head handoff\n${dirty}`);
  const gd = gitText(directory, 'rev-parse', '--absolute-git-dir');
  for (const marker of ['index.lock', 'MERGE_HEAD', 'CHERRY_PICK_HEAD', 'rebase-merge', 'rebase-apply']) {
    if (exists(path.join(gd, marker))) throw new Error(`${directory}: Git operation in progress (${marker})`);
  }
}
export function temporaryCI(parent, prefix, action) {
  fs.mkdirSync(parent, { recursive: true });
  const directory = fs.mkdtempSync(path.join(parent, prefix));
  try { return action(directory); } finally { fs.rmSync(directory, { recursive: true, force: true }); }
}
export function buildEnvironment(source = process.env) {
  const env = { ...source };
  for (const key of ['GH_TOKEN', 'GITHUB_TOKEN', 'GH_ENTERPRISE_TOKEN', 'GITHUB_ENTERPRISE_TOKEN',
    'FAMILY_AUTOMATION_TOKEN', 'SSH_AUTH_SOCK', 'GIT_ASKPASS', 'SSH_ASKPASS', 'GIT_CONFIG_PARAMETERS']) delete env[key];
  for (const key of Object.keys(env)) if (key.startsWith('GIT_CONFIG_')) delete env[key];
  Object.assign(env, { GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', GIT_TERMINAL_PROMPT: '0' });
  return env;
}
