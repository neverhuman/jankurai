import fs from 'node:fs';
import path from 'node:path';
import { sha256 } from './audit-recording.mjs';

// Refuse cleanup if any sample input changed or unknown material appeared.
export function cleanupSample(directory, expected) {
  if (fs.realpathSync(directory) !== directory) throw new Error('preserving redirected sample');
  const files = new Map(expected.map(entry => [entry.name, entry.sha256]));
  const directories = new Set(['.']);
  for (const name of files.keys()) {
    let parent = path.dirname(name);
    while (parent !== '.') { directories.add(parent); parent = path.dirname(parent); }
  }
  const seen = new Set();
  const walk = name => {
    const full = path.join(directory, name), stat = fs.lstatSync(full);
    if (stat.isSymbolicLink()) throw new Error('preserving sample containing a symlink');
    if (stat.isDirectory()) {
      if (!directories.has(name)) throw new Error('preserving unknown sample directory');
      for (const child of fs.readdirSync(full)) walk(name === '.' ? child : path.join(name, child));
    } else if (stat.isFile() && files.get(name) === sha256(fs.readFileSync(full))) seen.add(name);
    else throw new Error('preserving changed or unknown sample input');
  };
  walk('.');
  if (seen.size !== files.size) throw new Error('preserving sample with missing input');
  fs.rmSync(directory, { recursive: true });
}
