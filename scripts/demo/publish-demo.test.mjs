import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { publishDemo } from './publish-demo.mjs';
import { PUBLIC_FILES, CAPTURE_FILES } from './demo-catalog.mjs';

// Synthetic publication plumbing only. Real decoder/evidence checks run separately.
function fixture(run) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'jankurai-publication-control-')));
  const source = path.join(root, 'source'), rendered = path.join(source, 'rendered'), recording = path.join(source, 'recording'), catalog = path.join(root, 'catalog');
  for (const directory of [source, rendered, recording, catalog]) fs.mkdirSync(directory);
  const inputs = {};
  for (const name of PUBLIC_FILES) {
    const file = CAPTURE_FILES[name] ? path.join(recording, CAPTURE_FILES[name]) : path.join(rendered, name);
    fs.writeFileSync(file, `new:${name}`); inputs[name] = file;
    fs.writeFileSync(path.join(catalog, name), `old:${name}`);
  }
  fs.writeFileSync(path.join(catalog, 'README.md'), 'retained docs');
  const verify = candidate => { for (const name of PUBLIC_FILES) assert.equal(fs.readFileSync(path.join(candidate, name), 'utf8'), `new:${name}`); };
  try { run({ root, rendered, catalog, inputs, verify }); }
  finally { fs.rmSync(root, { recursive: true, force: true }); }
}
const oldCatalog = catalog => { for (const name of PUBLIC_FILES) assert.equal(fs.readFileSync(path.join(catalog, name), 'utf8'), `old:${name}`); };

test('complete capture publishes together with durable old/new receipts and retained docs', () => fixture(({ rendered, catalog, verify }) => {
  const result = publishDemo(rendered, catalog, { verify });
  for (const name of PUBLIC_FILES) {
    assert.equal(fs.readFileSync(path.join(catalog, name), 'utf8'), `new:${name}`);
    assert.equal(fs.readFileSync(path.join(result.journal, 'before', name), 'utf8'), `old:${name}`);
    assert.equal(fs.readFileSync(path.join(result.journal, 'retained', name), 'utf8'), `old:${name}`);
  }
  assert.equal(fs.readFileSync(path.join(catalog, 'README.md'), 'utf8'), 'retained docs');
  assert.ok(fs.readFileSync(path.join(result.journal, 'events.jsonl'), 'utf8').includes('complete'));
  assert.equal(fs.existsSync(path.join(catalog, '.audit-demo-publish.lock')), false);
}));

test('missing raw capture refuses all destination replacements', () => fixture(({ rendered, catalog, inputs, verify }) => {
  fs.unlinkSync(inputs['audit-recording.json']);
  assert.throws(() => publishDemo(rendered, catalog, { verify }), /publication stopped/); oldCatalog(catalog);
}));

test('changed source after verification refuses all destination replacements', () => fixture(({ rendered, catalog, inputs, verify }) => {
  assert.throws(() => publishDemo(rendered, catalog, { verify, observe: event => { if (event === 'verified') fs.writeFileSync(inputs['audit-readme.gif'], 'changed'); } }), /changed publication input/);
  oldCatalog(catalog);
}));

test('changed destination after verification is preserved', () => fixture(({ rendered, catalog, verify }) => {
  assert.throws(() => publishDemo(rendered, catalog, { verify, observe: event => { if (event === 'verified') fs.writeFileSync(path.join(catalog, 'audit-readme.gif'), 'unknown edit'); } }), /changed publication input/);
  assert.equal(fs.readFileSync(path.join(catalog, 'audit-readme.gif'), 'utf8'), 'unknown edit');
}));

test('destination symlink refuses publication without following it', () => fixture(({ root, rendered, catalog, verify }) => {
  const sentinel = path.join(root, 'sentinel'); fs.writeFileSync(sentinel, 'keep');
  fs.unlinkSync(path.join(catalog, 'audit-readme.gif')); fs.symlinkSync(sentinel, path.join(catalog, 'audit-readme.gif'));
  assert.throws(() => publishDemo(rendered, catalog, { verify }), /publication stopped/);
  assert.equal(fs.readFileSync(sentinel, 'utf8'), 'keep');
}));

