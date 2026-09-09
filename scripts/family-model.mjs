import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { readToml, exists, isLink, gitText, git, clean, atomicWrite, run, buildEnvironment } from './family-lib.mjs';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');

export class Family {
  constructor(hub) {
    this.hub = fs.realpathSync(hub);
    this.root = path.dirname(this.hub);
    this.manifest = readToml(path.join(this.hub, 'repos.manifest.toml'));
    this.lock = readToml(path.join(this.hub, 'family.lock'));
    this.repos = this.manifest.repo;
    this.pins = new Map(this.lock.repo.map(pin => [pin.repo, pin]));
    this.fusion = path.join(this.hub, '.fusion');
    this.validate();
  }
  validate() {
    const m = this.manifest;
    if (m.schema_version !== '2.0.0' || m.authority_forge !== 'github') throw new Error('expected GitHub family schema 2.0.0');
    const names = this.repos.map(repo => repo.name);
    if (new Set(names).size !== names.length || names.length !== m.expected_repo_count ||
        JSON.stringify([...names].sort()) !== JSON.stringify([...m.required_repos].sort())) throw new Error('duplicate or missing family repository');
    const components = names.filter(name => name !== 'jankurai').sort();
    if (this.pins.size !== this.lock.repo.length || JSON.stringify([...this.pins.keys()].sort()) !== JSON.stringify(components)) throw new Error('lock must pin each component exactly once');
    for (const repo of this.repos) this.validateRepo(repo);
  }
  validateRepo(repo) {
    if (!/^jankurai(?:-[a-z]+)*$/.test(repo.name)) throw new Error('invalid repository name');
    const url = `https://github.com/${this.manifest.public_owner}/${repo.name}.git`;
    if (repo.path !== repo.name || repo.github !== url || repo.hosted !== url) throw new Error(`${repo.name}: expected relative canonical path and GitHub URL`);
    if (repo.required_check !== `${repo.name}/required` || repo.default_branch !== 'main') throw new Error('invalid branch/check contract');
    if ('tag' in repo || 'commit' in repo) throw new Error('component revision pins belong only in family.lock');
    const pin = this.pins.get(repo.name);
    if (pin && (!/^[a-f0-9]{40}$/.test(pin.commit) || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(pin.tag))) throw new Error('malformed immutable pin');
  }
  components() { return this.repos.filter(repo => repo.name !== 'jankurai'); }
  path(repo) { return repo.name === 'jankurai' ? this.hub : path.join(this.root, repo.path); }
  existing(repo) {
    const directory = this.path(repo);
    if (!exists(directory) && !isLink(directory)) return false;
    const gd = path.join(directory, '.git');
    if (isLink(directory) || isLink(gd) || !exists(gd) || !fs.statSync(gd).isDirectory() ||
        gitText(directory, 'rev-parse', '--show-toplevel') !== directory) throw new Error(`${directory}: expected a canonical primary checkout`);
    return true;
  }
  fetchPin(repo) {
    const pin = this.pins.get(repo.name), directory = this.path(repo);
    git(directory, ['fetch', '--no-tags', repo.github, `refs/tags/${pin.tag}:refs/tags/${pin.tag}`], { env: buildEnvironment() });
    if (gitText(directory, 'rev-parse', `refs/tags/${pin.tag}^{commit}`) !== pin.commit) throw new Error(`${repo.name}: immutable tag differs from lock`);
  }
  bootstrap(restore = false) {
    const existing = this.components().filter(repo => this.existing(repo));
    if (restore) for (const repo of existing) clean(this.path(repo));
    for (const repo of this.components()) {
      const directory = this.path(repo), pin = this.pins.get(repo.name);
      if (!exists(directory)) {
        run(['git', 'clone', '--no-checkout', '--origin', 'origin', repo.github, directory], { env: buildEnvironment() });
        this.fetchPin(repo);
        git(directory, ['checkout', '--detach', pin.commit]);
      } else if (restore) this.fetchPin(repo);
    }
    if (restore) this.restore(existing);
  }
  restore(repos) {
    for (const repo of repos) {
      const directory = this.path(repo), pin = this.pins.get(repo.name), head = gitText(directory, 'rev-parse', 'HEAD');
      if (head !== pin.commit && git(directory, ['merge-base', '--is-ancestor', head, pin.commit], { check: false }).status !== 0) throw new Error(`${repo.name}: restoring lock would discard ahead/divergent commits`);
    }
    for (const repo of repos) {
      const directory = this.path(repo), pin = this.pins.get(repo.name);
      clean(directory);
      if (gitText(directory, 'rev-parse', 'HEAD') !== pin.commit) git(directory, ['checkout', '--detach', pin.commit]);
    }
  }
  fuse(copyLock = true, isolate = false) {
    const links = this.ownedComponentRoot(isolate), members = [], patches = new Map();
    for (const repo of this.components()) {
      const directory = this.path(repo), link = path.join(links, repo.name);
      if (!this.existing(repo)) throw new Error(`missing component: ${repo.name}`);
      if (isolate) this.materializeIsolate(directory, link);
      else if (isLink(link)) {
        if (fs.realpathSync(link) !== directory) throw new Error(`refusing mismatched link: ${link}`);
      } else if (exists(link) && isolateMarker(link)) {
        assertExactOwnedIsolate(link, links, directory);
        removeOwned(link, links);
        fs.symlinkSync(path.relative(links, directory), link, 'dir');
      } else if (exists(link)) throw new Error(`refusing to overwrite directory: ${link}`);
      else fs.symlinkSync(path.relative(links, directory), link, 'dir');
      if (!exists(path.join(directory, 'Cargo.toml'))) continue;
      for (const member of readToml(path.join(directory, 'Cargo.toml')).workspace.members) {
        const relative = `components/${repo.name}/${member}`;
        const name = readToml(path.join(directory, member, 'Cargo.toml')).package.name;
        members.push(relative);
        if (!patches.has(repo.github)) patches.set(repo.github, []);
        patches.get(repo.github).push([name, relative]);
      }
    }
    let cargo = `[workspace]\nresolver = "2"\nmembers = ${JSON.stringify(members, null, 2)}\n`;
    for (const [url, entries] of patches) {
      cargo += `\n[patch.${JSON.stringify(url)}]\n`;
      for (const [name, relative] of entries) cargo += `${name} = { path = ${JSON.stringify(relative)} }\n`;
    }
    atomicWrite(path.join(this.fusion, 'Cargo.toml'), cargo);
    if (copyLock) fs.copyFileSync(path.join(this.hub, 'Cargo.lock'), path.join(this.fusion, 'Cargo.lock'));
    atomicWrite(path.join(this.fusion, 'dev.sh'), '#!/usr/bin/env bash\nset -euo pipefail\nexec bash "$(dirname "${BASH_SOURCE[0]}")/../scripts/family.sh" "${@:-build}"\n');
    fs.chmodSync(path.join(this.fusion, 'dev.sh'), 0o755);
  }
  ownedComponentRoot(isolate) {
    if (isLink(this.fusion)) throw new Error('refusing symlinked .fusion');
    fs.mkdirSync(this.fusion, { recursive: true });
    if (isLink(this.fusion)) throw new Error('refusing symlinked .fusion');
    const links = path.join(this.fusion, 'components');
    if (isLink(links)) throw new Error('refusing symlinked .fusion/components');
    fs.mkdirSync(links, { recursive: true });
    if (isLink(links)) throw new Error('refusing symlinked .fusion/components');
    if (isolate && fs.realpathSync(links) !== links) throw new Error('refusing redirected .fusion/components');
    return links;
  }
  ownedRequiredRoot() {
    if (isLink(this.fusion)) throw new Error('refusing symlinked .fusion');
    const required = path.join(this.fusion, 'required-components');
    if (isLink(required)) throw new Error('refusing symlinked .fusion/required-components');
    fs.mkdirSync(required, { recursive: true });
    if (isLink(required) || fs.realpathSync(required) !== required) throw new Error('refusing redirected .fusion/required-components');
    return required;
  }
  materializeIsolate(directory, dest) {
    const parent = path.dirname(dest);
    if (isLink(parent) || fs.realpathSync(parent) !== parent) throw new Error(`refusing redirected isolate parent: ${parent}`);
    if (isLink(dest)) {
      if (fs.realpathSync(dest) !== directory) throw new Error(`refusing to replace symlink: ${dest}`);
      fs.unlinkSync(dest);
    }
    if (exists(dest) && !isolateMarker(dest)) throw new Error(`refusing to overwrite directory: ${dest}`);
    if (exists(dest)) {
      assertExactOwnedIsolate(dest, parent, directory);
      removeOwned(dest, parent);
    }
    const commit = gitText(directory, 'rev-parse', 'HEAD');
    const tree = gitText(directory, 'rev-parse', `${commit}^{tree}`);
    if (!/^[a-f0-9]{40}$/.test(commit) || !/^[a-f0-9]{40}$/.test(tree)) throw new Error('unfrozen isolate revision');
    fs.mkdirSync(dest, { recursive: true });
    const work = fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-isolate-'));
    try {
      const gitDir = gitText(directory, 'rev-parse', '--absolute-git-dir');
      if (isLink(gitDir) || isLink(path.join(directory, '.git'))) throw new Error('refusing symlinked git dir');
      const index = path.join(work, 'index');
      const env = { ...process.env, GIT_INDEX_FILE: index, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' };
      run(['git', `--git-dir=${gitDir}`, `--work-tree=${dest}`, 'read-tree', commit], { env });
      run(['git', `--git-dir=${gitDir}`, `--work-tree=${dest}`, 'checkout-index', '--all', '--quiet'], { env });
      if (gitText(directory, 'rev-parse', `${commit}^{tree}`) !== tree) throw new Error('isolate source tree changed during materialization');
      const files = isolateInventory(dest);
      assertCommittedInventory(directory, commit, files);
      writeIsolateMarker(dest, { kind: 'owned-execution-copy', source: directory, commit, tree, files });
    } catch (error) {
      throw error;
    } finally {
      fs.rmSync(work, { recursive: true, force: true });
    }
  }
  rematerializeIsolates() {
    const required = this.ownedRequiredRoot();
    for (const repo of this.components()) {
      if (!this.existing(repo)) throw new Error(`missing component: ${repo.name}`);
      this.materializeIsolate(this.path(repo), path.join(required, repo.name));
    }
  }
  disposeIsolates() {
    for (const rootName of ['components', 'required-components']) {
      const links = path.join(this.fusion, rootName);
      if (!exists(links) || isLink(links)) continue;
      for (const repo of this.components()) {
        const dest = path.join(links, repo.name);
        if (!exists(dest) || !isolateMarker(dest)) continue;
        try { assertExactOwnedIsolate(dest, links, this.path(repo)); }
        catch { continue; }
        removeOwned(dest, links);
      }
    }
  }
  executionPath(repo) {
    const live = this.path(repo);
    for (const rootName of ['required-components', 'components']) {
      const links = path.join(this.fusion, rootName);
      const isolated = path.join(links, repo.name);
      if (!exists(isolated) || !isolateMarker(isolated)) continue;
      if (isLink(links)) throw new Error(`refusing redirected isolate root: ${links}`);
      assertExactOwnedIsolate(isolated, links, live);
      return isolated;
    }
    if (this.allowLiveRequired) return live;
    throw new Error(`missing isolated execution copy: ${repo.name}`);
  }
  status() {
    for (const repo of this.repos) {
      if (!this.existing(repo)) { console.log(`${repo.name}: missing`); continue; }
      const directory = this.path(repo), head = gitText(directory, 'rev-parse', 'HEAD');
      const branch = gitText(directory, 'branch', '--show-current') || 'detached', pin = this.pins.get(repo.name);
      console.log(`${repo.name}: ${branch} ${head.slice(0, 12)} (${!pin ? 'hub' : head === pin.commit ? 'locked' : `differs from lock ${pin.commit.slice(0, 12)}`})`);
      const dirty = gitText(directory, 'status', '--short');
      if (dirty) console.log(dirty);
    }
  }
}

function isolateMarkPath(directory) {
  return `${directory}.jankurai-isolate`;
}

function isolateMarker(directory) {
  return exists(isolateMarkPath(directory)) && !isLink(isolateMarkPath(directory));
}

function readIsolateMarker(directory) {
  const mark = isolateMarkPath(directory);
  if (isLink(mark)) throw new Error(`refusing symlinked isolate marker: ${mark}`);
  const record = JSON.parse(fs.readFileSync(mark, 'utf8'));
  if (record?.kind !== 'owned-execution-copy' || !Array.isArray(record.files)) {
    throw new Error(`invalid isolate ownership record: ${mark}`);
  }
  return record;
}

function writeIsolateMarker(directory, record) {
  const mark = isolateMarkPath(directory);
  if (exists(mark) || isLink(mark)) throw new Error(`refusing to replace isolate marker: ${mark}`);
  const fd = fs.openSync(mark, fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_WRONLY | fs.constants.O_NOFOLLOW);
  try {
    fs.writeFileSync(fd, JSON.stringify(record) + '\n');
  } finally {
    fs.closeSync(fd);
  }
}

function containedLexical(root, candidate) {
  const rel = path.relative(path.resolve(root), path.resolve(candidate));
  return rel !== '..' && !rel.startsWith(`..${path.sep}`) && !path.isAbsolute(rel);
}

function isolateInventory(dest) {
  if (isLink(dest)) throw new Error(`refusing symlinked isolate copy: ${dest}`);
  const files = [];
  const walk = rel => {
    const full = rel ? path.join(dest, rel) : dest;
    const stat = fs.lstatSync(full);
    if (stat.isSymbolicLink()) {
      const target = fs.readlinkSync(full);
      if (!containedLexical(dest, path.resolve(path.dirname(full), target))) {
        throw new Error(`archive link escaped isolate: ${rel}`);
      }
      files.push({ name: rel, type: 'link', mode: stat.mode & 0o777, sha256: null, target });
      return;
    }
    if (stat.isDirectory()) {
      for (const child of fs.readdirSync(full).sort()) walk(rel ? path.join(rel, child) : child);
      return;
    }
    if (!stat.isFile() || !rel) throw new Error(`unsupported isolate entry: ${rel || dest}`);
    const bytes = fs.readFileSync(full);
    files.push({ name: rel, type: 'file', mode: stat.mode & 0o777, sha256: digest(bytes), target: null, size: bytes.length });
  };
  walk('');
  return files;
}

function gitBlob(directory, hash) {
  const result = spawnSync('git', ['-C', directory, 'cat-file', 'blob', hash], { encoding: 'buffer', maxBuffer: 32 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(`missing committed blob ${hash}`);
  return result.stdout;
}

function assertCommittedInventory(directory, commit, files) {
  const listed = gitText(directory, 'ls-tree', '-r', '-z', commit).split('\0').filter(Boolean).map(line => {
    const tab = line.indexOf('\t');
    const [mode, type, hash] = line.slice(0, tab).split(' ');
    return { mode, type, hash, name: line.slice(tab + 1) };
  });
  if (listed.length !== files.length) throw new Error('isolate inventory does not match the frozen commit tree');
  const byName = new Map(files.map(file => [file.name, file]));
  for (const entry of listed) {
    const file = byName.get(entry.name);
    if (!file) throw new Error(`isolate missing committed path: ${entry.name}`);
    const blob = gitBlob(directory, entry.hash);
    if (entry.mode === '120000') {
      if (file.type !== 'link' || file.target !== blob.toString()) throw new Error(`isolate link mismatch: ${entry.name}`);
    } else if (entry.mode === '100644' || entry.mode === '100755') {
      if (file.type !== 'file' || file.sha256 !== digest(blob)) throw new Error(`isolate blob mismatch: ${entry.name}`);
    } else throw new Error(`unsupported committed mode: ${entry.mode}`);
  }
}

function assertExactOwnedIsolate(dest, links, source) {
  if (isLink(dest) || isLink(links)) throw new Error(`refusing redirected isolate path: ${dest}`);
  const realDest = fs.realpathSync(dest);
  const realLinks = fs.realpathSync(links);
  if (realDest === realLinks || !realDest.startsWith(realLinks + path.sep)) {
    throw new Error(`isolate copy escaped owned root: ${dest}`);
  }
  if (source && realDest === fs.realpathSync(source)) throw new Error(`isolate copy resolved to live source: ${dest}`);
  const record = readIsolateMarker(dest);
  if (!/^[a-f0-9]{40}$/.test(record.commit || '') || !/^[a-f0-9]{40}$/.test(record.tree || '') || !Array.isArray(record.files)) {
    throw new Error(`incomplete isolate ownership record: ${dest}`);
  }
  if (!record.files.every(file => file && typeof file.name === 'string' && ['file', 'link'].includes(file.type)
      && Number.isInteger(file.mode) && (file.type === 'link' ? typeof file.target === 'string' : /^[a-f0-9]{64}$/.test(file.sha256)))) {
    throw new Error(`isolate record lacks exact identity: ${dest}`);
  }
  if (source && fs.realpathSync(record.source) !== fs.realpathSync(source)) {
    throw new Error(`isolate source identity mismatch: ${dest}`);
  }
  const actual = isolateInventory(dest);
  if (actual.length !== record.files.length) throw new Error(`preserving changed isolate inventory: ${dest}`);
  for (let i = 0; i < actual.length; i++) {
    const got = actual[i], want = record.files[i];
    if (got.name !== want.name || got.type !== want.type || got.mode !== want.mode
        || got.sha256 !== want.sha256 || got.target !== want.target) {
      throw new Error(`preserving changed isolate material: ${got.name || dest}`);
    }
  }
}

function removeOwned(dest, links) {
  if (isLink(dest)) throw new Error(`refusing to delete symlink: ${dest}`);
  if (!exists(dest)) return;
  const realDest = fs.realpathSync(dest);
  const realLinks = fs.realpathSync(links);
  if (realDest !== realLinks && !realDest.startsWith(realLinks + path.sep)) {
    throw new Error(`refusing to delete outside owned isolate root: ${dest}`);
  }
  fs.rmSync(dest, { recursive: true, force: false });
  const mark = isolateMarkPath(dest);
  if (exists(mark) && !isLink(mark)) fs.unlinkSync(mark);
}
