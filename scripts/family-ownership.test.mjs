import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { Family } from './family-model.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';

const sha = value => createHash('sha256').update(value).digest('hex');
function git(directory, ...args) {
  const r = spawnSync('/usr/bin/git', ['-C', directory, ...args], { encoding: 'utf8', env: {
    PATH: '/usr/bin:/bin', HOME: directory, GIT_CONFIG_NOSYSTEM: '1', GIT_CONFIG_GLOBAL: '/dev/null',
    GIT_AUTHOR_NAME: 'fixture', GIT_AUTHOR_EMAIL: 'fixture@example.invalid',
    GIT_COMMITTER_NAME: 'fixture', GIT_COMMITTER_EMAIL: 'fixture@example.invalid',
  } });
  if (r.status !== 0) throw new Error(r.stderr);
  return r.stdout.trim();
}
function fixture(fn) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'a5-authored-methods-')));
  try {
    const source = path.join(root, 'jankurai-core'), hub = path.join(root, 'jankurai');
    fs.mkdirSync(source); fs.mkdirSync(hub); git(source, 'init', '--quiet');
    fs.writeFileSync(path.join(source, 'owned.txt'), 'committed fixture\n');
    git(source, 'add', '.'); git(source, 'commit', '--quiet', '-m', 'authored fixture');
    const repo = { name: 'jankurai-core', path: 'jankurai-core', github: 'https://example.invalid/core.git' };
    const family = Object.create(Family.prototype);
    Object.assign(family, { hub, root, fusion: path.join(hub, '.fusion'), repos: [{ name: 'jankurai' }, repo],
      pins: new Map([[repo.name, { commit: git(source, 'rev-parse', 'HEAD') }]]) });
    const links = family.ownedComponentRoot(true), dest = path.join(links, repo.name);
    return fn({ root, source, hub, family, repo, links, dest });
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
}
function probe(name, fn) {
  test(name, () => { const result = fixture(fn); assert.equal(result.expectationMet, true, JSON.stringify(result)); });
}

function tryAction(fn) { try { return { value: fn() }; } catch (e) { return { error: e.message }; } }

for (const route of ['execution', 'rematerialize', 'default-fuse', 'dispose']) probe(`same-inode marker rewrite cannot authorize ${route}`, ({ source, family, repo, dest }) => {
  family.materializeIsolate(source, dest);
  const marker = `${dest}.jankurai-isolate`, beforeStat = fs.statSync(marker);
  const authority = path.join(family.fusion, '.isolate-owned', sha(path.resolve(dest)));
  const authorityBefore = sha(fs.readFileSync(authority));
  const file = path.join(dest, 'owned.txt'); fs.writeFileSync(file, 'UNIQUE CHANGED CONTENT');
  const record = JSON.parse(fs.readFileSync(marker, 'utf8'));
  const entry = record.files.find(f => f.name === 'owned.txt');
  entry.sha256 = sha(fs.readFileSync(file)); entry.size = fs.statSync(file).size;
  fs.writeFileSync(marker, JSON.stringify(record));
  const sameInode = fs.statSync(marker).ino === beforeStat.ino;
  const authorityInitiallyUnchanged = authorityBefore === sha(fs.readFileSync(authority));
  const outcome = tryAction(() => route === 'execution' ? family.executionPath(repo)
    : route === 'rematerialize' ? family.materializeIsolate(source, dest)
    : route === 'default-fuse' ? family.fuse(false, false) : family.disposeIsolates());
  const preserved = fs.existsSync(file) && fs.readFileSync(file, 'utf8') === 'UNIQUE CHANGED CONTENT';
  const accepted = route === 'execution' && outcome.value === dest;
  return { expectationMet: preserved && !accepted, sameInode, authorityInitiallyUnchanged,
    authorityHashBefore: authorityBefore, preserved, accepted, error: outcome.error };
});

probe('forged marker plus writable authority cannot authorize an unknown copy', ({ source, family, repo, dest }) => {
  fs.mkdirSync(dest); fs.writeFileSync(path.join(dest, 'valuable.txt'), 'UNIQUE');
  const commit = git(source, 'rev-parse', 'HEAD'), tree = git(source, 'rev-parse', 'HEAD^{tree}');
  const files = [{ name: 'valuable.txt', type: 'file', mode: fs.statSync(path.join(dest, 'valuable.txt')).mode & 0o777, sha256: sha('UNIQUE'), size: 6, target: null }];
  const marker = `${dest}.jankurai-isolate`, record = { kind: 'owned-execution-copy', source, commit, tree, files };
  fs.writeFileSync(marker, JSON.stringify(record));
  const ds = fs.statSync(dest), ms = fs.statSync(marker), dir = path.join(family.fusion, '.isolate-owned'); fs.mkdirSync(dir);
  fs.writeFileSync(path.join(dir, sha(path.resolve(dest))), JSON.stringify({ ...record, dest, destDev: ds.dev, destIno: ds.ino, markDev: ms.dev, markIno: ms.ino }));
  const result = tryAction(() => family.executionPath(repo));
  return { expectationMet: !!result.error, accepted: result.value === dest, error: result.error };
});

