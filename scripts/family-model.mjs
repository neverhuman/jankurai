import fs from 'node:fs';
import path from 'node:path';
import { readToml, exists, isLink, gitText, git, clean, atomicWrite, run, buildEnvironment } from './family-lib.mjs';

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
  materializeIsolate(directory, dest) {
    const links = path.join(this.fusion, 'components');
    if (isLink(dest)) {
      if (fs.realpathSync(dest) !== directory) throw new Error(`refusing to replace symlink: ${dest}`);
      fs.unlinkSync(dest);
    }
    if (exists(dest) && !isolateMarker(dest)) throw new Error(`refusing to overwrite directory: ${dest}`);
    if (exists(dest)) removeOwned(dest, links);
    fs.mkdirSync(dest, { recursive: true });
    const archive = `${dest}.git-archive.tar`;
    run(['git', '-C', directory, 'archive', '--format=tar', '-o', archive, 'HEAD']);
    try {
      run(['tar', '-xf', archive, '-C', dest]);
    } finally {
      if (exists(archive)) fs.unlinkSync(archive);
    }
    writeIsolateMarker(dest);
  }
  rematerializeIsolates() {
    const links = this.ownedComponentRoot(true);
    for (const repo of this.components()) {
      if (!this.existing(repo)) throw new Error(`missing component: ${repo.name}`);
      this.materializeIsolate(this.path(repo), path.join(links, repo.name));
    }
  }
  disposeIsolates() {
    const links = path.join(this.fusion, 'components');
    if (!exists(links) || isLink(links)) return;
    for (const repo of this.components()) {
      const dest = path.join(links, repo.name);
      if (exists(dest) && isolateMarker(dest)) removeOwned(dest, links);
    }
  }
  executionPath(repo) {
    const isolated = path.join(this.fusion, 'components', repo.name);
    if (exists(isolated) && isolateMarker(isolated)) return isolated;
    if (this.allowLiveRequired) return this.path(repo);
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
  return exists(isolateMarkPath(directory));
}

function writeIsolateMarker(directory) {
  fs.writeFileSync(isolateMarkPath(directory), 'owned-execution-copy\n');
}

function removeOwned(dest, links) {
  if (isLink(dest)) throw new Error(`refusing to delete symlink: ${dest}`);
  if (!exists(dest)) return;
  const realDest = fs.realpathSync(dest);
  const realLinks = fs.realpathSync(links);
  if (realDest !== realLinks && !realDest.startsWith(realLinks + path.sep)) {
    throw new Error(`refusing to delete outside .fusion/components: ${dest}`);
  }
  fs.rmSync(dest, { recursive: true, force: false });
  const mark = isolateMarkPath(dest);
  if (exists(mark) && !isLink(mark)) fs.unlinkSync(mark);
}
