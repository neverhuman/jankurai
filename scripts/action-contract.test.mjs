import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';

test('marketplace action blocks under a configurable score floor', () => {
  const action = fs.readFileSync(new URL('../action.yml', import.meta.url), 'utf8');
  assert.match(action, /fail-under:/);
  assert.match(action, /default: "85"/);
  assert.match(action, /default: "standard"/);
  assert.match(action, /--fail-under/);
  assert.match(action, /Enforce score floor/);
  assert.match(action, /\.score >= \$floor/);
});
