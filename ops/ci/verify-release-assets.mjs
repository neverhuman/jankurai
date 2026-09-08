import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { run } from '../../scripts/family-lib.mjs';

try {
  const dist = process.argv[2], version = fs.readFileSync('VERSION', 'utf8').trim();
  const assets = ['family.lock', 'Cargo.lock', 'jankurai-installer.sh', `jankurai-ux-qa-${version}.tgz`];
  for (const target of ['x86_64-unknown-linux-gnu', 'aarch64-apple-darwin']) {
    assets.push(`provenance-${target}.json`);
    for (const product of ['jankurai', 'tuiwright']) {
      const stem = `${product}-${version}-${target}`, name = `${stem}.tar.gz`;
      assets.push(name);
      const entries = run(['tar', '-tzf', path.join(dist, name)], { capture: true }).split('\n').map(entry => entry.replace(/\/$/, '')).sort();
      const allowed = [stem, ...[product, 'LICENSE', 'family.lock', 'Cargo.lock', 'provenance.json'].map(file => `${stem}/${file}`)].sort();
      if (JSON.stringify(entries) !== JSON.stringify(allowed)) throw new Error(`unexpected payload: ${name}`);
      const details = run(['tar', '-tvzf', path.join(dist, name)], { capture: true }).split('\n');
      if (details.some(entry => !/^[d-]/.test(entry))) throw new Error(`linked/special payload: ${name}`);
    }
  }
  for (const name of assets) {
    const file = path.join(dist, name), digest = createHash('sha256').update(fs.readFileSync(file)).digest('hex');
    if (fs.readFileSync(`${file}.sha256`, 'utf8') !== `${digest}  ${name}\n`) throw new Error(`checksum mismatch: ${name}`);
    if (!fs.statSync(`${file}.sigstore.bundle`).size) throw new Error(`missing signature: ${name}`);
  }
  console.log(`verified release inventory: ${assets.length} products and metadata files`);
} catch (error) { console.error(`release inventory: ${error.message}`); process.exitCode = 1; }