probe('Git reference rewriting must be rejected before execution', ({ root, source, family, repo, dest }) => {
  const other = path.join(root, 'other-authored'); fs.mkdirSync(other); git(other, 'init', '--quiet');
  fs.writeFileSync(path.join(other, 'other'), 'different fixture'); git(other, 'add', '.'); git(other, 'commit', '--quiet', '-m', 'different fixture');
  family.materializeIsolate(source, dest);
  fs.writeFileSync(path.join(dest, '.git'), `gitdir: ${path.join(other, '.git')}\n`);
  const result = tryAction(() => family.executionPath(repo));
  const observed = git(dest, 'rev-parse', 'HEAD'), accepted = git(source, 'rev-parse', 'HEAD');
  return { expectationMet: !!result.error, acceptedChangedReference: result.value === dest, observed, accepted, error: result.error };
});

probe('unknown Git metadata bytes must survive disposal', ({ source, family, dest }) => {
  family.materializeIsolate(source, dest);
  const unique = path.join(`${dest}.gitdir`, 'unique-owner-evidence'); fs.writeFileSync(unique, 'UNIQUE METADATA');
  family.disposeIsolates();
  const preserved = fs.existsSync(unique) && fs.readFileSync(unique, 'utf8') === 'UNIQUE METADATA';
  return { expectationMet: preserved, preserved };
});

probe('snapshot Git status must match the accepted tree', ({ source, family, dest }) => {
  family.materializeIsolate(source, dest);
  const headMatches = git(dest, 'rev-parse', 'HEAD') === git(source, 'rev-parse', 'HEAD');
  const status = git(dest, 'status', '--porcelain');
  return { expectationMet: headMatches && status === '', headMatches, status };
});


probe('changed authority receipt is refused and retained', ({ source, family, repo, dest }) => {
  family.materializeIsolate(source, dest);
  const authority = path.join(family.fusion, '.isolate-owned', sha(path.resolve(dest)));
  const changed = fs.readFileSync(authority, 'utf8') + ' ';
  fs.writeFileSync(authority, changed);
  const result = tryAction(() => family.executionPath(repo));
  family.disposeIsolates();
  return { expectationMet: !!result.error && fs.readFileSync(authority, 'utf8') === changed };
});

probe('a new process cannot reclaim disk-only ownership', ({ source, family, repo, dest }) => {
  family.materializeIsolate(source, dest);
  const script = `import { Family } from ${JSON.stringify(new URL('./family-model.mjs', import.meta.url).href)};
    const family = Object.create(Family.prototype);
    Object.assign(family, ${JSON.stringify({ hub: family.hub, root: family.root, fusion: family.fusion, repos: family.repos })});
    family.pins = new Map(${JSON.stringify([...family.pins])});
    try { family.executionPath(${JSON.stringify(repo)}); process.exitCode = 7; }
    catch (error) { if (!/process-owned isolate authority/.test(error.message)) throw error; }`;
  const result = spawnSync(process.execPath, ['--input-type=module', '-e', script], { encoding: 'utf8' });
  return { expectationMet: result.status === 0 && fs.existsSync(dest), status: result.status, stderr: result.stderr };
});

probe('metadata symlinks are preserved without following their targets', ({ root, source, family, repo, dest }) => {
  family.materializeIsolate(source, dest);
  const outside = path.join(root, 'outside-metadata'); fs.writeFileSync(outside, 'PRESERVE');
  const link = path.join(`${dest}.gitdir`, 'unknown-link'); fs.symlinkSync(outside, link);
  const result = tryAction(() => family.executionPath(repo)); family.disposeIsolates();
  return { expectationMet: !!result.error && fs.lstatSync(link).isSymbolicLink() && fs.readFileSync(outside, 'utf8') === 'PRESERVE' };
});

probe('oversized metadata is bounded and preserved', ({ source, family, repo, dest }) => {
  family.materializeIsolate(source, dest);
  const file = path.join(`${dest}.gitdir`, 'oversized-owner-output');
  const fd = fs.openSync(file, 'wx'); try { fs.ftruncateSync(fd, 64 * 1024 * 1024 + 1); } finally { fs.closeSync(fd); }
  const result = tryAction(() => family.executionPath(repo)); family.disposeIsolates();
  return { expectationMet: /oversized/.test(result.error || '') && fs.statSync(file).size === 64 * 1024 * 1024 + 1 };
});
