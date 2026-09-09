import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

try {
  const dist = process.argv[2], version = fs.readFileSync('VERSION', 'utf8').trim();
  const assets = ['family.lock', 'Cargo.lock', 'jankurai-installer.sh', `jankurai-ux-qa-${version}.tgz`];
  for (const target of ['x86_64-unknown-linux-gnu', 'aarch64-apple-darwin']) {
    assets.push(`provenance-${target}.json`);
    for (const product of ['jankurai', 'tuiwright']) {
      const stem = `${product}-${version}-${target}`, name = `${stem}.tar.gz`;
      assets.push(name);
      const entries = execFileSync('tar', ['-tzf', path.join(dist, name)], { encoding: 'utf8' }).trim().split('\n').map(entry => entry.replace(/\/$/, '')).sort();
      const allowed = [stem, ...[product, 'LICENSE', 'family.lock', 'Cargo.lock', 'provenance.json'].map(file => `${stem}/${file}`)].sort();
      if (JSON.stringify(entries) !== JSON.stringify(allowed)) throw new Error(`unexpected payload: ${name}`);
      const details = execFileSync('tar', ['-tvzf', path.join(dist, name)], { encoding: 'utf8' }).trim().split('\n');
      if (details.some(entry => !/^[d-]/.test(entry))) throw new Error(`linked/special payload: ${name}`);
    }
  }
  const expected = assets.flatMap(name => [name, `${name}.sha256`, `${name}.sigstore.bundle`, `${name}.attestation.jsonl`]).sort();
  if (JSON.stringify(fs.readdirSync(dist).sort()) !== JSON.stringify(expected)) throw new Error('unexpected release asset inventory');
  for (const name of expected) {
    if (!fs.lstatSync(path.join(dist, name)).isFile()) throw new Error(`non-regular release asset: ${name}`);
  }
  for (const name of assets) {
    const file = path.join(dist, name), digest = createHash('sha256').update(fs.readFileSync(file)).digest('hex');
    if (fs.readFileSync(`${file}.sha256`, 'utf8') !== `${digest}  ${name}\n`) throw new Error(`checksum mismatch: ${name}`);
    if (!fs.statSync(`${file}.sigstore.bundle`).size) throw new Error(`missing signature: ${name}`);
    if (!fs.statSync(`${file}.attestation.jsonl`).size) throw new Error(`missing attestation bundle: ${name}`);
  }
  console.log(`verified release inventory: ${assets.length} products and metadata files`);
} catch (error) { console.error(`release inventory: ${error.message}`); process.exitCode = 1; }
