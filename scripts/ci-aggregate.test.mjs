import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workflow = fs.readFileSync(path.join(root, '.github/workflows/ci.yml'), 'utf8');
const aggregateScript = fs.readFileSync(path.join(root, 'ops/ci/aggregate.sh'), 'utf8');
const required = workflow.slice(workflow.indexOf('\n  required:\n'));

assert.match(required, /^      - run: bash ops\/ci\/aggregate\.sh$/m);
assert.match(required, /^          NEEDS_JSON: \$\{\{ toJSON\(needs\) \}\}$/m);
assert.match(aggregateScript, /keys == \["fast", "integration", "release-build"\]/);

function aggregate(value) {
  const result = spawnSync('bash', [path.join(root, 'ops/ci/aggregate.sh')], {
    encoding: 'utf8',
    env: { PATH: process.env.PATH, NEEDS_JSON: value },
  });
  assert.ifError(result.error);
  return result.status === 0;
}

const success = { result: 'success', outputs: {} };
const lanes = ['fast', 'integration', 'release-build'];
const successfulJobs = Object.fromEntries(lanes.map(lane => [lane, success]));

test('all successful required jobs pass regardless of key order', () => {
  assert.equal(aggregate(JSON.stringify(successfulJobs)), true);
  assert.equal(
    aggregate(JSON.stringify(Object.fromEntries(Object.entries(successfulJobs).reverse()))),
    true,
  );
});

for (const [name, value] of [
  ['empty object', '{}'],
  ['empty array', '[]'],
  ['null', 'null'],
  ['string', '"success"'],
  ['boolean', 'true'],
  ['number', '0'],
  ['missing input', ''],
  ['malformed JSON', '{'],
  ['multiple JSON values', '{}\n{}'],
  ['missing fast', JSON.stringify({ integration: success, 'release-build': success })],
  ['missing integration', JSON.stringify({ fast: success, 'release-build': success })],
  ['missing release-build', JSON.stringify({ fast: success, integration: success })],
  ['renamed job', JSON.stringify({ fast: success, integration: success, release: success })],
  ['extra job', JSON.stringify({ ...successfulJobs, extra: success })],
  ['successful array', JSON.stringify([success, success, success])],
]) {
  test(`required aggregate rejects ${name}`, () => assert.equal(aggregate(value), false));
}

for (const lane of lanes) {
  test(`required aggregate rejects missing ${lane}`, () => {
    const jobs = { ...successfulJobs };
    delete jobs[lane];
    assert.equal(aggregate(JSON.stringify(jobs)), false);
  });
  for (const value of [
    null,
    [],
    'success',
    {},
    { result: true },
    { result: 0 },
    ...['failure', 'cancelled', 'skipped', 'pending', 'neutral', 'timed_out'].map(result => ({
      result,
    })),
  ]) {
    test(`required aggregate rejects ${lane} outcome ${JSON.stringify(value)}`, () => {
      const jobs = { ...successfulJobs, [lane]: value };
      assert.equal(aggregate(JSON.stringify(jobs)), false);
    });
  }
}

function assertWorkflowLanes(source) {
  // Keep this deliberately narrow: a workflow layout change requires review.
  const jobs = [...source.matchAll(/^  ([\w-]+):$/gm)].map(match => match[1]);
  assert.deepEqual(jobs, [
    'push',
    'pull_request',
    'workflow_dispatch',
    'fast',
    'integration',
    'release-build',
    'required',
    'publish-ci-tag',
  ]);
  assert.match(source, /^    needs: \[fast, integration, release-build\]$/m);
  assert.match(source, /^    if: always\(\)$/m);
  assert.match(source, /^      - run: bash ops\/ci\/aggregate\.sh$/m);
  assert.match(source, /^          NEEDS_JSON: \$\{\{ toJSON\(needs\) \}\}$/m);
  assert.match(
    source,
    /include:\n(?: {10}.+\n)*? {10}- os: ubuntu-24\.04\n {12}target: x86_64-unknown-linux-gnu\n {10}- os: macos-14\n {12}target: aarch64-apple-darwin/,
  );
}

test('workflow retains every dependency and both release-build matrix legs', () => {
  assertWorkflowLanes(workflow);
});

test('workflow mutations removing a dependency, job, matrix leg, or always gate are rejected', () => {
  for (const changed of [
    workflow.replace('needs: [fast, integration, release-build]', 'needs: [fast]'),
    workflow.replace('needs: [fast, integration, release-build]', 'needs: [fast, integration]'),
    workflow.replace('needs: [fast, integration, release-build]', 'needs: [integration, release-build]'),
    workflow.replace(/\n  fast:\n[\s\S]*?(?=\n  integration:)/, '\n'),
    workflow.replace(/\n  integration:\n[\s\S]*?(?=\n  release-build:)/, '\n'),
    workflow.replace(/\n  release-build:\n[\s\S]*?(?=\n  required:)/, '\n'),
    workflow.replace(
      /- os: ubuntu-24\.04\n {12}target: x86_64-unknown-linux-gnu\n/,
      '',
    ),
    workflow.replace(
      /- os: macos-14\n {12}target: aarch64-apple-darwin\n/,
      '',
    ),
    workflow.replace(/^    if: always\(\)\n/m, ''),
    workflow.replace('bash ops/ci/aggregate.sh', 'bash ops/ci/required.sh'),
  ]) {
    assert.notEqual(changed, workflow);
    assert.throws(() => assertWorkflowLanes(changed));
  }
});
