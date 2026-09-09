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
    fs.writeFileSync(path.join(dest, '.jankurai-isolate'), 'owned-execution-copy\n');
    family.fuse(false, true);
    assert.equal(fs.existsSync(path.join(dest, 'valuable.txt')), false);
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
    family.fuse(false, true);
    const copy = path.join(family.fusion, 'components', 'jankurai-core');
    assert.equal(fs.existsSync(path.join(copy, '.env')), false);
    assert.equal(fs.existsSync(path.join(copy, '.git')), false);
    assert.equal(fs.readFileSync(path.join(copy, 'owned.txt'), 'utf8'), 'jankurai-core committed\n');
    if (fs.existsSync(path.join(copy, 'outside-link'))) {
      assert.equal(fs.lstatSync(path.join(copy, 'outside-link')).isSymbolicLink(), true);
    }
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

test('isolate may unlink a fuse symlink that points at the live checkout', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'isolate-relink-'));
  try {
    const core = repoFixture(root, 'jankurai-core');
    const family = familyAt(root, [core]);
    family.fuse(false, false);
    family.fuse(false, true);
    const dest = path.join(family.fusion, 'components', 'jankurai-core');
    assert.equal(fs.lstatSync(dest).isSymbolicLink(), false);
    assert.equal(fs.existsSync(path.join(dest, '.jankurai-isolate')), true);
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
    family.rematerializeIsolates();
    assert.equal(fs.readFileSync(path.join(family.executionPath(core), 'owned.txt'), 'utf8'), 'jankurai-core committed\n');
    family.disposeIsolates();
    assert.throws(() => family.executionPath(core), /missing isolated execution copy/);
    assert.equal(fs.readFileSync(path.join(live, 'owned.txt'), 'utf8'), 'jankurai-core committed\n');
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
