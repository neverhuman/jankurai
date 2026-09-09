import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { publishRelease } from '../ops/ci/publish-release.mjs';

function fixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'release-publish-test-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  for (const file of ['binary.tar.gz', 'binary.tar.gz.sha256']) fs.writeFileSync(path.join(directory, file), file);
  const options = { directory, repository: 'neverhuman/jankurai', tag: 'v1.7.0', version: '1.7.0', commit: 'a'.repeat(40), notes: 'Reviewed release notes' };
  const state = { release: null, assets: [], mutations: [], uploads: [], failUpload: false, tag: options.commit };
  const asset = file => ({ name: path.basename(file), state: 'uploaded', size: fs.statSync(file).size,
    digest: 'sha256:' + createHash('sha256').update(fs.readFileSync(file)).digest('hex') });
  const api = (endpoint, body, method) => {
    if (method) state.mutations.push({ endpoint, body, method });
    if (endpoint.includes('/git/ref/tags/')) return { object: { type: 'commit', sha: state.tag } };
    if (endpoint.includes('/releases/tags/')) return state.release;
    if (endpoint.endsWith('/releases') && method === 'POST') return state.release = { id: 7, tag_name: body.tag_name, draft: true, prerelease: false };
    if (endpoint.includes('/assets?')) return structuredClone(state.assets);
    if (endpoint.endsWith('/releases/7') && method === 'PATCH') return state.release = { ...state.release, ...body, immutable: true };
    throw new Error('unexpected API call: ' + endpoint);
  };
  const upload = (_repository, _tag, file) => {
    if (state.failUpload) throw new Error('interrupted upload');
    state.uploads.push(path.basename(file)); state.assets.push(asset(file));
  };
  return { options, state, asset, run: () => publishRelease(options, api, upload) };
}
test('publication uploads a complete draft before making a stable immutable release', t => {
  const f = fixture(t), release = f.run();
  assert.equal(release.immutable, true); assert.equal(release.draft, false);
  assert.equal(f.state.uploads.length, 2);
  assert.deepEqual(f.state.mutations.map(x => x.method), ['POST', 'PATCH']);
  assert.deepEqual(f.state.mutations.at(-1).body, { draft: false, prerelease: false, make_latest: 'true' });
});
test('retry preserves a matching partial draft and uploads only missing assets', t => {
  const f = fixture(t); f.state.release = { id: 7, tag_name: 'v1.7.0', draft: true };
  f.state.assets.push(f.asset(path.join(f.options.directory, 'binary.tar.gz')));
  f.state.failUpload = true; assert.throws(f.run, /interrupted upload/);
  assert.equal(f.state.release.draft, true); assert.equal(f.state.assets.length, 1);
  f.state.failUpload = false; f.run();
  assert.deepEqual(f.state.uploads, ['binary.tar.gz.sha256']);
});
test('conflicting, extra, duplicate, or incomplete upload records are never replaced', t => {
  for (const defect of ['digest', 'extra', 'duplicate', 'state']) {
    const f = fixture(t); f.state.release = { id: 7, tag_name: 'v1.7.0', draft: true };
    const asset = f.asset(path.join(f.options.directory, 'binary.tar.gz'));
    if (defect === 'digest') asset.digest = 'sha256:' + '0'.repeat(64);
    if (defect === 'extra') asset.name = 'unreviewed';
    if (defect === 'state') asset.state = 'starter';
    f.state.assets.push(asset); if (defect === 'duplicate') f.state.assets.push(asset);
    assert.throws(f.run, /existing release asset differs/);
    assert.deepEqual(f.state.uploads, []); assert.deepEqual(f.state.mutations, []);
  }
});
test('a matching published release is read-only and idempotent', t => {
  const f = fixture(t); f.run(); f.state.mutations = []; f.state.uploads = [];
  f.run(); assert.deepEqual(f.state.mutations, []); assert.deepEqual(f.state.uploads, []);
  f.state.assets.pop(); assert.throws(f.run, /inventory is incomplete/);
  assert.deepEqual(f.state.mutations, []);
});
test('wrong source tags and non-immutable published releases fail closed', t => {
  const f = fixture(t); f.state.tag = 'b'.repeat(40);
  assert.throws(f.run, /source commit/); assert.deepEqual(f.state.mutations, []);
  f.state.tag = f.options.commit; f.run(); f.state.release.immutable = false; f.state.mutations = [];
  assert.throws(f.run, /not an immutable stable release/); assert.deepEqual(f.state.mutations, []);
});
