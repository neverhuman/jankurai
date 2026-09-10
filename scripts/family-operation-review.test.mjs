import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import * as op from './family-operation.mjs';

const self = fileURLToPath(import.meta.url);

function git(directory, ...args) {
  const r = spawnSync('/usr/bin/git', ['-C', directory, ...args], {
    encoding: 'utf8',
    env: {
      PATH: '/usr/bin:/bin', HOME: directory, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1',
      GIT_AUTHOR_NAME: 'fixture', GIT_AUTHOR_EMAIL: 'fixture@example.invalid',
      GIT_COMMITTER_NAME: 'fixture', GIT_COMMITTER_EMAIL: 'fixture@example.invalid',
    },
  });
  if (r.status !== 0) throw new Error(r.stderr || r.stdout);
  return r.stdout.trim();
}

function prepare(hub, observe) {
  const tx = op.acquire(hub, { observe });
  const identities = Object.fromEntries(op.LOCK_FILES.map(n => [n, op.captureFileIdentity(path.join(hub, n))]));
  op.beginPairedJournal(tx, { source: { hub: op.captureSource(hub), components: {} }, locks: identities });
  const after = { 'Cargo.lock': 'AFTER CARGO', 'family.lock': 'AFTER FAMILY' };
  op.recordAfterImages(tx, after);
  return { tx, identities, after };
}

function commit(hub, observe) {
  const p = prepare(hub, observe);
  op.commitPairedReplace(p.tx, p.after, p.identities);
}

function fixture(t, body) {
  const hub = fs.mkdtempSync(path.join(os.tmpdir(), 'pr33-defect-'));
  t.after(() => fs.rmSync(hub, { recursive: true, force: true }));
  for (const [n, b] of Object.entries({ 'Cargo.lock': 'BEFORE CARGO', 'family.lock': 'BEFORE FAMILY' })) {
    fs.writeFileSync(path.join(hub, n), b);
  }
  git(hub, 'init', '-q', '-b', 'main');
  git(hub, 'add', '.');
  git(hub, 'commit', '-q', '-m', 'fixture');
  body(hub);
}

if (process.argv[2] === 'crash-child') {
  commit(process.argv[3], (event, detail) => {
    if (event === 'after-replace' && detail.name === 'Cargo.lock') process.exit(37);
  });
  throw new Error('crash point not reached');
}

test('finishAdmissible allows naturally persisted mid-replace crash recovery', t => {
  fixture(t, hub => {
    const r = spawnSync(process.execPath, [self, 'crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(r.status, 37, r.stderr);
    const report = op.inspect(hub);
    assert.equal(report.writerStatus, 'stopped');
    assert.equal(report.journal.state, 'replacing');
    assert.equal(report.locks['Cargo.lock'].class, 'own-after');
    assert.equal(report.locks['family.lock'].class, 'original');
    assert.equal(report.finishAdmissible, true);
    const finished = op.finish(hub);
    assert.equal(finished.state, 'committed');
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'AFTER CARGO');
    assert.equal(fs.readFileSync(path.join(hub, 'family.lock'), 'utf8'), 'AFTER FAMILY');
    assert.equal(fs.existsSync(op.operationRoot(hub)), false);
  });
});

test('classifyLock own-after requires recorded after inode/dev/mode', t => {
  fixture(t, hub => {
    let externalInode;
    assert.throws(() => commit(hub, (event, detail) => {
      if (event === 'before-replace' && detail.name === 'family.lock') {
        const file = path.join(hub, 'Cargo.lock');
        fs.writeFileSync(`${file}.external`, 'AFTER CARGO', { mode: 0o750 });
        fs.renameSync(`${file}.external`, file);
        externalInode = fs.statSync(file).ino;
        throw new Error('authored second replacement failure');
      }
    }), /authored second replacement failure|lock identity changed/);
    const file = path.join(hub, 'Cargo.lock');
    // Foreign same-byte replacement must not be treated as owned and must be preserved.
    assert.equal(fs.readFileSync(file, 'utf8'), 'AFTER CARGO');
    assert.equal(fs.statSync(file).ino, externalInode);
    const report = op.inspect(hub);
    assert.equal(report.locks['Cargo.lock'].class, 'unknown-same-bytes');
  });
});

test('replace refuses unique concurrent edit observed at before-replace', t => {
  fixture(t, hub => {
    assert.throws(() => commit(hub, (event, detail) => {
      if (event === 'before-replace' && detail.name === 'Cargo.lock') {
        fs.writeFileSync(path.join(hub, 'Cargo.lock'), 'UNIQUE CONCURRENT EDIT');
      }
    }), /lock identity changed/);
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'UNIQUE CONCURRENT EDIT');
  });
});

test('rollback refuses unique concurrent edit observed at before-rollback-write', t => {
  fixture(t, hub => {
    assert.throws(() => commit(hub, (event, detail) => {
      if (event === 'before-replace' && detail.name === 'family.lock') {
        throw new Error('authored second replacement failure');
      }
      if (event === 'before-rollback-write' && detail.name === 'Cargo.lock') {
        fs.writeFileSync(path.join(hub, 'Cargo.lock'), 'UNIQUE ROLLBACK EDIT');
      }
    }), /authored second replacement failure|lock identity changed/);
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'UNIQUE ROLLBACK EDIT');
  });
});

test('absent or unreadable procfs reports uncertain writer status', () => {
  const writer = op.writerIdentity();
  const stat = fs.statSync;
  fs.statSync = (file, ...args) => {
    if (String(file) === '/proc') {
      throw Object.assign(new Error('simulated platform without procfs'), { code: 'ENOENT' });
    }
    return stat(file, ...args);
  };
  try {
    assert.equal(op.writerStatus(writer), 'uncertain');
  } finally {
    fs.statSync = stat;
  }
  process.kill(process.pid, 0);
  assert.equal(op.writerStatus(writer), 'live');
});
