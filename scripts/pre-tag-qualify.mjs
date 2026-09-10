#!/usr/bin/env node
// Local pre-tag gate: require successful release-services evidence bound to
// workflow run / source / artifact / signature identity; refuse release.yml masquerade.
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

function readIfFile(file) {
  try {
    if (fs.existsSync(file) && fs.statSync(file).isFile()) return fs.readFileSync(file, 'utf8');
  } catch { /* missing */ }
  return null;
}

function refuseReleaseYml(text) {
  if (!text.includes(RELEASE_WORKFLOW)) return;
  if (text.includes(`/${RELEASE_WORKFLOW}@`) || /cert-identity=.*\.github\/workflows\/release\.yml/i.test(text)) {
    throw new Error('pre-tag-qualify: refusing release.yml identity as release-services identity');
  }
  if (!text.includes(RELEASE_SERVICES_WORKFLOW)) {
    throw new Error('pre-tag-qualify: refusing release.yml identity as release-services evidence');
  }
}

function requireRegularNonempty(file, label) {
  if (!fs.existsSync(file) || !fs.statSync(file).isFile() || fs.statSync(file).size < 1) {
    throw new Error(`pre-tag-qualify: missing ${label}`);
  }
}

/** Bind workflow run / source / artifact / signature; refuse lone SUCCESS_MARKER. */
function bindEvidence(resultFile, text) {
  const dir = path.dirname(resultFile);
  const identityText = readIfFile(path.join(dir, 'identity.txt')) ?? '';
  refuseReleaseYml(text);
  refuseReleaseYml(identityText);

  if (!(text.includes(SUCCESS_MARKER) || text.trim() === SUCCESS_MARKER.trim())) {
    return false;
  }

  const combined = `${text}\n${identityText}`;
  const cert = /(?:^|\n)cert-identity=(https:\/\/github\.com\/[^\s]+)(?:\n|$)/.exec(combined);
  if (!cert) {
    throw new Error('pre-tag-qualify: missing release-services cert-identity (workflow run identity)');
  }
  if (!cert[1].includes(RELEASE_SERVICES_WORKFLOW) || cert[1].includes(RELEASE_WORKFLOW)) {
    throw new Error('pre-tag-qualify: refusing non-release-services workflow identity');
  }

  const source = /(?:^|\n)source=([0-9a-f]{40})(?:\n|$)/.exec(combined);
  if (!source) {
    throw new Error('pre-tag-qualify: missing source identity binding');
  }

  const artifactName = /(?:^|\n)artifact=([^\s]+)(?:\n|$)/.exec(combined)?.[1] ?? 'probe.txt';
  const artifactPath = path.join(dir, artifactName);
  requireRegularNonempty(artifactPath, `artifact identity file: ${artifactName}`);
  const artifactText = fs.readFileSync(artifactPath, 'utf8');
  const artifactSource = /(?:^|\n)source=([0-9a-f]{40})(?:\n|$)/.exec(`\n${artifactText}\n`);
  if (!artifactSource) {
    throw new Error('pre-tag-qualify: artifact missing source identity');
  }
  if (artifactSource[1] !== source[1]) {
    throw new Error('pre-tag-qualify: artifact source digest disagrees with identity binding');
  }

  requireRegularNonempty(path.join(dir, `${artifactName}.sigstore.bundle`), 'signature bundle identity');
  requireRegularNonempty(path.join(dir, `${artifactName}.attestation.jsonl`), 'attestation / signature identity');
  return true;
}

export function qualifyPreTag(evidenceDir) {
  if (!evidenceDir) throw new Error('usage: node scripts/pre-tag-qualify.mjs <evidence-dir>');
  const root = path.resolve(evidenceDir);
  if (!fs.existsSync(root) || !fs.statSync(root).isDirectory()) {
    throw new Error(`pre-tag-qualify: evidence directory missing: ${root}`);
  }
  const resultFile = path.join(root, 'result.txt');
  if (!fs.existsSync(resultFile)) {
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
    if (bindEvidence(file, text)) accepted = true;
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
