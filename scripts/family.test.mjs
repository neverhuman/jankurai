import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { Family } from './family-model.mjs';
import { git, gitText, atomicWrite, buildEnvironment } from './family-lib.mjs';
import { lockText, update } from './family-update.mjs';
import { branchFor, validateCandidate, publish } from './publish-family-update.mjs';

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'family-ci-test-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const hub = path.join(root, 'jankurai'), repo = path.join(root, 'jankurai-core');
  for (const directory of [hub, repo]) {
    fs.mkdirSync(directory);
    git(directory, ['init', '-q', '-b', 'main']);
    git(directory, ['config', 'user.name', 'CI fixture']);
    git(directory, ['config', 'user.email', 'fixture@example.invalid']);
    fs.writeFileSync(path.join(directory, 'README.md'), 'base\n');
    git(directory, ['add', '.']);
    git(directory, ['commit', '-qm', 'base']);
  }
  const sha = gitText(repo, 'rev-parse', 'HEAD');
  git(repo, ['tag', 'fixture-v1']);
  const names = ['jankurai', 'jankurai-core'];
  let manifest = 'schema_version = "2.0.0"\nauthority_forge = "github"\npublic_owner = "neverhuman"\nexpected_repo_count = 2\nrequired_repos = ["jankurai", "jankurai-core"]\n';
  for (const name of names) manifest += `\n[[repo]]\nname = "${name}"\npath = "${name}"\nslug = "neverhuman/${name}"\ndefault_branch = "main"\nrequired_check = "${name}/required"\ngithub = "https://github.com/neverhuman/${name}.git"\nhosted = "https://github.com/neverhuman/${name}.git"\n`;
  fs.writeFileSync(path.join(hub, 'repos.manifest.toml'), manifest);
  fs.writeFileSync(path.join(hub, 'family.lock'), `[[repo]]\nrepo = "jankurai-core"\ntag = "fixture-v1"\ncommit = "${sha}"\n`);
  const family = new Family(hub);
  return { root, hub, repo, sha, family };
}
test('build bootstrap preserves dirty files and does not fetch existing heads', t => {
  const { repo, sha, family } = fixture(t);
  fs.writeFileSync(path.join(repo, 'README.md'), 'valuable draft');
  family.fetchPin = () => { throw new Error('unexpected fetch'); };
  family.bootstrap();
  assert.equal(fs.readFileSync(path.join(repo, 'README.md'), 'utf8'), 'valuable draft');
  assert.equal(gitText(repo, 'rev-parse', 'HEAD'), sha);
});
test('setup rejects dirty checkouts before fetching or changing refs', t => {
  const { repo, sha, family } = fixture(t);
  fs.writeFileSync(path.join(repo, 'untracked'), 'valuable work');
  family.fetchPin = () => { throw new Error('unexpected fetch'); };
  assert.throws(() => family.bootstrap(true), /dirty checkout/);
  assert.equal(gitText(repo, 'rev-parse', 'HEAD'), sha);
});
test('setup preserves ahead commits and branch', t => {
  const { repo, family } = fixture(t);
  fs.writeFileSync(path.join(repo, 'README.md'), 'unpublished');
  git(repo, ['commit', '-qam', 'valuable commit']);
  const head = gitText(repo, 'rev-parse', 'HEAD');
  family.fetchPin = () => {};
  assert.throws(() => family.bootstrap(true), /ahead\/divergent/);
  assert.equal(gitText(repo, 'rev-parse', 'HEAD'), head);
  assert.equal(gitText(repo, 'branch', '--show-current'), 'main');
});
test('repeated setup preserves the locked head and branch', t => {
  const { repo, sha, family } = fixture(t);
  family.fetchPin = () => {};
  family.bootstrap(true); family.bootstrap(true);
  assert.equal(gitText(repo, 'rev-parse', 'HEAD'), sha);
  assert.equal(gitText(repo, 'branch', '--show-current'), 'main');
});
test('an unavailable commit leaves the checkout unchanged', t => {
  const { repo, sha, family } = fixture(t);
  family.fetchPin = () => { throw new Error('unavailable commit'); };
  assert.throws(() => family.bootstrap(true), /unavailable commit/);
  assert.equal(gitText(repo, 'rev-parse', 'HEAD'), sha);
});
test('symlink checkout and manifest path traversal are rejected', t => {
  const { root, repo, family } = fixture(t);
  family.repos[1].path = '../escape';
  assert.throws(() => family.validate(), /relative canonical path/);
  family.repos[1].path = 'jankurai-core';
  fs.renameSync(repo, path.join(root, 'unrelated'));
  fs.symlinkSync(path.join(root, 'unrelated'), repo);
  assert.throws(() => family.bootstrap(), /canonical primary checkout/);
});
test('manifest rejects duplicate membership and revision pins', t => {
  const { family } = fixture(t);
  family.repos[1].tag = 'mutable';
  assert.throws(() => family.validate(), /only in family.lock/);
  delete family.repos[1].tag;
  family.repos.push(family.repos[1]);
  assert.throws(() => family.validate(), /duplicate or missing/);
});
test('lock rendering changes only the selected immutable pin', t => {
  const { hub, family } = fixture(t);
  const original = fs.readFileSync(path.join(hub, 'family.lock'), 'utf8');
  assert.equal(lockText(original, family.pins), original);
  family.pins.get('jankurai-core').commit = '1'.repeat(40);
  assert.match(lockText(original, family.pins), /commit = "1{40}"/);
  assert.equal(fs.readFileSync(path.join(hub, 'family.lock'), 'utf8'), original);
});
test('failed candidate preserves locks and heads and removes its CI sandbox', t => {
  const { hub, repo, sha, family } = fixture(t);
  fs.writeFileSync(path.join(hub, 'Cargo.lock'), 'version = 4\n');
  fs.writeFileSync(path.join(hub, '.gitignore'), 'target/\n');
  git(hub, ['add', '.']); git(hub, ['commit', '-qm', 'accepted locks']);
  fs.writeFileSync(path.join(repo, 'README.md'), 'candidate');
  git(repo, ['commit', '-qam', 'candidate']);
  const candidate = gitText(repo, 'rev-parse', 'HEAD');
  git(repo, ['checkout', '--detach', sha], { capture: true });
  const before = fs.readFileSync(path.join(hub, 'family.lock'), 'utf8');
  assert.throws(() => update(family, { eligible: () => candidate, testCandidate: (_family, directory) => {
    assert.equal(fs.statSync(directory).isDirectory(), true);
    throw new Error('candidate integration failed');
  } }), /candidate integration failed/);
  assert.equal(fs.readFileSync(path.join(hub, 'family.lock'), 'utf8'), before);
  assert.equal(fs.readFileSync(path.join(hub, 'Cargo.lock'), 'utf8'), 'version = 4\n');
  assert.equal(gitText(repo, 'rev-parse', 'HEAD'), sha);
  assert.deepEqual(fs.readdirSync(path.join(hub, 'target')), []);
});
test('publisher rejects metadata changes before any remote lookup', t => {
  const { hub, family } = fixture(t), lock = fs.readFileSync(path.join(hub, 'family.lock'), 'utf8');
  const base = { 'family.lock': lock, 'Cargo.lock': 'version = 4\n' };
  const candidate = { ...base, 'family.lock': lock.replace('jankurai-core', 'jankurai-other') };
  assert.throws(() => validateCandidate(family, candidate, base), /metadata/);
  assert.notEqual(branchFor(base), branchFor(candidate));
});
test('publication credentials are removed from candidate build environment', () => {
  const before = process.env.GH_TOKEN;
  try { process.env.GH_TOKEN = 'controlled-test-value'; assert.equal(buildEnvironment().GH_TOKEN, undefined); }
  finally { if (before === undefined) delete process.env.GH_TOKEN; else process.env.GH_TOKEN = before; }
});

