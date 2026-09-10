import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const requiredParents = ['fast', 'integration', 'release-build'];
export const requiredJobs = [
  'fast',
  'integration',
  'release-build (ubuntu-24.04, x86_64-unknown-linux-gnu)',
  'release-build (macos-14, aarch64-apple-darwin)',
];

export function parseUniqueJson(raw) {
  const value = JSON.parse(raw);
  // JSON.parse silently replaces duplicate keys. Inspect the already valid
  // string/punctuation tokens, including escaped spellings of the same key.
  const scopes = [];
  for (const token of raw.match(/"(?:\\[\s\S]|[^"\\])*"|[{}\[\],:]/g) ?? []) {
    if (token === '{' || token === '[') {
      scopes.push({ object: token === '{', key: token === '{', seen: new Set() });
    } else if (token === '}' || token === ']') {
      scopes.pop();
    } else {
      const scope = scopes.at(-1);
      if (token === ',' && scope?.object) scope.key = true;
      if (token.startsWith('"') && scope?.object && scope.key) {
        const key = JSON.parse(token);
        if (scope.seen.has(key)) throw new Error(`duplicate JSON key: ${key}`);
        scope.seen.add(key);
        scope.key = false;
      }
    }
  }
  return value;
}

export function validateParents(jobs) {
  if (!jobs || Array.isArray(jobs) || typeof jobs !== 'object' ||
      JSON.stringify(Object.keys(jobs).sort()) !== JSON.stringify(requiredParents)) {
    throw new Error('required parent inventory must be exactly fast, integration, release-build');
  }
  for (const name of requiredParents) {
    const job = jobs[name];
    if (!job || Array.isArray(job) || typeof job !== 'object' || job.result !== 'success') {
      throw new Error(`required parent did not succeed: ${name}`);
    }
  }
}

export function validateExpandedJobs(payload, runId, head) {
  if (!/^[1-9][0-9]*$/.test(runId) || !/^[a-f0-9]{40}$/.test(head)) {
    throw new Error('missing valid workflow run/source identity');
  }
  if (!payload || !Array.isArray(payload.jobs) || !Number.isSafeInteger(payload.total_count) ||
      payload.total_count !== payload.jobs.length || payload.total_count === 0) {
    throw new Error('incomplete or malformed hosted job inventory');
  }
  const names = new Set(), ids = new Set();
  for (const job of payload.jobs) {
    if (!job || typeof job !== 'object' || !Number.isSafeInteger(job.id) || job.id <= 0 ||
        String(job.run_id) !== runId || job.head_sha !== head ||
        typeof job.name !== 'string' || ids.has(job.id) || names.has(job.name)) {
      throw new Error('duplicate, malformed, or foreign hosted job');
    }
    ids.add(job.id);
    names.add(job.name);
    if (requiredJobs.includes(job.name)) {
      if (job.status !== 'completed' || job.conclusion !== 'success') {
        throw new Error(`required hosted job did not succeed: ${job.name}`);
      }
    } else if (!['jankurai/required', 'publish-ci-tag'].includes(job.name)) {
      throw new Error(`unexpected hosted job: ${job.name}`);
    }
  }
  for (const name of requiredJobs) {
    if (!names.has(name)) throw new Error(`missing required hosted job: ${name}`);
  }
}

function main(args) {
  if (args.length > 1 || (args.length === 1 && args[0] !== '--hosted')) {
    throw new Error('usage: aggregate.sh [--hosted]');
  }
  validateParents(parseUniqueJson(process.env.NEEDS_JSON ?? ''));
  if (args[0] === '--hosted') {
    const { GITHUB_REPOSITORY: repo, GITHUB_RUN_ID: runId, EXPECTED_HEAD_SHA: head, GH_TOKEN: token } = process.env;
    if (!repo || !/^[\w.-]+\/[\w.-]+$/.test(repo) || !/^[1-9][0-9]*$/.test(runId ?? '') ||
        !/^[a-f0-9]{40}$/.test(head ?? '') || !token) {
      throw new Error('hosted aggregate requires the authenticated run/source identity');
    }
    // The expected inventory is smaller than one page. Reject truncation or
    // unexpected expansion, rather than trusting an incomplete page of jobs.
    const result = spawnSync('gh', ['api', '--hostname', 'github.com', `repos/${repo}/actions/runs/${runId}/jobs?filter=latest&per_page=100`], {
      encoding: 'utf8', timeout: 20000, maxBuffer: 4 * 1024 * 1024,
    });
    if (result.error || result.status !== 0) throw new Error('cannot read actual hosted job inventory');
    validateExpandedJobs(parseUniqueJson(result.stdout), runId, head);
  }
  console.log('complete required lane inventory passed');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
