import path from 'node:path';
import { clean, exists, gitText, run } from './family-lib.mjs';

export function dependencies(family) {
  run(['cargo', 'fetch', '--locked'], { cwd: family.fusion });
  for (const repo of family.components()) {
    const live = family.path(repo);
    const isolated = path.join(family.fusion, 'components', repo.name);
    const directories = [live];
    if (exists(isolated) && isolated !== live) directories.push(isolated);
    for (const directory of directories) {
      if (exists(path.join(directory, 'package-lock.json'))) run(['npm', 'ci'], { cwd: directory });
    }
  }
}
export function build(family, { release = false, target, isolate = false } = {}) {
  family.bootstrap();
  family.fuse(true, isolate);
  const args = ['cargo', 'build', '--locked', '-p', 'jankurai', '-p', 'tuiwright-cli'];
  if (release) args.push('--release');
  if (target) args.push('--target', target);
  run(args, { cwd: family.fusion });
  const ux = path.join(family.root, 'jankurai-tools-ux');
  if (!exists(path.join(ux, 'node_modules'))) run(['npm', 'ci'], { cwd: ux });
  run(['npm', 'run', 'build'], { cwd: ux });
}
export function check(family) {
  for (const repo of family.components()) {
    if (!family.existing(repo)) continue;
    clean(family.path(repo));
    if (gitText(family.path(repo), 'rev-parse', 'HEAD') !== family.pins.get(repo.name).commit) throw new Error(`${repo.name}: integration requires the accepted locked revision`);
  }
  // Isolate fusion members so workspace cargo test cannot write through
  // live sibling checkouts. Live trees stay the accepted source.
  build(family, { isolate: true });
  dependencies(family);
  run(['cargo', 'test', '--workspace', '--locked'], { cwd: family.fusion });
  const ux = path.join(family.root, 'jankurai-tools-ux');
  run(['npm', 'exec', '--', 'playwright', 'install', 'chromium', '--only-shell'], { cwd: ux });
  run(['npm', 'test'], { cwd: ux });
  const env = { ...process.env, PATH: path.join(family.fusion, 'target/debug') + path.delimiter + process.env.PATH };
  // Isolated copies absorb workspace tests. Live trees must stay the accepted
  // source; any mutation there is preserved and refuses further acceptance.
  for (const repo of family.components()) {
    if (!family.existing(repo)) throw new Error(`missing component: ${repo.name}`);
    clean(family.path(repo));
    if (gitText(family.path(repo), 'rev-parse', 'HEAD') !== family.pins.get(repo.name).commit) {
      throw new Error(`${repo.name}: integration requires the accepted locked revision`);
    }
  }
  run(['bash', 'ops/ci/integration.sh'], { cwd: family.hub, env });
  for (const repo of family.components()) {
    const live = family.path(repo);
    const isolated = path.join(family.fusion, 'components', repo.name);
    // Prefer the isolated fusion copy when check() materialized one. Portable
    // preservation tests mock fuse() and keep cwd on the live fixture.
    const directory = exists(isolated) ? isolated : live;
    // Component required lanes run cargo --offline. Prefetch each lockfile so
    // alternate Git sources (for example core's www.github.com kernel pin) are
    // already in CARGO_HOME.
    if (exists(path.join(directory, 'Cargo.lock'))) run(['cargo', 'fetch', '--locked'], { cwd: directory });
    run(['bash', 'scripts/ci-local.sh', 'required'], { cwd: directory, env });
  }
  // Required lanes must not mutate accepted live trees. Recheck every
  // component after all lanes, including changes to an earlier sibling.
  for (const repo of family.components()) {
    if (!family.existing(repo)) throw new Error(`missing component: ${repo.name}`);
    clean(family.path(repo));
    if (gitText(family.path(repo), 'rev-parse', 'HEAD') !== family.pins.get(repo.name).commit) {
      throw new Error(`${repo.name}: integration requires the accepted locked revision`);
    }
  }
}
