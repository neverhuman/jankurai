import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import assert from 'node:assert/strict';
import test from 'node:test';

const candidate = fs.readFileSync(new URL('./family-build.mjs', import.meta.url), 'utf8');
const importLine = "import { clean, exists, gitText, run } from './family-lib.mjs';";
assert.equal(candidate.split(importLine).length, 2);
const cases = [
  ['clean', null, null],
  ['initial-dirty', 'initial', 'tracked'],
  ...['fusion', 'required', 'later-sibling'].flatMap(stage =>
    ['tracked', 'staged', 'untracked', 'head', 'head-and-edit', 'missing'].map(kind => [`${stage}-${kind}`, stage, kind])),
];
for (const [scenario, stage, kind] of cases) await test(scenario, async () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'family-preservation-repair-'));
  try {
    const directory = path.join(temp, 'component');
    const second = path.join(temp, 'second');
    const moved = path.join(temp, 'component-retained');
    fs.mkdirSync(directory);
    fs.mkdirSync(second);
    const gitAt = (repoDirectory, ...args) => {
      const result = spawnSync('/usr/bin/git', ['-C', repoDirectory, ...args], {
        encoding: 'utf8', env: { PATH: '/usr/bin:/bin', HOME: temp,
          GIT_CONFIG_NOSYSTEM: '1', GIT_CONFIG_GLOBAL: '/dev/null',
          GIT_AUTHOR_NAME: 'fixture', GIT_AUTHOR_EMAIL: 'fixture@example.invalid',
          GIT_COMMITTER_NAME: 'fixture', GIT_COMMITTER_EMAIL: 'fixture@example.invalid' },
      });
      assert.equal(result.status, 0, result.stderr);
      return result.stdout.trim();
    };
    const git = (...args) => gitAt(fs.existsSync(directory) ? directory : moved, ...args);
    git('init', '--quiet');
    let tracked = path.join(directory, 'owned.txt');
    let untracked = path.join(directory, 'unknown.txt');
    fs.writeFileSync(tracked, 'committed baseline\n');
    git('add', 'owned.txt'); git('commit', '--quiet', '-m', 'fixture baseline');
    const expectedHead = git('rev-parse', 'HEAD');
    gitAt(second, 'init', '--quiet');
    fs.writeFileSync(path.join(second, 'owned.txt'), 'second component baseline\n');
    gitAt(second, 'add', 'owned.txt'); gitAt(second, 'commit', '--quiet', '-m', 'second fixture');
    const secondHead = gitAt(second, 'rev-parse', 'HEAD');
    const edit = 'unreviewed tracked change must survive\n';
    let stateAfterInjection;
    const snapshot = () => ({
      head: git('rev-parse', 'HEAD'), status: git('status', '--porcelain'),
      index: git('show', ':owned.txt'), bytes: fs.readFileSync(tracked, 'utf8'),
      untracked: fs.existsSync(untracked) ? fs.readFileSync(untracked, 'utf8') : null,
    });
    const inject = () => {
      if (kind.startsWith('head')) {
        fs.writeFileSync(tracked, 'new committed head\n');
        git('add', 'owned.txt'); git('commit', '--quiet', '-m', 'concurrent head');
      }
      if (['tracked', 'staged', 'head-and-edit'].includes(kind)) fs.writeFileSync(tracked, edit);
      if (kind === 'staged') git('add', 'owned.txt');
      if (kind === 'untracked') fs.writeFileSync(untracked, 'unknown owner bytes\n');
      if (kind === 'missing') {
        fs.renameSync(directory, moved);
        tracked = path.join(moved, 'owned.txt');
        untracked = path.join(moved, 'unknown.txt');
      }
      stateAfterInjection = snapshot();
    };
    if (stage === 'initial') inject();
    const events = [];
    globalThis.preservationRepair = {
      exists: fs.existsSync,
      clean: repoDirectory => { if (gitAt(repoDirectory, 'status', '--porcelain')) throw new Error('dirty checkout'); },
      gitText: (repoDirectory, ...args) => gitAt(repoDirectory, ...args),
      run: (args, options) => {
        events.push(args);
        if (stage === 'fusion' && args[0] === 'cargo' && args[1] === 'test') inject();
        if (stage === 'required' && args[1] === 'scripts/ci-local.sh' && options.cwd === directory) inject();
        if (stage === 'later-sibling' && args[1] === 'scripts/ci-local.sh' && options.cwd === second) inject();
        assert.notEqual(args[0], 'git', 'acceptance must never restore repository files');
      },
    };
    const module = await import('data:text/javascript;base64,' + Buffer.from(candidate.replace(
      importLine, 'const { clean, exists, gitText, run } = globalThis.preservationRepair;'
    ) + '\n// ' + scenario).toString('base64'));
    const component = { name: 'component' };
    const other = { name: 'second' };
    const repoPath = repo => repo.name === component.name ? directory : second;
    const family = { root: temp, hub: temp, fusion: temp, allowLiveRequired: true,
      components: () => [component, other], existing: repo => fs.existsSync(repoPath(repo)), path: repoPath,
      executionPath: repoPath, rematerializeIsolates() {}, disposeIsolates() {},
      pins: new Map([['component', { commit: expectedHead }], ['second', { commit: secondHead }]]), bootstrap() {}, fuse() {} };
    let error;
    try { module.check(family); } catch (failure) { error = failure.message; }
    if (stage === null) {
      assert.equal(error, undefined);
      assert(events.some(args => args[1] === 'ops/ci/integration.sh'));
      assert(events.some(args => args[1] === 'scripts/ci-local.sh'));
    } else {
      assert(error, scenario + ' must fail acceptance');
      assert.deepEqual(snapshot(), stateAfterInjection, scenario + ' must preserve head/index/files');
      if (stage === 'initial' || stage === 'fusion') assert(!events.some(args => args[1] === 'ops/ci/integration.sh'));
      if (kind === 'missing') assert.match(error, /missing component/);
    }
  } finally {
    fs.rmSync(temp, { recursive: true, force: true });
    assert.equal(fs.existsSync(temp), false);
  }
});
