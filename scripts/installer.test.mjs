import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const installer = path.join(path.dirname(path.dirname(fileURLToPath(import.meta.url))), 'jankurai-installer.sh');
const sha256 = value => createHash('sha256').update(value).digest('hex');
function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-ci-test-')), tools = path.join(root, 'tools');
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(tools);
  const stem = 'jankurai-1.7.0-x86_64-unknown-linux-gnu', asset = `${stem}.tar.gz`, commit = '4'.repeat(40);
  const payload = { jankurai: '#!/bin/sh\nprintf "jankurai 1.7.0\\n"\n', 'family.lock': 'family fixture', 'Cargo.lock': 'cargo fixture', LICENSE: 'MIT' };
  const provenance = { commit, target: 'x86_64-unknown-linux-gnu', version: '1.7.0',
    family_lock_sha256: sha256(payload['family.lock']), cargo_lock_sha256: sha256(payload['Cargo.lock']) };
  const tool = (name, source) => fs.writeFileSync(path.join(tools, name), '#!/usr/bin/env node\n' + source, { mode: 0o755 });
  tool('uname', "console.log(process.argv[2] === '-s' ? 'Linux' : 'x86_64');");
  tool('curl', "const fs=require('node:fs'),path=require('node:path'),a=process.argv; const url=a.find(x=>x.startsWith('https:')); fs.copyFileSync(path.join(process.env.FIXTURE_ROOT,url.split('/').at(-1)),a[a.indexOf('-o')+1]);");
  tool('gh', "const a=process.argv; if(a[2]==='api') console.log(process.env.FIXTURE_COMMIT); else { if(!a.includes('--deny-self-hosted-runners') || a[a.indexOf('--cert-identity')+1]!==process.env.FIXTURE_IDENTITY) throw new Error('wrong attestation identity'); if(process.env.ATTESTATION_FAILURE) throw new Error('attestation rejected'); }");
  tool('cosign', "const a=process.argv; if(a[a.indexOf('--certificate-identity')+1]!==process.env.FIXTURE_IDENTITY) throw new Error('wrong signature identity'); if(process.env.SIGNATURE_FAILURE) throw new Error('signature rejected');");
  const env = { ...process.env, PATH: tools + path.delimiter + process.env.PATH, FIXTURE_ROOT: root,
    FIXTURE_COMMIT: commit, FIXTURE_IDENTITY: 'https://github.com/neverhuman/jankurai/.github/workflows/release.yml@refs/tags/v1.7.0' };
  function pack(extra) {
    const stage = path.join(root, stem);
    fs.mkdirSync(stage, { recursive: true });
    for (const [name, contents] of Object.entries({ ...payload, 'provenance.json': JSON.stringify(provenance) })) fs.writeFileSync(path.join(stage, name), contents);
    if (extra) extra(stage);
    const result = spawnSync('tar', ['-czf', path.join(root, asset), '-C', root, stem]);
    assert.equal(result.status, 0);
    fs.writeFileSync(path.join(root, asset + '.sha256'), sha256(fs.readFileSync(path.join(root, asset))) + '  ' + asset + '\n');
    fs.writeFileSync(path.join(root, asset + '.sigstore.bundle'), 'controlled verifier fixture');
  }
  const install = (...args) => spawnSync('bash', [installer, '--verify-only', '--tag', 'v1.7.0', ...args], { env, encoding: 'utf8' });
  return { root, asset, payload, provenance, env, pack, install };
}
test('valid asset requires both exact workflow verification identities', t => {
  const f = fixture(t); f.pack(); const result = f.install();
  assert.equal(result.status, 0, result.stderr);
});
test('checksum tampering is rejected', t => {
  const f = fixture(t); f.pack(); fs.appendFileSync(path.join(f.root, f.asset), 'tampered');
  const result = f.install(); assert.notEqual(result.status, 0); assert.match(result.stderr, /checksum mismatch/);
});
test('provenance commit mismatch is rejected with a matching checksum', t => {
  const f = fixture(t); f.provenance.commit = '5'.repeat(40); f.pack();
  const result = f.install(); assert.notEqual(result.status, 0); assert.match(result.stderr, /provenance mismatch/);
});
test('embedded lock tampering is rejected', t => {
  const f = fixture(t); f.payload['family.lock'] = 'different family'; f.pack();
  const result = f.install(); assert.notEqual(result.status, 0); assert.match(result.stderr, /lock provenance mismatch/);
});
test('failed attestation or signature prevents installation', t => {
  const f = fixture(t); f.pack();
  for (const key of ['ATTESTATION_FAILURE', 'SIGNATURE_FAILURE']) {
    f.env[key] = 'yes'; assert.notEqual(f.install().status, 0); delete f.env[key];
  }
});
test('linked archive payload is rejected before extraction', t => {
  const f = fixture(t); f.pack(stage => { fs.unlinkSync(path.join(stage, 'jankurai')); fs.symlinkSync('../escape', path.join(stage, 'jankurai')); });
  const result = f.install(); assert.notEqual(result.status, 0); assert.match(result.stderr, /unsafe archive entry/);
});
test('unexpected archive inventory is rejected', t => {
  const f = fixture(t); f.pack(stage => fs.writeFileSync(path.join(stage, 'unwanted'), 'data'));
  const result = f.install(); assert.notEqual(result.status, 0); assert.match(result.stderr, /archive inventory/);
});
