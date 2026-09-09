#!/usr/bin/env node
// Publish the complete verified capture; retain a durable journal on interruption.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { sha256 } from './audit-recording.mjs';
import { PUBLIC_FILES, CAPTURE_FILES, MAX_BYTES } from './demo-catalog.mjs';

function plainDirectories(directory) {
  for (let current = path.resolve(directory); ; current = path.dirname(current)) {
    const stat = fs.lstatSync(current);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('refusing redirected publication directory');
    if (current === path.dirname(current)) break;
  }
}
function readFile(file) {
  plainDirectories(path.dirname(file));
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  try {
    const before = fs.fstatSync(fd);
    if (!before.isFile() || before.nlink !== 1 || before.size >= MAX_BYTES) throw new Error('refusing nonregular/linked/oversized publication input');
    const raw = fs.readFileSync(fd), after = fs.fstatSync(fd), entry = fs.lstatSync(file);
    if (after.size !== before.size || after.mtimeMs !== before.mtimeMs || after.ctimeMs !== before.ctimeMs
        || entry.dev !== before.dev || entry.ino !== before.ino || entry.isSymbolicLink()) throw new Error('publication input changed while read');
    return { raw, sha256: sha256(raw), dev: before.dev, ino: before.ino };
  } finally { fs.closeSync(fd); }
}
function sameFile(file, expected) {
  if (expected === null) {
    try { fs.lstatSync(file); } catch (error) { if (error.code === 'ENOENT') return; throw error; }
    throw new Error('preserving unexpected publication destination');
  }
  const actual = readFile(file);
  if (actual.sha256 !== expected.sha256 || actual.dev !== expected.dev || actual.ino !== expected.ino) throw new Error('preserving changed publication input');
}
function syncDirectory(directory) {
  const fd = fs.openSync(directory, fs.constants.O_RDONLY);
  try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}
function writeExclusive(file, raw) {
  const fd = fs.openSync(file, fs.constants.O_WRONLY | fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_NOFOLLOW, 0o600);
  try { fs.writeFileSync(fd, raw); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
}
function defaultVerify(directory) {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('./verify-audit-gif.mjs', import.meta.url)), directory], { stdio: 'inherit', timeout: 120000 });
  if (result.status !== 0 || result.signal || result.error) throw new Error('refusing an unverified complete capture');
}

export function publishDemo(source, destination, { verify = defaultVerify, observe = () => {} } = {}) {
  const rendered = path.resolve(source), catalog = path.resolve(destination);
  plainDirectories(rendered); plainDirectories(catalog);
  if (rendered === catalog || rendered.startsWith(catalog + path.sep) || catalog.startsWith(rendered + path.sep)) throw new Error('publication source/destination overlap');
  const lock = path.join(catalog, '.audit-demo-publish.lock');
  fs.mkdirSync(lock, { mode: 0o700 }); // Never reclaim or erase an existing operation.
  const lockIdentity = fs.lstatSync(lock);
  const journal = fs.mkdtempSync(path.join(path.dirname(catalog), '.audit-demo-publication-'));
  for (const name of ['candidate', 'before', 'retained']) fs.mkdirSync(path.join(journal, name));
  const events = path.join(journal, 'events.jsonl');
  writeExclusive(events, '');
  const log = event => {
    const fd = fs.openSync(events, fs.constants.O_WRONLY | fs.constants.O_APPEND | fs.constants.O_NOFOLLOW);
    try { fs.writeFileSync(fd, JSON.stringify(event) + '\n'); fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
  };
  const ownership = JSON.stringify({ schema: 1, pid: process.pid, journal, catalog }) + '\n';
  writeExclusive(path.join(lock, 'operation.json'), ownership); syncDirectory(lock); syncDirectory(catalog);
  const ownedRecord = readFile(path.join(lock, 'operation.json'));
  const snapshots = [];
  try {
    for (const name of PUBLIC_FILES) {
      const input = CAPTURE_FILES[name] ? path.join(path.dirname(rendered), 'recording', CAPTURE_FILES[name]) : path.join(rendered, name);
      const selected = readFile(input), target = path.join(catalog, name);
      let before;
      try { before = readFile(target); } catch (error) { if (error.code === 'ENOENT') before = null; else throw error; }
      writeExclusive(path.join(journal, 'candidate', name), selected.raw);
      if (before) writeExclusive(path.join(journal, 'before', name), before.raw);
      snapshots.push({ name, input, selected, before });
    }
    const receipt = { schema: 1, source: rendered, catalog, files: snapshots.map(({ name, input, selected, before }) => ({ name, input, afterSha256: selected.sha256, beforeSha256: before?.sha256 ?? null })) };
    writeExclusive(path.join(journal, 'operation.json'), JSON.stringify(receipt, null, 2) + '\n');
    for (const name of ['candidate', 'before', 'retained']) syncDirectory(path.join(journal, name));
    syncDirectory(journal); syncDirectory(path.dirname(journal));
    verify(path.join(journal, 'candidate')); observe('verified', { journal, catalog });
    for (const item of snapshots) { sameFile(item.input, item.selected); sameFile(path.join(catalog, item.name), item.before); }
    log({ phase: 'prepared' });
    for (const item of snapshots) {
      const target = path.join(catalog, item.name);
      sameFile(target, item.before);
      if (item.before) {
        fs.renameSync(target, path.join(journal, 'retained', item.name));
        syncDirectory(catalog); syncDirectory(path.join(journal, 'retained'));
        sameFile(path.join(journal, 'retained', item.name), item.before);
      }
      observe('before-install', { journal, catalog, name: item.name });
      fs.copyFileSync(path.join(journal, 'candidate', item.name), target, fs.constants.COPYFILE_EXCL);
      const fd = fs.openSync(target, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
      try { fs.fsyncSync(fd); } finally { fs.closeSync(fd); }
      syncDirectory(catalog); log({ phase: 'installed', name: item.name });
      observe('installed', { journal, catalog, name: item.name });
    }
    for (const item of snapshots) {
      sameFile(item.input, item.selected);
      if (readFile(path.join(catalog, item.name)).sha256 !== item.selected.sha256) throw new Error('preserving modified published output');
      if (item.before) sameFile(path.join(journal, 'retained', item.name), item.before);
    }
    sameFile(path.join(lock, 'operation.json'), ownedRecord);
    const lockNow = fs.lstatSync(lock);
    if (lockNow.dev !== lockIdentity.dev || lockNow.ino !== lockIdentity.ino || fs.readdirSync(lock).length !== 1) throw new Error('preserving changed publication lock');
    log({ phase: 'complete' });
    fs.unlinkSync(path.join(lock, 'operation.json')); fs.rmdirSync(lock); syncDirectory(catalog);
    return { journal, files: PUBLIC_FILES };
  } catch (error) {
    log({ phase: 'interrupted', error: String(error.message) });
    throw new Error(`publication stopped; preserve lock and journal for review/recovery: ${journal}: ${error.message}`, { cause: error });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [source, destination] = process.argv.slice(2);
  if (!source || !destination) throw new Error('usage: publish-demo.mjs RENDERED-DIR DOCS-DEMO-DIR');
  const result = publishDemo(source, destination);
  console.log(`published complete capture; retained before-images and journal: ${result.journal}`);
}