function publicationFixture(t) {
  const { root, hub, family } = fixture(t);
  const directory = path.join(root, 'candidate'); fs.mkdirSync(directory);
  const componentSha = 'c'.repeat(40), baseSha = 'a'.repeat(40), oldHead = 'b'.repeat(40), newHead = 'd'.repeat(40);
  const cargo = 'version = 4\n[[package]]\nname = "fixture"\nversion = "1.0.0"\n';
  const base = { 'family.lock': fs.readFileSync(path.join(hub, 'family.lock'), 'utf8'), 'Cargo.lock': cargo };
  const candidate = { ...base, 'family.lock': `[[repo]]\nrepo = "jankurai-core"\ntag = "ci-${componentSha}"\ncommit = "${componentSha}"\n` };
  for (const [file, text] of Object.entries(candidate)) fs.writeFileSync(path.join(directory, file), text);
  const branch = branchFor(candidate), calls = [];
  const state = { upToDate: false, unexpectedFile: false, eligible: true, changedCandidate: false };
  const request = (endpoint, body, method) => {
    calls.push({ endpoint, body, method });
    if (endpoint.endsWith('/git/ref/heads/main')) return { object: { sha: baseSha } };
    if (endpoint.includes('/contents/')) {
      const file = endpoint.split('/contents/')[1].split('?')[0], isBase = endpoint.endsWith(`ref=${baseSha}`);
      const text = (isBase ? base : candidate)[file] + (!isBase && state.changedCandidate ? '# changed\n' : '');
      return { content: Buffer.from(text).toString('base64') };
    }
    if (endpoint.includes('/git/ref/tags/')) return { object: { type: 'commit', sha: componentSha } };
    if (endpoint.includes('/jankurai-core/compare/')) return { status: 'identical' };
    if (endpoint.includes('/check-runs?')) return { check_runs: state.eligible ? [{
      name: 'jankurai-core/required', head_sha: componentSha, status: 'completed', conclusion: 'success', app: { slug: 'github-actions' }
    }] : [] };
    if (endpoint.includes('/pulls?')) return [{ number: 7, html_url: 'https://example.invalid/pull/7', head: { sha: oldHead, ref: branch, repo: { full_name: 'neverhuman/jankurai' } } }];
    if (endpoint.endsWith('/pulls/7/files?per_page=100')) return [{ filename: state.unexpectedFile ? 'README.md' : 'family.lock', status: 'modified' }];
    if (endpoint.includes('/compare/')) return { merge_base_commit: { sha: state.upToDate ? baseSha : 'e'.repeat(40) } };
    if (endpoint.endsWith(`/git/commits/${baseSha}`)) return { tree: { sha: 'base-tree' } };
    if (endpoint.endsWith('/git/blobs')) return { sha: 'blob-' + calls.length };
    if (endpoint.endsWith('/git/trees')) return { sha: 'new-tree' };
    if (endpoint.endsWith('/git/commits')) return { sha: newHead };
    if (endpoint.includes('/git/refs/heads/')) { state.upToDate = true; return {}; }
    throw new Error(`unexpected request ${endpoint}`);
  };
  return { family, directory, request, calls, state, baseSha, oldHead, newHead };
}
test('publisher refreshes the existing PR when main advances and is then idempotent', t => {
  const f = publicationFixture(t);
  publish(f.family, f.directory, f.request);
  const writes = f.calls.filter(call => call.body);
  const tree = writes.find(call => call.endpoint.endsWith('/git/trees')).body;
  assert.equal(tree.base_tree, 'base-tree');
  assert.deepEqual(tree.tree.map(entry => entry.path), ['family.lock', 'Cargo.lock']);
  assert.deepEqual(writes.find(call => call.endpoint.endsWith('/git/commits')).body.parents, [f.oldHead, f.baseSha]);
  assert.deepEqual(writes.at(-1).body, { sha: f.newHead, force: false });
  assert.equal(writes.at(-1).method, 'PATCH');
  f.calls.length = 0;
  publish(f.family, f.directory, f.request);
  assert.equal(f.calls.filter(call => call.body).length, 0);
});
test('publisher preserves an existing PR with unrelated or altered work', t => {
  for (const key of ['unexpectedFile', 'changedCandidate', 'eligible']) {
    const f = publicationFixture(t); f.state[key] = key !== 'eligible';
    assert.throws(() => publish(f.family, f.directory, f.request), /unexpected changes|candidate changed|successful default-branch/);
    assert.equal(f.calls.filter(call => call.body).length, 0);
  }
});
