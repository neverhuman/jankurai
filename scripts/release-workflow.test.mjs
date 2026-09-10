import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import { PROMOTION_JOBS } from '../ops/ci/publish-release.mjs';

const source = fs.readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');

function checkWorkflow(text) {
  // Deliberately fixed workflow layout: changing this boundary requires review.
  const start = text.indexOf('\njobs:\n');
  assert.ok(start > 0);
  const defaults = text.slice(0, start), body = text.slice(start + 7);
  assert.match(defaults, /\npermissions:\n  contents: read\n/);
  assert.doesNotMatch(defaults, /(?:id-token|attestations): write/);
  const headers = [...body.matchAll(/^  ([\w-]+):$/gm)];
  const jobs = Object.fromEntries(headers.map((match, i) => [match[1], body.slice(match.index, headers[i + 1]?.index)]));
  assert.deepEqual(Object.keys(jobs), ['validate', 'build', 'sign', 'verify', 'staged-native', 'publish', 'smoke', 'promote']);
  for (const [name, value] of Object.entries(jobs)) {
    if (name !== 'sign') assert.doesNotMatch(value, /(?:id-token|attestations): write/);
    if (!['publish', 'promote'].includes(name)) assert.doesNotMatch(value, /contents: write/);
  }
  assert.match(jobs.build, /^    needs: validate$/m);
  assert.ok(jobs.build.includes('    name: build (${{ matrix.os }}, ${{ matrix.target }})'));
  for (const name of ['staged-native', 'smoke']) assert.ok(jobs[name].includes('    name: ' + name + ' (${{ matrix.os }})'));
  assert.match(jobs.build, /name: unsigned-release-\$\{\{ matrix.target \}\}/);
  assert.doesNotMatch(jobs.build, /sign-release|attest-build|verify-staged-release/);
  for (const target of ['x86_64-unknown-linux-gnu', 'aarch64-apple-darwin']) assert.ok(jobs.build.includes(`target: ${target}`));
  assert.match(jobs.sign, /^    needs: build$/m);
  assert.match(jobs.sign, /pattern: unsigned-release-\*/);
  assert.match(jobs.sign, /name: signed-release/);
  assert.match(jobs.sign, /id-token: write/);
  assert.match(jobs.sign, /attestations: write/);
  assert.deepEqual([...jobs.sign.matchAll(/^      - run: (.+)$/gm)].map(match => match[1]), [
    'node ops/ci/verify-release-assets.mjs dist --unsigned',
    'bash ops/ci/sign-release.sh', 'bash ops/ci/record-attestations.sh',
  ]);
  assert.match(jobs.verify, /^    needs: sign$/m);
  assert.match(jobs.verify, /name: signed-release/);
  assert.match(jobs.verify, /name: verified-release/);
  assert.match(jobs.verify, /bash ops\/ci\/verify-release-signatures.sh/);
  assert.match(jobs['staged-native'], /^    needs: verify$/m);
  assert.match(jobs.publish, /^    needs: \[verify, staged-native\]$/m);
  assert.match(jobs.smoke, /^    needs: publish$/m);
  assert.match(jobs.promote, /^    needs: smoke$/m);
  for (const name of ['staged-native', 'publish', 'promote']) assert.match(jobs[name], /name: verified-release/);
  for (const name of ['staged-native', 'smoke']) assert.match(jobs[name], /os: \[ubuntu-24\.04, macos-14\]/);
  assert.match(jobs.smoke, /bash ops\/ci\/public-install-smoke.sh/);
  assert.match(jobs.smoke, /bash ops\/ci\/release-smoke.sh/);
  assert.match(jobs.promote, /node ops\/ci\/verify-release-assets.mjs dist/);
  assert.match(jobs.promote, /node ops\/ci\/publish-release.mjs --promote/);
  assert.doesNotMatch(jobs.promote, /if:|continue-on-error:/);
  assert.deepEqual(PROMOTION_JOBS, ['validate', 'build (ubuntu-24.04, x86_64-unknown-linux-gnu)',
    'build (macos-14, aarch64-apple-darwin)', 'sign', 'verify',
    'staged-native (ubuntu-24.04)', 'staged-native (macos-14)', 'publish',
    'smoke (ubuntu-24.04)', 'smoke (macos-14)']);
}

test('release authority is separated from building and executing candidate payloads', () => checkWorkflow(source));

test('workflow mutations cannot grant signing authority to builds or bypass native gates', () => {
  for (const changed of [
    source.replace('  build:\n', '  build:\n    permissions:\n      id-token: write\n'),
    source.replace('bash ops/ci/sign-release.sh', 'bash scripts/family.sh build'),
    source.replace('needs: [verify, staged-native]', 'needs: verify'),
    source.replace('needs: smoke', 'needs: publish'),
    source.replace('os: [ubuntu-24.04, macos-14]', 'os: [ubuntu-24.04]'),
    source.replace('name: signed-release\n          path: dist', 'name: unsigned-release\n          path: dist'),
    source.replace('bash ops/ci/public-install-smoke.sh', 'true'),
    source.replace('bash ops/ci/release-smoke.sh', 'true'),
    source.replace('  promote:\n', '  promote:\n    if: always()\n'),
    source.replace('  staged-native:\n', '  renamed-native:\n'),
  ]) {
    assert.notEqual(changed, source);
    assert.throws(() => checkWorkflow(changed));
  }
});
