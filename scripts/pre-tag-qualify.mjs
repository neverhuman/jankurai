#!/usr/bin/env node
// Local pre-tag gate: require successful release-services evidence; refuse release.yml identity.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const SUCCESS_MARKER =
  'Real anonymous signatures and attestations verified; modified content and wrong repository/workflow/source/tag rejected.';

const RELEASE_SERVICES_WORKFLOW = '.github/workflows/release-services.yml';
const RELEASE_WORKFLOW = '.github/workflows/release.yml';

function walkFiles(directory, files = []) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) walkFiles(full, files);
    else if (entry.isFile()) files.push(full);
  }
  return files;
}

export function qualifyPreTag(evidenceDir) {
  if (!evidenceDir) throw new Error('usage: node scripts/pre-tag-qualify.mjs <evidence-dir>');
  const root = path.resolve(evidenceDir);
  if (!fs.existsSync(root) || !fs.statSync(root).isDirectory()) {
    throw new Error(`pre-tag-qualify: evidence directory missing: ${root}`);
  }
  const resultFile = path.join(root, 'result.txt');
  if (!fs.existsSync(resultFile)) {
    // Also accept nested release-services artifact layouts.
    const matches = walkFiles(root).filter(file => path.basename(file) === 'result.txt');
    if (!matches.length) throw new Error('pre-tag-qualify: missing release-services result.txt (fail closed)');
    return qualifyResultFiles(matches);
  }
  return qualifyResultFiles([resultFile]);
}

function qualifyResultFiles(files) {
  let accepted = false;
  for (const file of files) {
    const text = fs.readFileSync(file, 'utf8');
    if (text.includes(RELEASE_WORKFLOW) && !text.includes(RELEASE_SERVICES_WORKFLOW)) {
      throw new Error('pre-tag-qualify: refusing release.yml identity as release-services evidence');
    }
    // Explicit identity files / sidecar claims
    const sidecar = path.join(path.dirname(file), 'identity.txt');
    if (fs.existsSync(sidecar)) {
      const identity = fs.readFileSync(sidecar, 'utf8');
      if (identity.includes(RELEASE_WORKFLOW) && !identity.includes(RELEASE_SERVICES_WORKFLOW)) {
        throw new Error('pre-tag-qualify: refusing release.yml identity as release-services identity');
      }
      if (identity.includes(RELEASE_WORKFLOW) && identity.includes('release-services')) {
        throw new Error('pre-tag-qualify: refusing release.yml identity as release-services identity');
      }
    }
    if (text.includes(SUCCESS_MARKER) || text.trim() === SUCCESS_MARKER.trim()) {
      // Prefer evidence that documents release-services when identity is present.
      if (text.includes(RELEASE_WORKFLOW) && text.includes('cert-identity') && !text.includes(RELEASE_SERVICES_WORKFLOW)) {
        throw new Error('pre-tag-qualify: refusing release.yml identity as release-services identity');
      }
      accepted = true;
    }
  }
  if (!accepted) throw new Error('pre-tag-qualify: result.txt does not record a successful release-services verification');
  return { ok: true, results: files };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const result = qualifyPreTag(process.argv[2]);
    console.log(`pre-tag-qualify: ok (${result.results.length} result.txt)`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
