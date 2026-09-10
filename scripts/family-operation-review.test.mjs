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

if (process.argv[2] === 'rename-crash-child') {
  const hub = process.argv[3], rename = fs.renameSync;
  fs.renameSync = (from, to) => {
    const result = rename(from, to);
    if (to === path.join(hub, 'Cargo.lock')) process.exit(38);
    return result;
  };
  commit(hub);
  throw new Error('rename crash point not reached');
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


for (const command of ['finish', 'rollback']) test(`real crash after rename before bookkeeping permits ${command}`, t => {
  fixture(t, hub => {
    const result = spawnSync(process.execPath, [self, 'rename-crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 38, result.stderr);
    const report = op.inspect(hub);
    assert.equal(report.locks['Cargo.lock'].class, 'own-after');
    assert.equal(report.finishAdmissible, true);
    assert.equal(report.rollbackAdmissible, true);
    const recovered = op[command](hub);
    assert.equal(recovered.state, command === 'finish' ? 'committed' : 'rolled-back');
    for (const name of op.LOCK_FILES) assert.match(fs.readFileSync(path.join(hub, name), 'utf8'), command === 'finish' ? /AFTER/ : /BEFORE/);
    assert.equal(fs.existsSync(op.operationRoot(hub)), false);
    assert.equal(fs.readdirSync(path.join(hub, '.git/family-operation-history')).length, 1);
  });
});

for (const name of op.LOCK_FILES) for (const side of ['before', 'after']) {
  test(`corrupted ${name} ${side} image blocks both recovery paths before mutation`, t => {
    fixture(t, hub => {
      const result = spawnSync(process.execPath, [self, 'crash-child', hub], { encoding: 'utf8', timeout: 10000 });
      assert.equal(result.status, 37, result.stderr);
      const root = op.operationRoot(hub);
      fs.writeFileSync(op.imagePath(root, name, side), 'CORRUPTED UNVALIDATED IMAGE');
      const before = op.LOCK_FILES.map(file => fs.readFileSync(path.join(hub, file), 'utf8'));
      const report = op.inspect(hub);
      assert.equal(report.finishAdmissible, false);
      assert.equal(report.rollbackAdmissible, false);
      assert.throws(() => op.finish(hub), /saved .* image disagrees/);
      assert.throws(() => op.rollback(hub), /saved .* image disagrees/);
      assert.deepEqual(op.LOCK_FILES.map(file => fs.readFileSync(path.join(hub, file), 'utf8')), before);
      assert.equal(fs.existsSync(root), true);
      assert.equal(fs.readFileSync(op.imagePath(root, name, side), 'utf8'), 'CORRUPTED UNVALIDATED IMAGE');
    });
  });
}

test('source advancement prevents finish and rollback and retains recovery evidence', t => {
  fixture(t, hub => {
    const result = spawnSync(process.execPath, [self, 'crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 37, result.stderr);
    git(hub, 'commit', '--quiet', '--allow-empty', '-m', 'concurrent source advance');
    assert.throws(() => op.finish(hub), /source HEAD\/tree changed/);
    assert.throws(() => op.rollback(hub), /source HEAD\/tree changed/);
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'AFTER CARGO');
    assert.equal(fs.existsSync(op.operationRoot(hub)), true);
  });
});

test('completed operations archive unknown additions and links without deleting evidence', t => {
  fixture(t, hub => {
    const { tx, identities, after } = prepare(hub);
    op.commitPairedReplace(tx, after, identities);
    const sentinel = path.join(tx.root, 'unknown-writer.txt');
    fs.writeFileSync(sentinel, 'UNKNOWN CONCURRENT EVIDENCE');
    const inode = fs.lstatSync(sentinel).ino;
    fs.symlinkSync('unknown-writer.txt', path.join(tx.root, 'unknown-link'));
    assert.equal(op.release(tx), true);
    const history = path.join(hub, '.git/family-operation-history');
    const entries = fs.readdirSync(history);
    assert.equal(entries.length, 1);
    const archive = path.join(history, entries[0]);
    assert.equal(fs.lstatSync(path.join(archive, 'unknown-writer.txt')).ino, inode);
    assert.equal(fs.readFileSync(path.join(archive, 'unknown-writer.txt'), 'utf8'), 'UNKNOWN CONCURRENT EVIDENCE');
    assert.equal(fs.readlinkSync(path.join(archive, 'unknown-link')), 'unknown-writer.txt');
    assert.equal(fs.existsSync(path.join(archive, 'journal/journal.json')), true);
  });
});
