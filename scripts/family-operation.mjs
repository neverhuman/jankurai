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
    directory: path.resolve(directory),
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
      && actual.size === expected.size
      && (expected.mode == null || actual.mode === expected.mode);
  } catch {
    return false;
  }
}

export function assertIdentity(file, expected, label = file) {
  if (!sameIdentity(file, expected)) throw new Error(`${label}: lock identity changed (dev/ino/mode/bytes)`);
}

function processStartTicks(pid) {
  const raw = fs.readFileSync(`/proc/${pid}/stat`, 'utf8');
  const close = raw.lastIndexOf(')');
  if (close < 0) throw new Error('unreadable process identity');
  return raw.slice(close + 2).split(' ')[19];
}

/** True when /proc is present and readable enough to interpret pid absence as stopped. */
export function procfsUsable() {
  try {
    const st = fs.statSync('/proc');
    if (!st.isDirectory()) return false;
    fs.accessSync('/proc', fs.constants.R_OK);
    // Confirm the interface we read — missing here means absent/unreadable procfs, not a dead pid.
    fs.accessSync('/proc/self/stat', fs.constants.R_OK);
    return true;
  } catch {
    return false;
  }
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
  if (!procfsUsable()) return 'uncertain';
  try {
    const start = processStartTicks(writer.pid);
    if (writer.startTime != null && String(writer.startTime) !== String(start)) return 'stopped';
    return 'live';
  } catch (error) {
    // With usable procfs, missing /proc/<pid>/stat means the process is gone.
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

export function durableWriteFile(file, raw, mode = 0o644, prepared) {
  const temporary = `${file}.${process.pid}.tmp`;
  let created = false;
  try {
    const fd = fs.openSync(temporary, 'wx', mode);
    created = true;
    try {
      fs.writeFileSync(fd, raw);
      fs.fsyncSync(fd);
    } finally { fs.closeSync(fd); }
    // Persist the intended inode before the replacement syscall can succeed.
    prepared?.(captureFileIdentity(temporary));
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
  const expected = readJournal(root)?.locks?.[name]?.[side];
  const actual = captureFileIdentity(file);
  if (!expected || actual.sha256 !== expected.sha256 || actual.size !== expected.size) {
    throw new Error(`${name}: saved ${side} image disagrees with journal`);
  }
  return { bytes: actual.bytes, sha256: actual.sha256, size: actual.size };
}

function validateRecoveryInputs(hub, root) {
  const journal = readJournal(root);
  if (!journal?.source?.hub || !journal.source.components) throw new Error('missing journal source identities');
  const sources = [[hub, journal.source.hub], ...Object.entries(journal.source.components).map(([name, source]) => {
    if (!source.directory || !path.isAbsolute(source.directory)) throw new Error(`${name}: missing recorded source directory`);
    return [source.directory, source];
  })];
  for (const [directory, expected] of sources) {
    const actual = captureSource(directory);
    if (actual.head !== expected.head || actual.tree !== expected.tree ||
        (expected.directory && actual.directory !== expected.directory)) throw new Error('recorded source HEAD/tree changed; recovery refused');
  }
  for (const name of LOCK_FILES) {
    loadImage(root, name, 'before');
    if (journal.locks[name]?.after) loadImage(root, name, 'after');
  }
  return journal;
}

function afterIdentityRecorded(after) {
  return after
    && after.sha256
    && after.dev != null
    && after.ino != null
    && after.mode != null
    && after.size != null;
}

function matchesRecordedIdentity(actual, recorded) {
  return actual.sha256 === recorded.sha256
    && actual.dev === recorded.dev
    && actual.ino === recorded.ino
    && actual.size === recorded.size
    && actual.mode === recorded.mode;
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
  if (record?.restored && matchesRecordedIdentity(actual, record.restored)) {
    return { class: 'original', identity: identityMeta(actual) };
  }
  if (before && actual.sha256 === before.sha256 && actual.dev === before.dev && actual.ino === before.ino
      && (before.mode == null || actual.mode === before.mode)) {
    return { class: 'original', identity: identityMeta(actual) };
  }
  if (after && actual.sha256 === after.sha256) {
    // Own after only when recorded after inode/dev/mode match — content alone is not ownership.
    if (afterIdentityRecorded(after) && matchesRecordedIdentity(actual, after)) {
      const image = loadImage(root, name, 'after');
      if (image.sha256 === actual.sha256) return { class: 'own-after', identity: identityMeta(actual) };
    }
    return { class: 'unknown-same-bytes', identity: identityMeta(actual) };
  }
  if (before && actual.sha256 === before.sha256 && (actual.dev !== before.dev || actual.ino !== before.ino
      || (before.mode != null && actual.mode !== before.mode))) {
    return { class: 'unknown-same-bytes', identity: identityMeta(actual) };
  }
  return { class: 'unknown', identity: identityMeta(actual) };
}

function recordWrittenAfter(tx, name, written, stage = 'replaced') {
  const current = readJournal(tx.root);
  current.locks[name].after = identityMeta(written);
  current.locks[name].stage = stage;
  tx.journal = writeJournal(tx.root, current, tx.observe);
  return written;
}

function replaceLock(tx, name, afterBytes, expectedPrior) {
  const target = path.join(tx.hub, name);
  assertIdentity(target, expectedPrior, name);
  markLockStage(tx, name, 'replacing');
  tx.observe?.('before-replace', { name });
  // Re-assert immediately before write so unique concurrent edits at before-replace are not overwritten.
  assertIdentity(target, expectedPrior, name);
  const bytes = Buffer.isBuffer(afterBytes) ? afterBytes : Buffer.from(afterBytes);
  const saved = loadImage(tx.root, name, 'after');
  if (digest(bytes) !== saved.sha256) throw new Error(`${name}: replacement differs from validated after image`);
  durableWriteFile(target, bytes, expectedPrior.mode & 0o777,
    prepared => recordWrittenAfter(tx, name, prepared, 'replacing'));
  const written = captureFileIdentity(target);
  if (written.sha256 !== digest(bytes)) throw new Error(`${name}: post-replace digest mismatch`);
  recordWrittenAfter(tx, name, written, 'replaced');
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
  const journal = validateRecoveryInputs(tx.hub, tx.root);
  setJournalState(tx, 'rolling-back');
  const restored = [];
  const preservedUnknown = [];
  const leftOriginal = [];
  for (const name of LOCK_FILES) {
    const classification = classifyLock(tx.hub, tx.root, name, journal.locks[name]);
    if (classification.class === 'own-after') {
      const before = loadImage(tx.root, name, 'before');
      const mode = journal.locks[name].before?.mode ?? 0o644;
      const afterRecord = journal.locks[name].after;
      if (!afterIdentityRecorded(afterRecord)) {
        throw new Error(`${name}: own-after missing recorded after inode/dev/mode`);
      }
      tx.observe?.('before-rollback-write', { name });
      // Re-assert owned after identity at write; unique concurrent edits stay put.
      assertIdentity(path.join(tx.hub, name), afterRecord, name);
      durableWriteFile(path.join(tx.hub, name), before.bytes, mode & 0o777, prepared => {
        const current = readJournal(tx.root);
        current.locks[name].restored = identityMeta(prepared);
        tx.journal = writeJournal(tx.root, current, tx.observe);
      });
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
  // Preserve complete journals, displaced evidence and unknown additions. Never recursively delete them.
  if (!fs.lstatSync(root).isDirectory()) throw new Error('operation root is not a regular directory');
  const history = path.join(path.dirname(root), 'family-operation-history');
  try { fs.mkdirSync(history, { mode: 0o700 }); } catch (error) { if (error.code !== 'EEXIST') throw error; }
  if (!fs.lstatSync(history).isDirectory()) throw new Error('operation history is not a regular directory');
  const destination = path.join(history, randomUUID());
  fs.renameSync(root, destination);
  syncDirectory(history);
  syncDirectory(path.dirname(root));
  return destination;
}

export function release(tx, { force = false } = {}) {
  if (!force && tx.retain) return false;
  const journal = safeReadJournal(tx.root);
  if (!journal && fs.existsSync(journalPath(tx.root))) return false;
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
    try { report.locks[name] = classifyLock(hub, root, name, journal.locks?.[name] ?? emptyLocks()[name]); }
    catch (error) { report.locks[name] = { class: 'unreadable', error: error.message }; }
  }
  if (report.writerStatus === 'live') report.reasons.push('recorded writer is live');
  if (report.writerStatus === 'foreign-host') report.reasons.push('recorded writer host differs');
  if (report.writerStatus === 'uncertain') report.reasons.push('writer liveness uncertain');
  let inputsValid = false;
  try { validateRecoveryInputs(hub, root); inputsValid = true; }
  catch (error) { report.reasons.push(error.message); }
  const classes = LOCK_FILES.map(name => report.locks[name].class);
  const onlyOwnOrOriginal = classes.every(c => c === 'original' || c === 'own-after');
  report.rollbackAdmissible = report.writerStatus === 'stopped' && inputsValid && onlyOwnOrOriginal
    && !['committed', 'rolled-back'].includes(journal.state);
  // Allow finish for naturally persisted mid-replace crashes (replacing / partial own-after+original),
  // not only the prepared checkpoint.
  const finishStates = new Set(['prepared', 'replacing', 'verifying', 'needs-recovery']);
  report.finishAdmissible = report.writerStatus === 'stopped' && inputsValid
    && finishStates.has(journal.state)
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
  const journal = validateRecoveryInputs(hub, report.operationRoot);
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
      const mode = (journal.locks[name].before?.mode ?? 0o644) & 0o777;
      const target = path.join(tx.hub, name);
      assertIdentity(target, journal.locks[name].before, name);
      durableWriteFile(target, after.bytes, mode,
        prepared => recordWrittenAfter(tx, name, prepared, 'replacing'));
      const written = captureFileIdentity(target);
      if (written.sha256 !== after.sha256) throw new Error(`${name}: finish digest mismatch`);
      recordWrittenAfter(tx, name, written, 'replaced');
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
