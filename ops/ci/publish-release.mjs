import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

function request(endpoint, body, method, missing = false) {
  const args = ['api', endpoint];
  if (method) args.push('--method', method);
  if (body !== undefined) args.push('--input', '-');
  const result = spawnSync('gh', args, { encoding: 'utf8', input: body === undefined ? undefined : JSON.stringify(body) });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    if (missing && /\(HTTP 404\)/.test(result.stderr)) return null;
    throw new Error(`GitHub request failed: ${result.stderr.trim()}`);
  }
  return result.stdout.trim() ? JSON.parse(result.stdout) : undefined;
}
function upload(repository, tag, file) {
  const result = spawnSync('gh', ['release', 'upload', tag, file, '--repo', repository], { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error('release upload failed; verified draft assets are preserved for retry');
}
export function publishRelease({ repository, tag, commit, version, directory, notes }, api = request, put = upload) {
  if (repository !== 'neverhuman/jankurai' || tag !== `v${version}` || !/^[0-9a-f]{40}$/.test(commit)) throw new Error('invalid release identity');
  const prefix = `repos/${repository}`;
  const verifyTag = () => {
    let object = api(`${prefix}/git/ref/tags/${tag}`).object;
    for (let depth = 0; object.type === 'tag' && depth < 4; depth++) object = api(`${prefix}/git/tags/${object.sha}`).object;
    if (object.type !== 'commit' || object.sha !== commit) throw new Error('release tag does not identify the verified source commit');
  };
  verifyTag();
  const expected = new Map(fs.readdirSync(directory).sort().map(name => {
    const file = path.join(directory, name);
    if (!fs.lstatSync(file).isFile()) throw new Error(`non-regular release asset: ${name}`);
    return [name, { file, digest: 'sha256:' + createHash('sha256').update(fs.readFileSync(file)).digest('hex'), size: fs.statSync(file).size }];
  }));
  if (!expected.size) throw new Error('empty release inventory');
  let release = api(`${prefix}/releases/tags/${tag}`, undefined, undefined, true);
  if (!release) release = api(`${prefix}/releases`, {
    tag_name: tag, target_commitish: commit, draft: true, prerelease: false,
    name: `Jankurai ${tag}`, body: notes,
  }, 'POST');
  if (release.tag_name !== tag) throw new Error('release tag mismatch');
  const checkAssets = complete => {
    const seen = new Set();
    for (let page = 1; ; page++) {
      const assets = api(`${prefix}/releases/${release.id}/assets?per_page=100&page=${page}`);
      for (const asset of assets) {
        const wanted = expected.get(asset.name);
        if (!wanted || seen.has(asset.name) || asset.state !== 'uploaded' || asset.digest !== wanted.digest || asset.size !== wanted.size) {
          throw new Error(`existing release asset differs from verified candidate: ${asset.name}`);
        }
        seen.add(asset.name);
      }
      if (assets.length < 100) break;
    }
    if (complete && seen.size !== expected.size) throw new Error('release asset inventory is incomplete');
    return seen;
  };
  if (!release.draft) {
    checkAssets(true);
    if (!release.immutable || release.prerelease) throw new Error('existing release is not an immutable stable release');
    return release;
  }
  const existing = checkAssets(false);
  for (const [name, asset] of expected) if (!existing.has(name)) put(repository, tag, asset.file);
  checkAssets(true);
  verifyTag();
  release = api(`${prefix}/releases/${release.id}`, { draft: false, prerelease: false, make_latest: 'true' }, 'PATCH');
  if (release.draft || release.prerelease || !release.immutable) throw new Error('published release did not become immutable and stable');
  return release;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const release = publishRelease({ repository: process.env.GITHUB_REPOSITORY, tag: process.env.RELEASE_TAG,
      commit: process.env.GITHUB_SHA, version: fs.readFileSync('VERSION', 'utf8').trim(), directory: 'dist',
      notes: fs.readFileSync('docs/release-notes.md', 'utf8') });
    console.log(`Verified immutable release: ${release.html_url}`);
  } catch (error) { console.error(`release publication: ${error.message}`); process.exitCode = 1; }
}
