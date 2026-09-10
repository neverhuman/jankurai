import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { SUCCESS_MARKER, qualifyPreTag } from './pre-tag-qualify.mjs';

const SOURCE = '0123456789abcdef0123456789abcdef01234567';
const CERT = 'https://github.com/neverhuman/jankurai/.github/workflows/release-services.yml@refs/heads/main';

export function writeBoundEvidence(dir, {
  marker = SUCCESS_MARKER,
  cert = CERT,
  source = SOURCE,
  artifact = 'probe.txt',
  releaseYmlIdentity = false,
} = {}) {
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'result.txt'), `${marker}\n`);
  const identityCert = releaseYmlIdentity
    ? 'https://github.com/neverhuman/jankurai/.github/workflows/release.yml@refs/heads/main'
    : cert;
  fs.writeFileSync(path.join(dir, 'identity.txt'),
    `cert-identity=${identityCert}\nworkflow=neverhuman/jankurai/.github/workflows/release-services.yml\nsource=${source}\nartifact=${artifact}\n`);
  fs.writeFileSync(path.join(dir, artifact), `Non-release signing probe\nsource=${source}\nplatform=Linux/x86_64\n`);
  fs.writeFileSync(path.join(dir, `${artifact}.sigstore.bundle`), '{"bundle":true}\n');
  fs.writeFileSync(path.join(dir, `${artifact}.attestation.jsonl`), '{"attestation":true}\n');
  return dir;
}

test('pre-tag-qualify fails closed without evidence and rejects release.yml masquerade', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'pre-tag-qualify-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  assert.throws(() => qualifyPreTag(path.join(root, 'missing')), /missing/);
  fs.mkdirSync(path.join(root, 'empty'));
  assert.throws(() => qualifyPreTag(path.join(root, 'empty')), /missing release-services result/);

  const lone = path.join(root, 'lone');
  fs.mkdirSync(lone);
  fs.writeFileSync(path.join(lone, 'result.txt'), `${SUCCESS_MARKER}\n`);
  assert.throws(() => qualifyPreTag(lone), /missing release-services cert-identity|workflow run identity/);

  const ok = path.join(root, 'ok');
  writeBoundEvidence(ok);
  assert.deepEqual(qualifyPreTag(ok).ok, true);

  const bad = path.join(root, 'bad-yml');
  writeBoundEvidence(bad, { releaseYmlIdentity: true });
  assert.throws(() => qualifyPreTag(bad), /refusing/);
});

test('pre-tag-qualify refuses incomplete workflow/source/artifact/signature bindings', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'pre-tag-bind-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));

  const noBundle = path.join(root, 'no-bundle');
  writeBoundEvidence(noBundle);
  fs.unlinkSync(path.join(noBundle, 'probe.txt.sigstore.bundle'));
  assert.throws(() => qualifyPreTag(noBundle), /signature bundle/);

  const noAttest = path.join(root, 'no-attest');
  writeBoundEvidence(noAttest);
  fs.unlinkSync(path.join(noAttest, 'probe.txt.attestation.jsonl'));
  assert.throws(() => qualifyPreTag(noAttest), /attestation/);

  const noProbe = path.join(root, 'no-probe');
  writeBoundEvidence(noProbe);
  fs.unlinkSync(path.join(noProbe, 'probe.txt'));
  assert.throws(() => qualifyPreTag(noProbe), /artifact identity/);

  const sourceMismatch = path.join(root, 'mismatch');
  writeBoundEvidence(sourceMismatch);
  fs.writeFileSync(path.join(sourceMismatch, 'probe.txt'), 'source=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n');
  assert.throws(() => qualifyPreTag(sourceMismatch), /source digest disagrees/);
});
