import path from 'node:path';
import { clean, exists, gitText, run } from './family-lib.mjs';

export function dependencies(family) {
  run(['cargo', 'fetch', '--locked'], { cwd: family.fusion });
  for (const repo of family.components()) {
    if (exists(path.join(family.path(repo), 'package-lock.json'))) run(['npm', 'ci'], { cwd: family.path(repo) });
  }
}
export function build(family, { release = false, target } = {}) {
  family.bootstrap();
  family.fuse();
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
  build(family);
  dependencies(family);
  run(['cargo', 'test', '--workspace', '--locked'], { cwd: family.fusion });
  const ux = path.join(family.root, 'jankurai-tools-ux');
  run(['npm', 'exec', '--', 'playwright', 'install', 'chromium', '--only-shell'], { cwd: ux });
  run(['npm', 'test'], { cwd: ux });
  const env = { ...process.env, PATH: path.join(family.fusion, 'target/debug') + path.delimiter + process.env.PATH };
  // Fusion members are live checkouts. Workspace cargo test can rewrite
  // sibling Cargo.lock files through those path links. Restore tracked files
  // so hub integration still sees the accepted locked pins.
  for (const repo of family.components()) {
    if (!family.existing(repo)) continue;
    run(['git', '-C', family.path(repo), 'checkout', '--', '.']);
    if (gitText(family.path(repo), 'rev-parse', 'HEAD') !== family.pins.get(repo.name).commit) {
      throw new Error(`${repo.name}: integration requires the accepted locked revision`);
    }
  }
  run(['bash', 'ops/ci/integration.sh'], { cwd: family.hub, env });
  for (const repo of family.components()) {
    const directory = family.path(repo);
    // Component required lanes run cargo --offline. Prefetch each lockfile so
    // alternate Git sources (for example core's www.github.com kernel pin) are
    // already in CARGO_HOME.
    if (exists(path.join(directory, 'Cargo.lock'))) run(['cargo', 'fetch', '--locked'], { cwd: directory });
    run(['bash', 'scripts/ci-local.sh', 'required'], { cwd: directory, env });
  }
}
