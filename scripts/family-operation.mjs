// Durable paired-lock journal and recovery. Node builtins only — no npm/TOML.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const JOURNAL_SCHEMA = 1;
export const LOCK_FILES = ['Cargo.lock', 'family.lock'];
const ABNORMAL = new Set(['needs-recovery', 'rolling-back', 'replacing', 'verifying', 'prepared', 'validating', 'preparing']);

const digest = bytes => createHash('sha256').update(bytes).digest('hex');

function gitText(directory, ...args) {
  const result = spawnSync('git', ['-C', directory, ...args], {
    encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
    env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', GIT_TERMINAL_PROMPT: '0' },
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed: ${result.stderr ?? ''}`);
  return result.stdout.trim();
}

export function syncDirectory(directory) {
  const fd = fs.openSync(directory, fs.constants.O_RDONLY);
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}

export function operationRoot(hub) {
  return path.join(gitText(hub, 'rev-parse', '--absolute-git-dir'), 'family-operation');
}

export function journalDir(root) {
  return path.join(root, 'journal');
}

export function journalPath(root) {
  return path.join(journalDir(root), 'journal.json');
}

export function imagePath(root, name, side) {
  return path.join(journalDir(root), 'images', `${name}.${side}`);
}

export function captureSource(directory) {
  return {
    head: gitText(directory, 'rev-parse', 'HEAD'),
    tree: gitText(directory, 'rev-parse', 'HEAD^{tree}'),
  };
}

export function captureFileIdentity(file) {
  const st = fs.lstatSync(file);
  if (st.isSymbolicLink() || !st.isFile()) throw new Error(`${file}: refusing non-regular lock path`);
  const bytes = fs.readFileSync(file);
  return {
    sha256: digest(bytes),
    mode: st.mode,
    size: st.size,
    dev: st.dev,
    ino: st.ino,
    bytes,
  };
}

export function identityMeta(identity) {
  const { sha256, mode, size, dev, ino } = identity;
  return { sha256, mode, size, dev, ino };
}

export function sameIdentity(file, expected) {
  try {
    const actual = captureFileIdentity(file);
    return actual.sha256 === expected.sha256 && actual.dev === expected.dev && actual.ino === expected.ino
      && actual.size === expected.size;
  } catch {
    return false;
  }
}

export function assertIdentity(file, expected, label = file) {
  if (!sameIdentity(file, expected)) throw new Error(`${label}: lock identity changed (dev/ino/bytes)`);
}

function processStartTicks(pid) {
  const raw = fs.readFileSync(`/proc/${pid}/stat`, 'utf8');
  const close = raw.lastIndexOf(')');
  if (close < 0) throw new Error('unreadable process identity');
  return raw.slice(close + 2).split(' ')[19];
}

export function writerIdentity() {
  const pid = process.pid;
  let startTime = null;
  try { startTime = processStartTicks(pid); } catch { startTime = String(Date.now()); }
  return { pid, hostname: os.hostname(), startTime, startedAt: new Date().toISOString() };
}

/** @returns {'live'|'stopped'|'uncertain'|'foreign-host'} */
export function writerStatus(writer) {
  if (!writer?.pid) return 'uncertain';
  if (writer.hostname && writer.hostname !== os.hostname()) return 'foreign-host';
  try {
    const start = processStartTicks(writer.pid);
    if (writer.startTime != null && String(writer.startTime) !== String(start)) return 'stopped';
    return 'live';
  } catch (error) {
    if (error?.code === 'ENOENT') return 'stopped';
    return 'uncertain';
  }
}

function writeExclusive(file, raw, mode = 0o644) {
  const fd = fs.openSync(file, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL, mode);
  try {
    fs.writeFileSync(fd, raw);
    fs.fsyncSync(fd);
  } finally { fs.closeSync(fd); }
}

export function durableWriteFile(file, raw, mode = 0o644) {
  const temporary = `${file}.${process.pid}.tmp`;
  let created = false;
  try {
    const fd = fs.openSync(temporary, 'wx', mode);
    created = true;
    try {
      fs.writeFileSync(fd, raw);
      fs.fsyncSync(fd);
    } finally { fs.closeSync(fd); }
    fs.renameSync(temporary, file);
    syncDirectory(path.dirname(file));
  } finally {
    try { if (created && fs.existsSync(temporary)) fs.unlinkSync(temporary); } catch { /* keep journal */ }
  }
}

function readJournal(root) {
  const file = journalPath(root);
  if (!fs.existsSync(file)) return null;
  const text = fs.readFileSync(file, 'utf8');
  const data = JSON.parse(text);
  if (data.schema !== JOURNAL_SCHEMA) throw new Error(`unsupported journal schema ${data.schema}`);
  return data;
}

function writeJournal(root, data, observe) {
  data.generation = (data.generation ?? 0) + 1;
  data.updatedAt = new Date().toISOString();
  const dir = journalDir(root);
  fs.mkdirSync(path.join(dir, 'images'), { recursive: true });
  durableWriteFile(journalPath(root), `${JSON.stringify(data, null, 2)}\n`);
  syncDirectory(dir);
  syncDirectory(root);
  observe?.('journal', { state: data.state, generation: data.generation });
  return data;
}

function emptyLocks() {
  return Object.fromEntries(LOCK_FILES.map(name => [name, {
    stage: 'original',
    before: null,
    after: null,
  }]));
}

export function acquire(hub, { observe } = {}) {
  const root = operationRoot(hub);
  try { fs.mkdirSync(root); } catch (error) {
    if (error.code === 'EEXIST') {
      const existing = safeReadJournal(root);
      if (existing && ABNORMAL.has(existing.state) && existing.state !== 'committed' && existing.state !== 'rolled-back') {
        throw new Error('family-operation needs recovery; run: family recover inspect');
      }
      throw new Error('another family command owns this checkout; obtain a stopped-head handoff if interrupted');
    }
    throw error;
  }
  syncDirectory(path.dirname(root));
  const writer = writerIdentity();
  durableWriteFile(path.join(root, 'owner.json'), `${JSON.stringify(writer, null, 2)}\n`);
  syncDirectory(root);
  const tx = {
    hub: path.resolve(hub),
    root,
    writer,
    journal: null,
    observe: observe ?? (() => {}),
    retain: false,
  };
  return tx;
}

function safeReadJournal(root) {
  try { return readJournal(root); } catch { return null; }
}

export function beginPairedJournal(tx, { source, locks }) {
  const dir = journalDir(tx.root);
  fs.mkdirSync(path.join(dir, 'images'), { recursive: true });
  syncDirectory(dir);
  for (const name of LOCK_FILES) {
    const identity = locks[name];
    writeExclusive(imagePath(tx.root, name, 'before'), identity.bytes);
    syncDirectory(path.join(dir, 'images'));
  }
  const journal = {
    schema: JOURNAL_SCHEMA,
    uuid: randomUUID(),
    generation: 0,
    state: 'preparing',
    source,
    writer: tx.writer,
    createdAt: new Date().toISOString(),
    locks: Object.fromEntries(LOCK_FILES.map(name => [name, {
      stage: 'original',
      before: identityMeta(locks[name]),
      after: null,
    }])),
    lastError: null,
  };
  tx.journal = writeJournal(tx.root, journal, tx.observe);
  return tx.journal;
}

export function setJournalState(tx, state, patch = {}) {
  const current = readJournal(tx.root);
  Object.assign(current, patch, { state });
  tx.journal = writeJournal(tx.root, current, tx.observe);
  if (ABNORMAL.has(state) && state !== 'preparing' && state !== 'validating') tx.retain = true;
  return tx.journal;
}

export function recordAfterImages(tx, afterBytes) {
  const current = readJournal(tx.root);
  const dir = path.join(journalDir(tx.root), 'images');
  for (const name of LOCK_FILES) {
    const bytes = Buffer.isBuffer(afterBytes[name]) ? afterBytes[name] : Buffer.from(afterBytes[name]);
    durableWriteFile(imagePath(tx.root, name, 'after'), bytes);
    current.locks[name].after = { sha256: digest(bytes), size: bytes.length };
  }
  syncDirectory(dir);
  tx.journal = writeJournal(tx.root, current, tx.observe);
  return tx.journal;
}

export function markLockStage(tx, name, stage) {
  const current = readJournal(tx.root);
  current.locks[name].stage = stage;
  tx.journal = writeJournal(tx.root, current, tx.observe);
  return tx.journal;
}

function loadImage(root, name, side) {
  const file = imagePath(root, name, side);
  const bytes = fs.readFileSync(file);
  return { bytes, sha256: digest(bytes), size: bytes.length };
}

export function classifyLock(hub, root, name, record) {
  const file = path.join(hub, name);
  let actual;
  try { actual = captureFileIdentity(file); }
  catch (error) {
    if (error?.code === 'ENOENT') return { class: 'missing', file };
    return { class: 'unreadable', file, error: String(error.message ?? error) };
  }
  const before = record?.before;
  const after = record?.after;
  if (before && actual.sha256 === before.sha256 && actual.dev === before.dev && actual.ino === before.ino) {
    return { class: 'original', identity: identityMeta(actual) };
  }
  if (after && actual.sha256 === after.sha256) {
    // Own after-image by content; inode may be the written file.
    const image = loadImage(root, name, 'after');
    if (image.sha256 === actual.sha256) return { class: 'own-after', identity: identityMeta(actual) };
  }
  if (before && actual.sha256 === before.sha256 && (actual.dev !== before.dev || actual.ino !== before.ino)) {
    return { class: 'unknown-same-bytes', identity: identityMeta(actual) };
  }
  return { class: 'unknown', identity: identityMeta(actual) };
}

function replaceLock(tx, name, afterBytes, expectedPrior) {
  const target = path.join(tx.hub, name);
  assertIdentity(target, expectedPrior, name);
  markLockStage(tx, name, 'replacing');
  tx.observe?.('before-replace', { name });
  const bytes = Buffer.isBuffer(afterBytes) ? afterBytes : Buffer.from(afterBytes);
  durableWriteFile(target, bytes, expectedPrior.mode & 0o777);
  const written = captureFileIdentity(target);
  if (written.sha256 !== digest(bytes)) throw new Error(`${name}: post-replace digest mismatch`);
  markLockStage(tx, name, 'replaced');
  tx.observe?.('after-replace', { name, identity: identityMeta(written) });
  return written;
}

export function commitPairedReplace(tx, afterBytes, beforeIdentities, { observe } = {}) {
  if (observe) tx.observe = observe;
  setJournalState(tx, 'prepared');
  setJournalState(tx, 'replacing');
  try {
    replaceLock(tx, 'Cargo.lock', afterBytes['Cargo.lock'], beforeIdentities['Cargo.lock']);
    replaceLock(tx, 'family.lock', afterBytes['family.lock'], beforeIdentities['family.lock']);
    setJournalState(tx, 'verifying');
    for (const name of LOCK_FILES) {
      const after = loadImage(tx.root, name, 'after');
      const actual = captureFileIdentity(path.join(tx.hub, name));
      if (actual.sha256 !== after.sha256) throw new Error(`${name}: verification digest mismatch`);
    }
    setJournalState(tx, 'committed');
    tx.retain = false;
  } catch (error) {
    tx.retain = true;
    setJournalState(tx, 'needs-recovery', { lastError: String(error.message ?? error) });
    try { conditionalRollback(tx); }
    catch (rollbackError) {
      setJournalState(tx, 'needs-recovery', {
        lastError: `${error.message}; rollback: ${rollbackError.message}`,
      });
    }
    throw error;
  }
}

/** Restore only members that still match this operation's after-image. */
export function conditionalRollback(tx) {
  const journal = readJournal(tx.root);
  setJournalState(tx, 'rolling-back');
  const restored = [];
  const preservedUnknown = [];
  const leftOriginal = [];
  for (const name of LOCK_FILES) {
    const classification = classifyLock(tx.hub, tx.root, name, journal.locks[name]);
    if (classification.class === 'own-after') {
      const before = loadImage(tx.root, name, 'before');
      const mode = journal.locks[name].before?.mode ?? 0o644;
      tx.observe?.('before-rollback-write', { name });
      durableWriteFile(path.join(tx.hub, name), before.bytes, mode & 0o777);
      const actual = captureFileIdentity(path.join(tx.hub, name));
      if (actual.sha256 !== before.sha256) throw new Error(`${name}: rollback digest mismatch`);
      markLockStage(tx, name, 'restored');
      restored.push(name);
      tx.observe?.('after-rollback-write', { name });
    } else if (classification.class === 'original') {
      leftOriginal.push(name);
      markLockStage(tx, name, 'original');
    } else {
      preservedUnknown.push(name);
      markLockStage(tx, name, `preserved-${classification.class}`);
      tx.retain = true;
    }
  }
  if (preservedUnknown.length) {
    setJournalState(tx, 'needs-recovery', {
      lastError: 'unknown lock edits preserved; journal retained',
      rollback: { restored, preservedUnknown, leftOriginal },
    });
    tx.retain = true;
    return { restored, preserved: preservedUnknown, unresolved: preservedUnknown, state: 'needs-recovery' };
  }
  setJournalState(tx, 'rolled-back', { rollback: { restored, leftOriginal } });
  tx.retain = false;
  return { restored, preserved: leftOriginal, unresolved: [], state: 'rolled-back' };
}

function releaseRoot(root) {
  fs.rmSync(root, { recursive: true, force: true });
  try { syncDirectory(path.dirname(root)); } catch { /* parent may be gone in tests */ }
}

export function release(tx, { force = false } = {}) {
  if (!force && tx.retain) return false;
  const journal = safeReadJournal(tx.root);
  if (!force && journal && ABNORMAL.has(journal.state) && journal.state !== 'committed' && journal.state !== 'rolled-back') {
    return false;
  }
  if (!force && journal && (journal.state === 'needs-recovery' || journal.state === 'rolling-back')) return false;
  releaseRoot(tx.root);
  return true;
}

function shouldRetain(tx) {
  if (tx.retain) return true;
  const journal = safeReadJournal(tx.root);
  if (!journal) return false;
  if (journal.state === 'committed' || journal.state === 'rolled-back') return false;
  if (journal.state === 'needs-recovery' || journal.state === 'rolling-back') return true;
  // Paired journal past preparing/validating without terminal state.
  if (['prepared', 'replacing', 'verifying'].includes(journal.state)) return true;
  return false;
}

export function operation(hub, action, { observe } = {}) {
  const tx = acquire(hub, { observe });
  try {
    const result = action(tx);
    if (!shouldRetain(tx)) release(tx);
    return result;
  } catch (error) {
    if (!shouldRetain(tx)) {
      try { release(tx, { force: true }); } catch { /* preserve lock on release failure */ }
    }
    throw error;
  }
}

function buildReport(hub) {
  const root = operationRoot(hub);
  const present = fs.existsSync(root);
  const report = {
    hub: path.resolve(hub),
    operationRoot: root,
    present,
    journal: null,
    writer: null,
    writerStatus: null,
    locks: {},
    finishAdmissible: false,
    rollbackAdmissible: false,
    reasons: [],
  };
  if (!present) {
    report.reasons.push('no family-operation directory');
    return report;
  }
  try {
    report.writer = JSON.parse(fs.readFileSync(path.join(root, 'owner.json'), 'utf8'));
  } catch {
    report.reasons.push('owner.json unreadable');
  }
  let journal;
  try {
    journal = readJournal(root);
    report.journal = {
      schema: journal.schema,
      uuid: journal.uuid,
      generation: journal.generation,
      state: journal.state,
      source: journal.source,
      createdAt: journal.createdAt,
      updatedAt: journal.updatedAt,
      lastError: journal.lastError ?? null,
      locks: Object.fromEntries(LOCK_FILES.map(name => [name, {
        stage: journal.locks?.[name]?.stage,
        before: journal.locks?.[name]?.before ?? null,
        after: journal.locks?.[name]?.after ?? null,
      }])),
    };
  } catch (error) {
    report.reasons.push(`journal unreadable: ${error.message}`);
    return report;
  }
  report.writerStatus = writerStatus(journal.writer ?? report.writer);
  for (const name of LOCK_FILES) {
    report.locks[name] = classifyLock(hub, root, name, journal.locks?.[name] ?? emptyLocks()[name]);
  }
  if (report.writerStatus === 'live') report.reasons.push('recorded writer is live');
  if (report.writerStatus === 'foreign-host') report.reasons.push('recorded writer host differs');
  if (report.writerStatus === 'uncertain') report.reasons.push('writer liveness uncertain');
  const classes = LOCK_FILES.map(name => report.locks[name].class);
  const onlyOwnOrOriginal = classes.every(c => c === 'original' || c === 'own-after');
  report.rollbackAdmissible = report.writerStatus === 'stopped' && onlyOwnOrOriginal
    && !['committed', 'rolled-back'].includes(journal.state);
  report.finishAdmissible = report.writerStatus === 'stopped'
    && journal.state === 'prepared'
    && classes.includes('own-after')
    && classes.every(c => c === 'original' || c === 'own-after')
    && journal.locks
    && LOCK_FILES.every(name => journal.locks[name]?.after && journal.locks[name]?.before);
  if (!report.rollbackAdmissible && report.writerStatus === 'stopped') {
    if (!onlyOwnOrOriginal) report.reasons.push('unknown lock edits block automatic rollback');
  }
  return report;
}

export function inspect(hub) {
  return buildReport(hub);
}

function refuseLive(report) {
  if (report.writerStatus === 'live') throw new Error('refusing recovery while recorded writer is live');
  if (report.writerStatus === 'foreign-host') throw new Error('refusing recovery for foreign-host writer');
  if (report.writerStatus === 'uncertain') throw new Error('refusing recovery while writer liveness is uncertain');
}

export function rollback(hub, { observe } = {}) {
  const report = buildReport(hub);
  if (!report.present) throw new Error('no family-operation to roll back');
  refuseLive(report);
  const tx = {
    hub: path.resolve(hub),
    root: report.operationRoot,
    writer: report.writer,
    journal: report.journal,
    observe: observe ?? (() => {}),
    retain: true,
  };
  const result = conditionalRollback(tx);
  if (result.state === 'rolled-back') release(tx);
  return { ...result, report: buildReport(hub) };
}

export function finish(hub, { observe } = {}) {
  const report = buildReport(hub);
  if (!report.present) throw new Error('no family-operation to finish');
  refuseLive(report);
  const journal = readJournal(report.operationRoot);
  if (journal.state === 'committed') {
    releaseRoot(report.operationRoot);
    return { state: 'committed', cleaned: true };
  }
  if (!report.finishAdmissible) {
    throw new Error(`finish not admissible: ${(report.reasons.join('; ') || journal.state)}`);
  }
  const tx = {
    hub: path.resolve(hub),
    root: report.operationRoot,
    writer: report.writer,
    journal,
    observe: observe ?? (() => {}),
    retain: false,
  };
  for (const name of LOCK_FILES) {
    const classification = report.locks[name];
    if (classification.class === 'original') {
      const after = loadImage(tx.root, name, 'after');
      durableWriteFile(path.join(tx.hub, name), after.bytes, (journal.locks[name].before?.mode ?? 0o644) & 0o777);
      markLockStage(tx, name, 'replaced');
    } else if (classification.class !== 'own-after') {
      throw new Error(`${name}: finish refused for class ${classification.class}`);
    }
  }
  setJournalState(tx, 'committed');
  release(tx);
  return { state: 'committed', cleaned: true };
}

export function recoverMain(argv = process.argv.slice(2)) {
  const [command, ...rest] = argv[0] === 'recover' ? argv.slice(1) : argv;
  const hub = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), '..'));
  if (command === 'inspect') {
    const report = inspect(hub);
    const json = rest.includes('--json') || !process.stdout.isTTY;
    if (json) process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
    else {
      console.log(`state=${report.journal?.state ?? 'absent'} writer=${report.writerStatus ?? 'n/a'}`);
      for (const name of LOCK_FILES) console.log(`${name}: ${report.locks[name]?.class ?? 'n/a'}`);
      if (report.reasons.length) console.log(`reasons: ${report.reasons.join('; ')}`);
    }
    return report;
  }
  if (command === 'finish') {
    const result = finish(hub);
    console.log(`recover finish: ${result.state}`);
    return result;
  }
  if (command === 'rollback') {
    const result = rollback(hub);
    console.log(`recover rollback: ${result.state}`);
    return result;
  }
  throw new Error('usage: family recover {inspect [--json]|finish|rollback}');
}
