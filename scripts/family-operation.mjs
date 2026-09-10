// Durable paired-lock journal and recovery. No npm/TOML bootstrap.
// Atomic exchange and recovery exclusion use a standalone Rust helper compiled before package bootstrap.
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
const nativeSource = fileURLToPath(new URL('./family-native.rs', import.meta.url));
let nativeExecutable;
const nativeEnvironment = () => ({ PATH: '/usr/bin:/bin', LANG: 'C.UTF-8',
  GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', GIT_TERMINAL_PROMPT: '0', GIT_OPTIONAL_LOCKS: '0' });
function nativeCompiler() {
  const arch = { x64: 'x86_64', arm64: 'aarch64' }[process.arch];
  const platform = { linux: 'unknown-linux-gnu', darwin: 'apple-darwin' }[process.platform];
  if (!arch || !platform) throw new Error('native lock recovery is unsupported on this platform');
  // Resolve only account-owned or system installation locations. Repository
  // overrides (PATH, HOME, RUSTC, RUSTUP_HOME and RUSTUP_TOOLCHAIN) have no authority.
  const roots = [path.join(os.userInfo().homedir, '.rustup'), '/usr/local/rustup', '/opt/rustup', '/opt/hostedtoolcache/rustup'];
  for (const root of roots) {
    const compiler = path.join(root, 'toolchains', `1.97.1-${arch}-${platform}`, 'bin', 'rustc');
    if (!fs.existsSync(compiler)) continue;
    const identity = captureFileIdentity(compiler);
    const version = spawnSync(compiler, ['--version', '--verbose'], { env: nativeEnvironment(), encoding: 'utf8', timeout: 10000 });
    assertIdentity(compiler, identity, 'pinned Rust compiler');
    if (version.status !== 0 || !/^rustc 1\.97\.1 /.test(version.stdout ?? '') ||
        !version.stdout.includes(`host: ${arch}-${platform}\n`)) {
      throw new Error(`native lock recovery requires Rust1.97.1 for ${arch}-${platform}: ${compiler}`);
    }
    return { compiler, identity };
  }
  throw new Error('native lock recovery requires pinned Rust1.97.1 in an approved account or system rustup installation');
}
function nativeProgram() {
  if (nativeExecutable) {
    assertIdentity(nativeExecutable.path, nativeExecutable.identity, 'compiled recovery helper');
    assertIdentity(nativeSource, nativeExecutable.sourceIdentity, 'native helper source');
    return nativeExecutable.path;
  }
  const { compiler, identity: compilerIdentity } = nativeCompiler();
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-family-native-'));
  const source = path.join(directory, 'helper.rs'), executable = path.join(directory, 'helper');
  const original = captureFileIdentity(nativeSource);
  writeExclusive(source, original.bytes, 0o600);
  const result = spawnSync(compiler, ['--edition=2024', '-Dwarnings', source, '-o', executable], {
    env: nativeEnvironment(), encoding: 'utf8', timeout: 30000, maxBuffer: 1024 * 1024,
  });
  if (result.status !== 0) throw new Error(`native recovery helper compilation failed; retained ${directory}: ${result.stderr ?? result.error}`);
  assertIdentity(compiler, compilerIdentity, 'pinned Rust compiler');
  assertIdentity(nativeSource, original, 'native helper source');
  const identities = [source, executable].map(captureFileIdentity);
  if (!identities[1].size || !(identities[1].mode & 0o111)) throw new Error(`native recovery helper is not executable; retained ${directory}`);
  const fd = fs.openSync(executable, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  syncDirectory(directory);
  process.once('exit', () => {
    // Delete only this process's unchanged private build files; retain unknown additions.
    try {
      for (const [i, file] of [source, executable].entries()) if (sameIdentity(file, identities[i])) fs.unlinkSync(file);
      fs.rmdirSync(directory);
    } catch { /* preserve unknown or failed cleanup evidence */ }
  });
  nativeExecutable = { path: executable, identity: identities[1], sourceIdentity: original };
  return executable;
}
function native(args, lease) {
  const executable = nativeProgram();
  const result = spawnSync(executable, args, {
    env: nativeEnvironment(), encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, timeout: args[0] === 'recover' ? 60000 : 10000,
    stdio: lease === undefined ? ['ignore', 'pipe', 'pipe'] : ['ignore', 'pipe', 'pipe', lease],
  });
  assertIdentity(executable, nativeExecutable.identity, 'compiled recovery helper');
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`family native operation failed: ${result.stderr}`);
  return result.stdout;
}
const swapPath = (root, name) => path.join(journalDir(root), 'swaps', name);
function atomicExchange(left, right) {
  native(['exchange', left, right]);
}
function preservingExchange(tx, name, target, swap, incoming, outgoing, direction) {
  tx.observe?.('before-atomic-exchange', { name, direction });
  atomicExchange(target, swap);
  tx.observe?.('after-atomic-exchange', { name, direction });
  if (!sameIdentity(swap, outgoing) && sameIdentity(target, incoming)) {
    // Restore an edit displaced after the last assertion. Exchange preserves
    // both names even if a second edit races this restoration.
    atomicExchange(target, swap);
    throw new Error(`${name}: concurrent edit preserved at exchange boundary`);
  }
  assertIdentity(target, incoming, `${name} installed identity`);
  assertIdentity(swap, outgoing, `${name} displaced identity`);
}
function lockedRecovery(hub, command) {
  const lock = path.join(path.dirname(operationRoot(hub)), 'family-recovery.lock');
  return JSON.parse(native(['recover', lock, process.execPath, fileURLToPath(import.meta.url), fs.realpathSync(hub), command]));
}


function gitText(directory, ...args) {
  const result = spawnSync('/usr/bin/git', ['-c', 'core.fsmonitor=false', '-C', directory, ...args], {
    encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
    env: nativeEnvironment(), timeout: 10000,
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
    directory: fs.realpathSync(directory),
    head: gitText(directory, 'rev-parse', 'HEAD'),
    tree: gitText(directory, 'rev-parse', 'HEAD^{tree}'),
  };
}

export function captureFileIdentity(file) {
  // Open once without following a final symlink or blocking on a FIFO. Bound
  // reads even if another process extends a regular file during capture.
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK);
  try {
    const st = fs.fstatSync(fd);
    const limit = 64 * 1024 * 1024;
    if (!st.isFile() || st.size > limit) throw new Error(`${file}: refusing non-regular or oversized lock path`);
    const bytes = Buffer.alloc(st.size);
    let offset = 0;
    while (offset < bytes.length) {
      const read = fs.readSync(fd, bytes, offset, bytes.length - offset, offset);
      if (!read) throw new Error(`${file}: file changed during capture`);
      offset += read;
    }
    const after = fs.fstatSync(fd), named = fs.lstatSync(file);
    if (after.size !== st.size || after.mtimeMs !== st.mtimeMs || after.ctimeMs !== st.ctimeMs ||
        after.mode !== st.mode || named.dev !== st.dev || named.ino !== st.ino || !named.isFile()) {
      throw new Error(`${file}: file identity changed during capture`);
    }
    return { sha256: digest(bytes), mode: st.mode, size: st.size, dev: st.dev, ino: st.ino, bytes };
  } finally { fs.closeSync(fd); }
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
  const startKind = process.platform === 'darwin' ? 'darwin-proc-starttime' : 'linux-proc-startticks';
  try {
    startTime = process.platform === 'darwin'
      ? JSON.parse(native(['darwin-process', String(pid)])).startTime
      : processStartTicks(pid);
  } catch { /* no invented fallback identity: recovery must fail closed */ }
  return { pid, hostname: os.hostname(), startKind, startTime, startedAt: new Date().toISOString() };
}

/** @returns {'live'|'stopped'|'uncertain'|'foreign-host'} */
export function writerStatus(writer) {
  if (!Number.isInteger(writer?.pid) || writer.pid <= 0) return 'uncertain';
  if (writer.hostname && writer.hostname !== os.hostname()) return 'foreign-host';
  if (process.platform !== 'darwin' && !procfsUsable()) return 'uncertain';
  try {
    const expectedKind = process.platform === 'darwin' ? 'darwin-proc-starttime' : 'linux-proc-startticks';
    const identity = process.platform === 'darwin'
      ? JSON.parse(native(['darwin-process', String(writer.pid)]))
      : { startTime: processStartTicks(writer.pid) };
    if (identity.stopped) return 'stopped';
    // Old journals lacking a kernel identity kind cannot authorize mutation of
    // an extant process: their startTime may have been a wall-clock fallback.
    if (writer.startKind !== expectedKind || writer.startTime == null) return 'uncertain';
    return String(writer.startTime) === String(identity.startTime) ? 'live' : 'stopped';
  } catch (error) {
    if (process.platform !== 'darwin' && error?.code === 'ENOENT') return 'stopped';
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
  writeExclusive(temporary, raw, mode);
  prepared?.(captureFileIdentity(temporary));
  fs.renameSync(temporary, file);
  syncDirectory(path.dirname(file));
  // On any failure, retain the temporary inode instead of deleting unknown edits.
}

function readJournal(root) {
  assertRecoveryDirectories(root);
  const file = journalPath(root);
  if (!fs.existsSync(file)) return null;
  const data = JSON.parse(captureFileIdentity(file).bytes.toString('utf8'));
  if (data?.schema !== JOURNAL_SCHEMA) throw new Error(`unsupported journal schema ${data?.schema}`);
  return data;
}

function writeJournal(root, data, observe) {
  assertRecoveryDirectories(root);
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
    hub: fs.realpathSync(hub),
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
    const excluded = directory === hub ? LOCK_FILES.map(name => `:(exclude)${name}`) : [];
    if (gitText(directory, 'status', '--porcelain=v1', '--untracked-files=all', '--', '.', ...excluded)) {
      throw new Error('recorded source has concurrent edits; recovery refused');
    }
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
  const target = path.join(tx.hub, name), swap = swapPath(tx.root, name);
  assertIdentity(target, expectedPrior, name);
  const bytes = Buffer.isBuffer(afterBytes) ? afterBytes : Buffer.from(afterBytes);
  const saved = loadImage(tx.root, name, 'after');
  if (digest(bytes) !== saved.sha256) throw new Error(`${name}: replacement differs from validated after image`);
  fs.mkdirSync(path.dirname(swap), { recursive: true, mode: 0o700 });
  const prior = readJournal(tx.root).locks[name].after;
  if (fs.existsSync(swap)) assertIdentity(swap, prior, `${name} prepared replacement`);
  else {
    writeExclusive(swap, bytes, expectedPrior.mode & 0o777);
    syncDirectory(path.dirname(swap));
    recordWrittenAfter(tx, name, captureFileIdentity(swap), 'replacing');
  }
  const intended = readJournal(tx.root).locks[name].after;
  tx.observe?.('before-replace', { name });
  assertIdentity(target, expectedPrior, name);
  assertIdentity(swap, intended, `${name} prepared replacement`);
  // Both names survive the atomic swap, including any post-check concurrent edit.
  preservingExchange(tx, name, target, swap, intended, expectedPrior, 'replace');
  markLockStage(tx, name, 'replaced');
  const written = captureFileIdentity(target);
  tx.observe?.('after-replace', { name, identity: identityMeta(written) });
  return written;
}

export function commitPairedReplace(tx, afterBytes, beforeIdentities, { observe } = {}) {
  if (observe) tx.observe = observe;
  validateRecoveryInputs(tx.hub, tx.root);
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
      assertIdentity(path.join(tx.hub, name), tx.journal.locks[name].after, `${name} final identity`);
      assertIdentity(swapPath(tx.root, name), beforeIdentities[name], `${name} displaced original`);
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
      const afterRecord = journal.locks[name].after;
      if (!afterIdentityRecorded(afterRecord)) throw new Error(`${name}: missing owned after identity`);
      const swap = swapPath(tx.root, name), target = path.join(tx.hub, name);
      let restoredIdentity = journal.locks[name].restored ?? journal.locks[name].before;
      if (!fs.existsSync(swap)) {
        // Legacy journals may lack the original inode. Preserve their current
        // destination by exchanging it with a validated before-image copy.
        fs.mkdirSync(path.dirname(swap), { recursive: true, mode: 0o700 });
        writeExclusive(swap, before.bytes, restoredIdentity.mode & 0o777);
        syncDirectory(path.dirname(swap));
        restoredIdentity = identityMeta(captureFileIdentity(swap));
        const current = readJournal(tx.root);
        current.locks[name].restored = restoredIdentity;
        tx.journal = writeJournal(tx.root, current, tx.observe);
      }
      tx.observe?.('before-rollback-write', { name });
      assertIdentity(target, afterRecord, name);
      // If the forward swap displaced an unknown edit, restore that exact inode
      // to its original name. A mismatch then retains the journal for inspection.
      preservingExchange(tx, name, target, swap, restoredIdentity, afterRecord, 'rollback');
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

function assertRecoveryDirectories(root) {
  for (const directory of [root, journalDir(root), path.join(journalDir(root), 'images'), path.join(journalDir(root), 'swaps')]) {
    try {
      if (!fs.lstatSync(directory).isDirectory()) throw new Error(`unsafe recovery directory: ${directory}`);
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
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
    recoveryWriter: null,
    recoveryWriterStatus: null,
    locks: {},
    finishAdmissible: false,
    rollbackAdmissible: false,
    reasons: [],
  };
  if (!present) {
    report.reasons.push('no family-operation directory');
    return report;
  }
  try { assertRecoveryDirectories(root); }
  catch (error) { report.reasons.push(error.message); return report; }
  try {
    report.writer = JSON.parse(captureFileIdentity(path.join(root, 'owner.json')).bytes.toString('utf8'));
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
  report.recoveryWriter = journal.recoveryWriter ?? null;
  report.recoveryWriterStatus = report.recoveryWriter ? writerStatus(report.recoveryWriter) : 'stopped';
  const recoveryAvailable = report.recoveryWriterStatus === 'stopped' ||
    (report.recoveryWriter?.pid === process.pid && process.env.JANKURAI_RECOVERY_FD);
  if (!recoveryAvailable) report.reasons.push('another recovery writer is active or uncertain');
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
  report.rollbackAdmissible = report.writerStatus === 'stopped' && recoveryAvailable && inputsValid && onlyOwnOrOriginal
    && !['committed', 'rolled-back'].includes(journal.state);
  // Allow finish for naturally persisted mid-replace crashes (replacing / partial own-after+original),
  // not only the prepared checkpoint.
  const finishStates = new Set(['prepared', 'replacing', 'verifying', 'needs-recovery']);
  report.finishAdmissible = report.writerStatus === 'stopped' && recoveryAvailable && inputsValid
    && finishStates.has(journal.state)
    && classes.every(c => c === 'original' || c === 'own-after')
    && journal.locks
    && LOCK_FILES.every(name => journal.locks[name]?.after && journal.locks[name]?.before);
  if (!report.rollbackAdmissible && report.writerStatus === 'stopped') {
    if (!onlyOwnOrOriginal) report.reasons.push('unknown lock edits block automatic rollback');
  }
  return report;
}

export function inspect(hub) {
  const report = buildReport(hub);
  try {
    const { compiler, identity } = nativeCompiler();
    report.nativeCapability = { available: true, compiler, compilerSha256: identity.sha256 };
  } catch (error) {
    report.nativeCapability = { available: false, reason: error.message };
    report.finishAdmissible = false;
    report.rollbackAdmissible = false;
    report.reasons.push(error.message);
  }
  return report;
}

function refuseLive(report) {
  if (!report.journal) {
    throw new Error(`recovery admission failed: ${report.reasons.join('; ') || 'journal unavailable'}`);
  }
  if (report.recoveryWriterStatus !== 'stopped' && report.recoveryWriter?.pid !== process.pid) {
    throw new Error('refusing recovery while another recovery writer is active or uncertain');
  }
  if (report.writerStatus === 'live') throw new Error('refusing recovery while recorded writer is live');
  if (report.writerStatus === 'foreign-host') throw new Error('refusing recovery for foreign-host writer');
  if (report.writerStatus === 'uncertain') throw new Error('refusing recovery while writer liveness is uncertain');
}

function rollbackUnlocked(hub, { observe } = {}) {
  const report = buildReport(hub);
  if (!report.present) throw new Error('no family-operation to roll back');
  refuseLive(report);
  const tx = {
    hub: fs.realpathSync(hub),
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

function finishUnlocked(hub, { observe } = {}) {
  const report = buildReport(hub);
  if (!report.present) throw new Error('no family-operation to finish');
  refuseLive(report);
  const journal = validateRecoveryInputs(hub, report.operationRoot);
  if (journal.state === 'committed') {
    if (!LOCK_FILES.every(name => report.locks[name].class === 'own-after')) {
      throw new Error('committed locks changed; preserve journal for inspection');
    }
    releaseRoot(report.operationRoot);
    return { state: 'committed', cleaned: true };
  }
  if (!report.finishAdmissible) {
    throw new Error(`finish not admissible: ${(report.reasons.join('; ') || journal.state)}`);
  }
  const tx = {
    hub: fs.realpathSync(hub),
    root: report.operationRoot,
    writer: report.writer,
    journal,
    observe: observe ?? (() => {}),
    retain: false,
  };
  for (const name of LOCK_FILES) {
    const swap = swapPath(tx.root, name);
    if (report.locks[name].class === 'own-after' && fs.existsSync(swap)) {
      assertIdentity(swap, journal.locks[name].before, `${name} displaced original`);
    }
  }
  for (const name of LOCK_FILES) {
    const classification = report.locks[name];
    if (classification.class === 'original') {
      const after = loadImage(tx.root, name, 'after');
      replaceLock(tx, name, after.bytes, journal.locks[name].before);
    } else if (classification.class !== 'own-after') {
      throw new Error(`${name}: finish refused for class ${classification.class}`);
    }
  }
  for (const name of LOCK_FILES) {
    const swap = swapPath(tx.root, name);
    const current = readJournal(tx.root).locks[name];
    assertIdentity(path.join(hub, name), current.after, `${name} finished identity`);
    if (fs.existsSync(swap)) assertIdentity(swap, current.before, `${name} displaced original`);
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


export const rollback = hub => lockedRecovery(hub, 'rollback');
export const finish = hub => lockedRecovery(hub, 'finish');

if (process.argv[1] === fileURLToPath(import.meta.url) && process.argv[2] === '--recovery-worker') {
  try {
    const [hub, command] = process.argv.slice(3);
    if (!['finish', 'rollback'].includes(command)) throw new Error('invalid recovery command');
    const fd = Number(process.env.JANKURAI_RECOVERY_FD);
    if (!Number.isInteger(fd) || fd < 3 || process.ppid !== Number(process.env.JANKURAI_RECOVERY_PARENT)) {
      throw new Error('recovery worker requires its owning lock supervisor');
    }
    const held = fs.fstatSync(fd), current = fs.lstatSync(path.join(path.dirname(operationRoot(hub)), 'family-recovery.lock'));
    if (!held.isFile() || !current.isFile() || held.dev !== current.dev || held.ino !== current.ino) {
      throw new Error('recovery lock identity changed');
    }
    native(['validate-lease', path.join(path.dirname(operationRoot(hub)), 'family-recovery.lock')], fd);
    const report = buildReport(hub);
    if (!report.present) throw new Error('no family-operation to recover');
    refuseLive(report);
    const journal = readJournal(report.operationRoot);
    journal.recoveryWriter = writerIdentity();
    writeJournal(report.operationRoot, journal);
    const result = command === 'finish' ? finishUnlocked(hub) : rollbackUnlocked(hub);
    process.stdout.write(`${JSON.stringify(result)}\n`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
