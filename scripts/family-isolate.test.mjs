import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { Family } from './family-model.mjs';

function git(repo, args) {
  const result = spawnSync('git', ['-C', repo, ...args], {
    encoding: 'utf8',
    env: {
      PATH: '/usr/bin:/bin', HOME: repo,
      GIT_CONFIG_NOSYSTEM: '1', GIT_CONFIG_GLOBAL: '/dev/null',
      GIT_AUTHOR_NAME: 'fixture', GIT_AUTHOR_EMAIL: 'fixture@example.invalid',
      GIT_COMMITTER_NAME: 'fixture', GIT_COMMITTER_EMAIL: 'fixture@example.invalid',
    },
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function repoFixture(root, name) {
  const directory = path.join(root, name);
  fs.mkdirSync(directory);
  git(directory, ['init', '--quiet']);
  fs.writeFileSync(path.join(directory, 'owned.txt'), `${name} committed\n`);
  fs.writeFileSync(path.join(directory, '.env'), 'SECRET=1\n');
  fs.writeFileSync(path.join(directory, '.gitignore'), '.env\n');
  git(directory, ['add', 'owned.txt', '.gitignore']);
  git(directory, ['commit', '--quiet', '-m', 'fixture']);
  return { name, path: name, github: `https://example.invalid/${name}.git` };
}

function familyAt(root, repos) {
  const hub = path.join(root, 'jankurai');
  fs.mkdirSync(hub, { recursive: true });
  const family = Object.create(Family.prototype);
  Object.assign(family, {
    hub, root, fusion: path.join(hub, '.fusion'),
    repos: [{ name: 'jankurai', path: 'jankurai' }, ...repos],
    pins: new Map(repos.map(repo => [repo.name, { commit: git(path.join(root, repo.name), ['rev-parse', 'HEAD']) }])),
  });
  return family;
}

test('symlinked components ancestor is refused and source survives', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-symlink-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    fs.mkdirSync(family.fusion, { recursive: true });
    fs.symlinkSync(root, path.join(family.fusion, 'components'));
    assert.throws(() => family.fuse(false, true), /symlinked|redirected/);
    assert.equal(fs.existsSync(path.join(root, 'jankurai-core', 'owned.txt')), true);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('isolate replaces only a marked owned copy', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-owned-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    fs.mkdirSync(dest, { recursive: true });
    fs.writeFileSync(path.join(dest, 'valuable.txt'), 'UNIQUE\n');
    assert.throws(() => family.fuse(false, true), /overwrite|owned|outside|isolate/);
    assert.equal(fs.readFileSync(path.join(dest, 'valuable.txt'), 'utf8'), 'UNIQUE\n');
    fs.rmSync(dest, { recursive: true, force: true });
    family.fuse(false, true);
    fs.writeFileSync(path.join(dest, 'valuable.txt'), 'UNIQUE\n');
    assert.throws(() => family.fuse(false, true), /changed|inventory|material|identity/);
    assert.equal(fs.readFileSync(path.join(dest, 'valuable.txt'), 'utf8'), 'UNIQUE\n');
    assert.equal(fs.existsSync(path.join(dest, 'owned.txt')), true);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('isolate copies committed files only and does not dereference outside links', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-bound-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const directory = path.join(root, 'jankurai-core');
    const outside = path.join(root, 'outside.txt');
    fs.writeFileSync(outside, 'OUTSIDE\n');
    fs.symlinkSync(outside, path.join(directory, 'outside-link'));
    git(directory, ['add', 'outside-link']);
    git(directory, ['commit', '--quiet', '-m', 'link']);
    const family = familyAt(root, [core]);
    assert.throws(() => family.fuse(false, true), /escaped isolate|link/);
    assert.equal(fs.readFileSync(outside, 'utf8'), 'OUTSIDE\n');
    assert.equal(fs.readFileSync(path.join(directory, 'owned.txt'), 'utf8'), 'jankurai-core committed\n');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('missing isolate copy fails instead of running required on live source', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-missing-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    assert.throws(() => family.executionPath(core), /missing isolated execution copy/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('sidecar marker symlink is refused and the target is unchanged', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-marker-link-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    const sentinel = path.join(root, 'sentinel.txt');
    fs.writeFileSync(sentinel, 'KEEP\n');
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.symlinkSync(sentinel, `${dest}.jankurai-isolate`);
    assert.throws(() => family.fuse(false, true), /isolate marker|symlink/);
    assert.equal(fs.readFileSync(sentinel, 'utf8'), 'KEEP\n');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('isolate may unlink a fuse symlink that points at the live checkout', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-relink-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    family.fuse(false, false);
    family.fuse(false, true);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    assert.equal(fs.lstatSync(dest).isSymbolicLink(), false);
    const record = JSON.parse(fs.readFileSync(`${dest}.jankurai-isolate`, 'utf8'));
    assert.equal(record.kind, 'owned-execution-copy');
    assert.equal(fs.existsSync(path.join(dest, '.jankurai-isolate')), false);
    assert.equal(fs.readFileSync(path.join(root, 'jankurai-core', 'owned.txt'), 'utf8'), 'jankurai-core committed\n');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('default fuse can replace a marked isolate directory', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-reuse-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    family.fuse(false, true);
    family.fuse(false, false);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    assert.equal(fs.lstatSync(dest).isSymbolicLink(), true);
    assert.equal(fs.realpathSync(dest), path.join(root, 'jankurai-core'));
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('check rematerializes then disposes isolate copies', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-check-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    const live = path.join(root, 'jankurai-core');
    family.rematerializeIsolates();
    const copy = family.executionPath(core);
    fs.writeFileSync(path.join(copy, 'owned.txt'), 'dirty copy\n');
    assert.throws(() => family.rematerializeIsolates(), /changed|inventory|material/);
    assert.equal(fs.readFileSync(path.join(copy, 'owned.txt'), 'utf8'), 'dirty copy\n');
    family.disposeIsolates();
    assert.equal(fs.readFileSync(path.join(copy, 'owned.txt'), 'utf8'), 'dirty copy\n');
    assert.equal(fs.readFileSync(path.join(live, 'owned.txt'), 'utf8'), 'jankurai-core committed\n');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('forged filename membership does not authorize deleting unique bytes', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-forged-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    fs.mkdirSync(dest, { recursive: true });
    fs.writeFileSync(path.join(dest, 'valuable'), 'UNIQUE');
    fs.writeFileSync(`${dest}.jankurai-isolate`, JSON.stringify({
      kind: 'owned-execution-copy',
      source: path.join(root, 'jankurai-core'),
      commit: '0'.repeat(40),
      files: ['valuable'],
    }));
    assert.throws(() => family.fuse(false, true), /identity|incomplete|record/);
    assert.equal(fs.readFileSync(path.join(dest, 'valuable'), 'utf8'), 'UNIQUE');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('default fuse and dispose preserve changed isolate material', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-preserve-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    family.fuse(false, true);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    fs.writeFileSync(path.join(dest, 'valuable'), 'UNIQUE');
    assert.throws(() => family.fuse(false, false), /changed|inventory|material/);
    assert.equal(fs.readFileSync(path.join(dest, 'valuable'), 'utf8'), 'UNIQUE');
    family.disposeIsolates();
    assert.equal(fs.readFileSync(path.join(dest, 'valuable'), 'utf8'), 'UNIQUE');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('execution refuses a redirected root and changed copy contents', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-exec-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    family.fuse(false, true);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    fs.writeFileSync(path.join(dest, 'owned.txt'), 'MUTATED');
    assert.throws(() => family.executionPath(core), /changed|material/);
    const links = path.join(family.fusion, 'components');
    fs.rmSync(links, { recursive: true, force: true });
    fs.symlinkSync(root, links);
    fs.writeFileSync(`${path.join(root, 'jankurai-core')}.jankurai-isolate`, JSON.stringify({
      kind: 'owned-execution-copy', source: path.join(root, 'jankurai-core'),
      commit: git(path.join(root, 'jankurai-core'), ['rev-parse', 'HEAD']),
      files: [{ name: 'owned.txt', type: 'file', mode: 0o644, sha256: 'ab'.repeat(32), target: null }],
    }));
    assert.throws(() => family.executionPath(core), /redirected|live source|changed/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('export-ignore still materializes the complete committed tree', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-export-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const directory = path.join(root, 'jankurai-core');
    fs.writeFileSync(path.join(directory, '.gitattributes'), 'owned.txt export-ignore\n');
    git(directory, ['add', '.gitattributes']);
    git(directory, ['commit', '--quiet', '-m', 'export omission']);
    const family = familyAt(root, [core]);
    family.fuse(false, true);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    const record = JSON.parse(fs.readFileSync(`${dest}.jankurai-isolate`, 'utf8'));
    assert.equal(fs.existsSync(path.join(dest, 'owned.txt')), true);
    assert.equal(record.files.some(file => file.name === 'owned.txt'), true);
    assert.equal(family.executionPath(core), dest);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