test('source symlink refuses publication', () => fixture(({ root, rendered, catalog, inputs, verify }) => {
  const sentinel = path.join(root, 'sentinel'); fs.writeFileSync(sentinel, 'new:audit-readme.gif');
  fs.unlinkSync(inputs['audit-readme.gif']); fs.symlinkSync(sentinel, inputs['audit-readme.gif']);
  assert.throws(() => publishDemo(rendered, catalog, { verify }), /publication stopped/); oldCatalog(catalog);
}));

test('old predictable publishing symlink is neither followed nor removed', () => fixture(({ root, rendered, catalog, verify }) => {
  const sentinel = path.join(root, 'sentinel'); fs.writeFileSync(sentinel, 'keep');
  const link = path.join(catalog, '.audit-readme.gif.publishing'); fs.symlinkSync(sentinel, link);
  publishDemo(rendered, catalog, { verify });
  assert.equal(fs.readFileSync(sentinel, 'utf8'), 'keep'); assert.ok(fs.lstatSync(link).isSymbolicLink());
}));

test('existing operation refuses a competing publisher', () => fixture(({ rendered, catalog, verify }) => {
  fs.mkdirSync(path.join(catalog, '.audit-demo-publish.lock'));
  assert.throws(() => publishDemo(rendered, catalog, { verify }), /EEXIST/); oldCatalog(catalog);
}));

test('verification failure preserves the complete old catalog', () => fixture(({ rendered, catalog }) => {
  assert.throws(() => publishDemo(rendered, catalog, { verify: () => { throw new Error('pixel failure'); } }), /pixel failure/); oldCatalog(catalog);
}));

test('interruption retains every before-image and candidate and blocks unsafe retry', () => fixture(({ rendered, catalog, verify }) => {
  assert.throws(() => publishDemo(rendered, catalog, { verify, observe: event => { if (event === 'installed') throw new Error('simulated interruption'); } }), /simulated interruption/);
  const operation = JSON.parse(fs.readFileSync(path.join(catalog, '.audit-demo-publish.lock', 'operation.json')));
  for (const name of PUBLIC_FILES) {
    assert.equal(fs.readFileSync(path.join(operation.journal, 'before', name), 'utf8'), `old:${name}`);
    assert.equal(fs.readFileSync(path.join(operation.journal, 'candidate', name), 'utf8'), `new:${name}`);
  }
  assert.throws(() => publishDemo(rendered, catalog, { verify }), /EEXIST/);
}));

test('new destination appearing during replacement survives exclusive installation', () => fixture(({ rendered, catalog, verify }) => {
  assert.throws(() => publishDemo(rendered, catalog, { verify, observe: (event, context) => { if (event === 'before-install') fs.writeFileSync(path.join(catalog, context.name), 'concurrent new file'); } }), /EEXIST/);
  assert.equal(fs.readFileSync(path.join(catalog, 'audit-readme.gif'), 'utf8'), 'concurrent new file');
  const operation = JSON.parse(fs.readFileSync(path.join(catalog, '.audit-demo-publish.lock', 'operation.json')));
  assert.equal(fs.readFileSync(path.join(operation.journal, 'retained', 'audit-readme.gif'), 'utf8'), 'old:audit-readme.gif');
}));


test('redirected lock record is preserved even with identical bytes', () => fixture(({ root, rendered, catalog, verify }) => {
  let link;
  assert.throws(() => publishDemo(rendered, catalog, { verify, observe: event => {
    if (event !== 'verified') return;
    link = path.join(catalog, '.audit-demo-publish.lock', 'operation.json');
    const target = path.join(root, 'unknown-operation-copy');
    fs.writeFileSync(target, fs.readFileSync(link)); fs.unlinkSync(link); fs.symlinkSync(target, link);
  } }), /publication stopped/);
  assert.ok(fs.lstatSync(link).isSymbolicLink());
}));
