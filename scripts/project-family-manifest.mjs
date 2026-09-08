import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { Family } from './family-model.mjs';
import { atomicWrite, exists, isLink } from './family-lib.mjs';

const digest = value => createHash('sha256').update(value).digest('hex');
try {
  const family = new Family(path.dirname(path.dirname(fileURLToPath(import.meta.url))));
  const authority = path.join(family.hub, 'repos.manifest.toml'), projection = path.join(family.root, 'repos.manifest.toml');
  if (isLink(authority) || fs.statSync(authority).nlink !== 1) throw new Error('authority must be a regular file with one link');
  const source = fs.readFileSync(authority, 'utf8'), hash = digest(source);
  const expected = `# GENERATED FILE — DO NOT EDIT\n# Authority: jankurai/repos.manifest.toml\n# Authority SHA-256: ${hash}\n\n${source}`;
  if (isLink(projection) || (exists(projection) && fs.statSync(projection).nlink !== 1)) throw new Error('refusing aliased projection');
  const previous = exists(projection) ? fs.readFileSync(projection, 'utf8') : undefined;
  if (process.argv.includes('--check')) {
    if (previous !== expected) throw new Error('runtime projection differs from tracked authority');
  } else if (process.argv.includes('--apply')) {
    if (previous !== expected && previous !== undefined) {
      const custody = path.join(family.root, '.bundles/manifest-projection-custody');
      if (isLink(custody) || isLink(path.dirname(custody))) throw new Error('refusing aliased custody directory');
      fs.mkdirSync(custody, { recursive: true, mode: 0o700 });
      const backup = path.join(custody, `${digest(previous)}.toml`);
      if (!exists(backup)) fs.writeFileSync(backup, previous, { flag: 'wx', mode: 0o600 });
      if (isLink(backup) || digest(fs.readFileSync(backup)) !== digest(previous)) throw new Error('custody verification failed');
      if (fs.readFileSync(projection, 'utf8') !== previous) throw new Error('projection changed during generation');
    }
    if (previous !== expected) atomicWrite(projection, expected);
  } else throw new Error('expected --check or --apply');
  console.log(`project-family-manifest: ok authority_sha256=${hash}`);
} catch (error) { console.error(`project-family-manifest: ${error.message}`); process.exitCode = 1; }
