import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const hub = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
function inventory(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'release-inventory-test-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const dist = path.join(root, 'dist'); fs.mkdirSync(dist);
  for (const name of ['family.lock', 'Cargo.lock', 'jankurai-installer.sh', 'jankurai-ux-qa-1.7.0.tgz']) fs.writeFileSync(path.join(dist, name), name);
  for (const target of ['x86_64-unknown-linux-gnu', 'aarch64-apple-darwin']) {
    fs.writeFileSync(path.join(dist, `provenance-${target}.json`), '{}');
    for (const product of ['jankurai', 'tuiwright']) {
      const stem = `${product}-1.7.0-${target}`, stage = path.join(root, stem); fs.mkdirSync(stage);
      for (const name of [product, 'LICENSE', 'family.lock', 'Cargo.lock', 'provenance.json']) fs.writeFileSync(path.join(stage, name), name);
      const packed = spawnSync('tar', ['-czf', path.join(dist, `${stem}.tar.gz`), '-C', root, stem]);
      assert.equal(packed.status, 0);
    }
  }
  for (const name of fs.readdirSync(dist)) {
    const digest = createHash('sha256').update(fs.readFileSync(path.join(dist, name))).digest('hex');
    fs.writeFileSync(path.join(dist, name + '.sha256'), `${digest}  ${name}\n`);
    // Cryptographic verification runs separately in the staged release job.
    fs.writeFileSync(path.join(dist, name + '.sigstore.bundle'), 'fixture signature');
    fs.writeFileSync(path.join(dist, name + '.attestation.jsonl'), 'fixture attestation');
  }
  // The collected-artifact jobs have Node but do not install npm dependencies.
  fs.copyFileSync(path.join(hub, 'ops/ci/verify-release-assets.mjs'), path.join(root, 'verify.mjs'));
  fs.writeFileSync(path.join(root, 'VERSION'), '1.7.0\n');
  const verify = () => spawnSync(process.execPath, ['verify.mjs', dist], { cwd: root, encoding: 'utf8' });
  return { root, dist, verify };
}
test('release inventory accepts exactly the public products and verification companions', t => {
  const f = inventory(t), result = f.verify(); assert.equal(result.status, 0, result.stderr);
  fs.writeFileSync(path.join(f.dist, 'jankurai-governed-launcher'), 'internal');
  const extra = f.verify(); assert.notEqual(extra.status, 0); assert.match(extra.stderr, /unexpected release asset inventory/);
});
test('release inventory rejects missing attestations and linked metadata', t => {
  const f = inventory(t), bundle = path.join(f.dist, 'family.lock.attestation.jsonl');
  fs.unlinkSync(bundle); assert.notEqual(f.verify().status, 0);
  fs.writeFileSync(bundle, 'fixture attestation');
  const lock = path.join(f.dist, 'family.lock'), outside = path.join(f.root, 'lock');
  fs.renameSync(lock, outside); fs.symlinkSync(outside, lock);
  const result = f.verify(); assert.notEqual(result.status, 0); assert.match(result.stderr, /non-regular release asset/);
});
