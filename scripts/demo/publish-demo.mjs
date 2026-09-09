#!/usr/bin/env node
// Copy a verified render into the public docs/demo catalog.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { PUBLIC_FILES } from './demo-catalog.mjs';

const [source, destination] = process.argv.slice(2);
if (!source || !destination) throw new Error('usage: publish-demo.mjs RENDERED-DIR DOCS-DEMO-DIR');
const rendered = path.resolve(source);
const catalog = path.resolve(destination);
const verify = spawnSync(process.execPath, [fileURLToPath(new URL('./verify-audit-gif.mjs', import.meta.url)), rendered], { stdio: 'inherit' });
if (verify.status !== 0) throw new Error('refusing to publish an unverified render');
fs.mkdirSync(catalog, { recursive: true });
for (const name of PUBLIC_FILES) {
  const raw = fs.readFileSync(path.join(rendered, name));
  const staged = path.join(catalog, `.${name}.publishing`);
  fs.writeFileSync(staged, raw);
  fs.renameSync(staged, path.join(catalog, name));
}
console.log(`published ${PUBLIC_FILES.join(', ')} -> ${catalog}`);
