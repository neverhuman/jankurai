// Invoked only inside temporaryCI's cleanup guard and sanitized environment.
import fs from 'node:fs';
import path from 'node:path';
import { Family } from './family-model.mjs';
import { atomicWrite, git, gitText, run } from './family-lib.mjs';

const [source, commit, directory] = process.argv.slice(2);
try {
  if (!/^[0-9a-f]{40}$/.test(commit)) throw new Error('invalid candidate source commit');
  const hub = path.join(directory, 'jankurai');
  run(['git', 'clone', '--no-local', '--no-hardlinks', '--no-checkout', source, hub]);
  git(hub, ['checkout', '--detach', commit]);
  if (gitText(hub, 'rev-parse', 'HEAD') !== commit) throw new Error('candidate source commit mismatch');
  atomicWrite(path.join(hub, 'family.lock'), fs.readFileSync(path.join(directory, 'candidate.lock'), 'utf8'));
  const sandbox = new Family(hub);
  sandbox.bootstrap(true);
  sandbox.fuse(false);
  run(['cargo', 'generate-lockfile'], { cwd: sandbox.fusion });
  atomicWrite(path.join(hub, 'Cargo.lock'), fs.readFileSync(path.join(sandbox.fusion, 'Cargo.lock'), 'utf8'));
  run(['bash', 'scripts/family.sh', 'check'], { cwd: hub });
} catch (error) { console.error(`candidate validation: ${error.message}`); process.exitCode = 1; }
