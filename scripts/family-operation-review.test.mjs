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
  commit(process.argv[3], (event, detail) => {
    if (event === 'after-atomic-exchange' && detail.name === 'Cargo.lock' && detail.direction === 'replace') process.exit(38);
  });
  throw new Error('exchange crash point not reached');
}

if (process.argv[2] === 'prepared-crash-child') {
  const { tx } = prepare(process.argv[3]);
  op.setJournalState(tx, 'prepared');
  process.exit(36);
}

if (process.argv[2] === 'rollback-crash-child') {
  commit(process.argv[3], (event, detail) => {
    if (event === 'before-replace' && detail.name === 'family.lock') throw new Error('force rollback');
    if (event === 'after-atomic-exchange' && detail.direction === 'rollback') process.exit(39);
  });
  throw new Error('rollback crash point not reached');
}

if (process.argv[2] === 'crash-child') {
  commit(process.argv[3], (event, detail) => {
    if (event === 'after-replace' && detail.name === 'Cargo.lock') process.exit(37);
  });
  throw new Error('crash point not reached');
}

if (process.argv[2] === 'helper-substitution-child') {
  const hub = process.argv[3];
  let replaced = false;
  assert.throws(() => commit(hub, (event, detail) => {
    if (event !== 'after-atomic-exchange' || detail.name !== 'Cargo.lock' || detail.direction !== 'replace') return;
    const builds = fs.readdirSync(os.tmpdir()).filter(name => name.startsWith('jankurai-family-native-'));
    assert.equal(builds.length, 1);
    const helper = path.join(os.tmpdir(), builds[0], 'helper');
    fs.writeFileSync(helper, `#!/bin/sh\nprintf substituted > '${path.join(hub, '.git/substituted')}'\n`, { mode: 0o700 });
    replaced = true;
  }), /compiled recovery helper: lock identity changed/);
  assert.equal(replaced, true);
  assert.equal(fs.existsSync(path.join(hub, '.git/substituted')), false);
  assert.equal(fs.existsSync(path.join(hub, '.git/fake-compiler-invoked')), false);
  process.exit(0);
}

test('compiler overrides and substitution of a compiled helper cannot execute candidate bytes', t => {
  fixture(t, hub => {
    const tmp = path.join(hub, '.git/native-test');
    fs.mkdirSync(tmp);
    const fake = path.join(hub, '.git/fake-rustc');
    fs.writeFileSync(fake, `#!/bin/sh\nprintf invoked > '${path.join(hub, '.git/fake-compiler-invoked')}'\necho 'rustc 1.97.1 forged'\n`, { mode: 0o700 });
    const child = spawnSync(process.execPath, [self, 'helper-substitution-child', hub], {
      encoding: 'utf8', timeout: 60000,
      env: { ...process.env, TMPDIR: tmp, HOME: path.join(hub, '.git'), RUSTC: fake,
        RUSTUP_HOME: path.join(hub, '.git'), RUSTUP_TOOLCHAIN: 'nightly' },
    });
    assert.equal(child.status, 0, child.stderr);
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'AFTER CARGO');
    assert.equal(fs.readFileSync(path.join(hub, 'family.lock'), 'utf8'), 'BEFORE FAMILY');
    assert.equal(fs.existsSync(op.operationRoot(hub)), true);
    const report = op.inspect(hub);
    assert.equal(report.nativeCapability.available, true);
    assert.match(report.nativeCapability.compilerSha256, /^[a-f0-9]{64}$/);
    assert.equal(op.finish(hub).state, 'committed');
    assert.equal(fs.readFileSync(path.join(hub, 'family.lock'), 'utf8'), 'AFTER FAMILY');
  });
});

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

