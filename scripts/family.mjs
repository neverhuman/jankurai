#!/usr/bin/env node
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Family } from './family-model.mjs';
import { operation, run } from './family-lib.mjs';
import { dependencies, build, check } from './family-build.mjs';
import { update } from './family-update.mjs';

try {
  const family = new Family(path.dirname(path.dirname(fileURLToPath(import.meta.url))));
  const [command, ...args] = process.argv.slice(2);
  if (command === 'status') family.status();
  else if (command === 'validate') console.log('validate-family: ok');
  else operation(family.hub, () => {
    switch (command) {
      case 'setup': family.bootstrap(true); family.fuse(); dependencies(family); break;
      case 'build': build(family, { release: args.includes('--release'), target: args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined }); break;
      case 'test':
      case 'check': check(family); break;
      case 'version': family.bootstrap(); family.fuse(); run(['cargo', 'run', '--locked', '-p', 'jankurai', '--', '--version'], { cwd: family.fusion }); break;
      case 'fuse': family.bootstrap(); family.fuse(); break;
      case 'pull': update(family); break;
      default: throw new Error('usage: family.sh {setup|pull|build|check|status|fuse|validate} [--release] [--target triple]');
    }
  });
} catch (error) { console.error(`family: ${error.message}`); process.exitCode = 1; }