test('absent or unreadable procfs reports uncertain writer status', { skip: process.platform !== 'linux' }, () => {
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


for (const command of ['finish', 'rollback']) test(`real crash after atomic exchange before bookkeeping permits ${command}`, t => {
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


for (const direction of ['replace', 'rollback']) test(`post-assertion edit at actual ${direction} exchange remains at its original path`, t => {
  fixture(t, hub => {
    let injected = false;
    assert.throws(() => commit(hub, (event, detail) => {
      if (direction === 'rollback' && event === 'before-replace' && detail.name === 'family.lock') throw new Error('force rollback');
      if (event === 'before-atomic-exchange' && detail.name === 'Cargo.lock' && detail.direction === direction) {
        injected = true;
        fs.writeFileSync(path.join(hub, 'Cargo.lock'), 'UNIQUE POST-ASSERTION EDIT');
      }
    }), /concurrent edit preserved|force rollback/);
    assert.equal(injected, true);
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'UNIQUE POST-ASSERTION EDIT');
    assert.equal(fs.existsSync(op.operationRoot(hub)), true);
  });
});


test('live writer blocks both recovery commands without journal or lock mutation', t => {
  fixture(t, hub => {
    const { tx } = prepare(hub);
    const journal = fs.readFileSync(op.journalPath(tx.root));
    const locks = op.LOCK_FILES.map(name => op.captureFileIdentity(path.join(hub, name)));
    for (const command of ['finish', 'rollback']) assert.throws(() => op[command](hub), /recorded writer is live/);
    assert.deepEqual(fs.readFileSync(op.journalPath(tx.root)), journal);
    for (const [i, name] of op.LOCK_FILES.entries()) assert.equal(op.sameIdentity(path.join(hub, name), locks[i]), true);
  });
});

test('legacy wall-clock fallback cannot declare an extant writer stopped', () => {
  const writer = { ...op.writerIdentity(), startTime: String(Date.now()) };
  delete writer.startKind;
  assert.equal(op.writerStatus(writer), 'uncertain');
});

test('concurrent recovery lock excludes a second real recovery process', t => {
  fixture(t, hub => {
    const crashed = spawnSync(process.execPath, [self, 'crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(crashed.status, 37, crashed.stderr);
    const root = op.operationRoot(hub), before = fs.readFileSync(op.journalPath(root));
    const code = `import fcntl, os, subprocess, sys
fd = os.open(sys.argv[1], os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
try:
    child = subprocess.run([sys.argv[2], '--input-type=module', '-e', sys.argv[3]], capture_output=True, text=True, timeout=10)
    assert child.returncode != 0, child.stdout
    assert 'Resource temporarily unavailable' in child.stderr, child.stderr
    # The internal worker must not trust a matching but unlocked descriptor,
    # even with a matching parent id and attacker-selected environment fields.
    separate = os.open(sys.argv[1], os.O_RDWR | os.O_NOFOLLOW)
    try:
        env = dict(os.environ, JANKURAI_RECOVERY_FD=str(separate), JANKURAI_RECOVERY_PARENT=str(os.getpid()))
        forged = subprocess.run([sys.argv[2], sys.argv[4], '--recovery-worker', sys.argv[5], 'finish'],
            capture_output=True, text=True, timeout=10, env=env, pass_fds=(separate,))
        assert forged.returncode != 0, forged.stdout
        assert 'Resource temporarily unavailable' in forged.stderr, forged.stderr
    finally:
        os.close(separate)
finally:
    os.close(fd)
`;
    const module = new URL('./family-operation.mjs', import.meta.url).href;
    const result = spawnSync('/usr/bin/python3', ['-I', '-c', code, path.join(hub, '.git/family-recovery.lock'),
      process.execPath, `import { finish } from ${JSON.stringify(module)}; finish(${JSON.stringify(hub)});`,
      fileURLToPath(module), hub], {
      encoding: 'utf8', timeout: 30000,
    });
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(fs.readFileSync(op.journalPath(root)), before);
    assert.equal(op.finish(hub).state, 'committed');
  });
});

test('uncommitted source edits block recovery and remain untouched', t => {
  fixture(t, hub => {
    const crashed = spawnSync(process.execPath, [self, 'crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(crashed.status, 37, crashed.stderr);
    fs.writeFileSync(path.join(hub, 'source-change'), 'UNCOMMITTED CONCURRENT SOURCE');
    for (const command of ['finish', 'rollback']) assert.throws(() => op[command](hub), /source has concurrent edits/);
    assert.equal(fs.readFileSync(path.join(hub, 'source-change'), 'utf8'), 'UNCOMMITTED CONCURRENT SOURCE');
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'AFTER CARGO');
    assert.equal(fs.existsSync(op.operationRoot(hub)), true);
  });
});

for (const kind of ['symlink', 'fifo', 'oversized']) test(`non-regular or oversized ${kind} image is refused before recovery`, t => {
  fixture(t, hub => {
    const crashed = spawnSync(process.execPath, [self, 'crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(crashed.status, 37, crashed.stderr);
    const root = op.operationRoot(hub), image = op.imagePath(root, 'Cargo.lock', 'before');
    fs.renameSync(image, `${image}.retained`);
    if (kind === 'symlink') fs.symlinkSync(`${image}.retained`, image);
    if (kind === 'fifo') {
      const mkfifo = spawnSync('/usr/bin/mkfifo', [image], { encoding: 'utf8' });
      assert.equal(mkfifo.status, 0, mkfifo.stderr);
    }
    if (kind === 'oversized') { fs.writeFileSync(image, ''); fs.truncateSync(image, 64 * 1024 * 1024 + 1); }
    const report = op.inspect(hub);
    assert.equal(report.finishAdmissible, false);
    assert.equal(report.rollbackAdmissible, false);
    for (const command of ['finish', 'rollback']) assert.throws(() => op[command](hub));
    assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'AFTER CARGO');
    assert.equal(fs.existsSync(root), true);
    assert.equal(fs.readFileSync(`${image}.retained`, 'utf8'), 'BEFORE CARGO');
  });
});


for (const command of ['finish', 'rollback']) test(`prepared crash before either replacement permits ${command}`, t => {
  fixture(t, hub => {
    const crashed = spawnSync(process.execPath, [self, 'prepared-crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(crashed.status, 36, crashed.stderr);
    assert.equal(op.inspect(hub).finishAdmissible, true);
    assert.equal(op[command](hub).state, command === 'finish' ? 'committed' : 'rolled-back');
    for (const name of op.LOCK_FILES) assert.match(fs.readFileSync(path.join(hub, name), 'utf8'), command === 'finish' ? /^AFTER/ : /^BEFORE/);
  });
});

test('real crash during rollback can restart and complete rollback', t => {
  fixture(t, hub => {
    const crashed = spawnSync(process.execPath, [self, 'rollback-crash-child', hub], { encoding: 'utf8', timeout: 10000 });
    assert.equal(crashed.status, 39, crashed.stderr);
    assert.equal(op.inspect(hub).journal.state, 'rolling-back');
    assert.equal(op.rollback(hub).state, 'rolled-back');
    for (const name of op.LOCK_FILES) assert.match(fs.readFileSync(path.join(hub, name), 'utf8'), /^BEFORE/);
  });
});
