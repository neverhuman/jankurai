# Bad DOCKER Behavior: Comprehensive Guide

This document organizes the worst DOCKER behaviors that are inexcusable in production.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core DOCKER best practices:

- **Keep images small and secure**: Use minimal base images (e.g., Alpine or Distroless) and multi-stage builds.
- **Run as non-root**: Always define a `USER` instruction to limit privilege escalation risks.
- **Avoid embedding secrets**: Pass secrets at runtime or build-time via safe mechanisms, never store them in layers.
- **Pin base image tags**: Use specific versions or SHA digests instead of `latest`.
- **Optimize build caching**: Order Dockerfile instructions to maximize layer caching (copy dependency files before source code).

## 10.1 Dockerfile policy

1. Trusted base image.
2. Specific tag or digest.
3. `.dockerignore`.
4. Lockfile copied before source.
5. Dependency install before app copy.
6. Multi-stage build where applicable.
7. No secrets in layers.
8. Non-root `USER`.
9. Minimal runtime image.
10. Healthcheck for long-running services.
11. Exec-form `ENTRYPOINT` or `CMD`.
12. No unnecessary packages.
13. Package-manager cache cleanup.
14. BuildKit cache mounts for slow package managers.
15. Rebuild process using fresh base images.

## 10.3 CI/CD policy

1. `privileged = true` on shared runners.
2. Docker socket mounts.
3. `docker:dind` for untrusted PRs.
4. `docker:latest`.
5. Registry push from unprotected branches.
6. Secrets available to forked PRs.
7. Long-lived registry tokens.
8. No image scanning before push.
9. No secret scanning.
10. No digest output after build.
11. No SBOM/provenance.
12. No cache isolation.
13. Persistent self-hosted runner for untrusted workloads.
14. Cloud metadata access from build containers.
15. Plain HTTP Docker daemon access.
16. Ephemeral runners.
17. Protected branches/tags for release.
18. Pinned Docker/BuildKit versions.
19. Rootless builders where practical.
20. Isolated privileged runners only when unavoidable.
21. Registry-backed build cache.
22. Short-lived OIDC credentials.
23. SBOM/provenance/signing.
24. Vulnerability and secret scanning gates.
25. No privileged mode for untrusted code.

## 12. CI/CD best practices

1. ALL
2. seccomp=unconfined
3. /var/run/docker.sock:/var/run/docker.sock
4. /:/host
5. "5432:5432"

## 2.1 Base image mistakes

1. Using random images from unknown publishers.
2. Using abandoned base images.
3. Using unnecessarily large bases such as full Ubuntu/Debian when a slim or distroless runtime would work.
4. Using mutable tags like `latest`, `stable`, `dev`, `main`, or unqualified tags.
5. Using digest pinning with no update automation, causing images to silently age.
6. Using images from public registries without verifying publisher, signature, digest, or provenance.
7. Pulling from untrusted registries over insecure transport.
8. Mixing package ecosystems from multiple distributions.
9. Using EOL distributions.
10. Using language runtime images that include compilers, shells, package managers, test tooling, and build credentials in the final production image.

## 2.11 Bad multi-stage build behavior

1. Compiler remains in final image.
2. Source code remains in final image.
3. Build cache and module cache may remain.
4. Secrets used during build may remain.
5. Final image is larger than necessary.
6. Attack surface is larger.
7. Startup can be slower if build happens at container start.
8. Scanning finds more irrelevant CVEs.
9. Final image may include package managers and shells.
10. Runtime and build concerns are mixed.

## 2.12 Bad ENTRYPOINT/CMD behavior

1. Shell-form commands can mishandle signals.
2. PID 1 may not reap zombie processes.
3. `tail -f /dev/null` hides process failure.
4. Startup scripts can swallow exit codes.
5. Multiple daemons in one container complicate shutdown and health.
6. App cannot receive SIGTERM cleanly.
7. `docker stop` can hang or kill ungracefully.
8. Logs may go to files instead of stdout/stderr.
9. Shell expansion can create injection risks.
10. Debug wrappers become production behavior.

## 2.13 Healthcheck mistakes

1. No healthcheck for long-running services.
2. Healthcheck only checks process existence.
3. Healthcheck requires internet access.
4. Healthcheck mutates state.
5. Healthcheck authenticates with a real privileged credential.
6. Healthcheck logs secrets.
7. Healthcheck interval is too aggressive.
8. Healthcheck timeout is too short for cold starts.
9. Healthcheck uses tools missing from minimal image.
10. Compose `depends_on` used as a replacement for readiness.

## 2.4 Bad package-manager behavior

1. `apt-get update` and `apt-get install` in separate layers can use stale package indexes because of Docker cache behavior.
2. Installing without `--no-install-recommends` pulls unnecessary packages.
3. Leaving `/var/lib/apt/lists/*` bloats the image.
4. Installing broad toolsets increases attack surface.
5. Running `apt-get upgrade` or `dist-upgrade` inside app images can make builds less predictable.
6. Not pinning package versions can make rebuilds non-reproducible.
7. Installing compilers and build tools into the final runtime image increases exploitability.
8. Installing from random curl scripts bypasses package verification.
9. Installing from default package repos without checking EOL or security support.
10. Installing debug tools “temporarily” and never removing them.

## 2.5 `curl | bash` and unverified downloads

1. No checksum verification.
2. No signature verification.
3. No pinned version.
4. No auditable artifact source.
5. Installer script can change without notice.
6. Build becomes non-reproducible.
7. Build can execute compromised installer content.
8. Downloaded artifact can be replaced server-side.
9. Redirects can fetch unexpected content.
10. Scripts often install extra dependencies.

## 2.6 Misusing `ADD`

1. `ADD` has extra behavior that can surprise reviewers.
2. Remote URL downloads are harder to verify.
3. Local tar auto-extraction can hide unexpected file structure.
4. Cache behavior can be confusing.
5. Security review is harder than with explicit `COPY`, `curl`, checksum, and `tar`.

## 2.7 Bad file permissions

1. World-writable directories invite tampering.
2. Recursive permission changes create huge layers.
3. Setuid/setgid binaries increase privilege-escalation risk.
4. Root-owned app files can force runtime root.
5. Writable code directories let attackers modify application code after compromise.
6. Writable config directories can let attackers alter runtime behavior.
7. Overly broad permissions hide the real ownership problem.
8. Recursive `chown` after `COPY` is slow on large contexts.
9. Permission mistakes often trigger agents to add `privileged: true`.
10. Permissions may behave differently with bind mounts across host OSes.

## 2.8 No `.dockerignore`

1. Larger build context.
2. Slower local and CI builds.
3. Cache invalidations from irrelevant files.
4. Secret leakage.
5. Source-control history leakage.
6. Test artifacts in production image.
7. Host dependencies copied into image.
8. Non-reproducible builds.
9. Unnecessary Docker daemon upload time.
10. More vulnerability scanner noise.

## 3.3 Unstable build inputs

1. Unpinned branch.
2. Unpinned package version.
3. Unpinned artifact.
4. No lockfile.
5. Build output changes over time.
6. Cache becomes misleading.
7. Rollbacks become difficult.
8. Vulnerability triage becomes impossible.
9. SBOMs are less useful.
10. “Works on my machine” becomes likely.
11. Use lockfiles: `package-lock.json`, `pnpm-lock.yaml`, `poetry.lock`, `Pipfile.lock`, `go.sum`, `Cargo.lock`.
12. Use deterministic installers: `npm ci`, `pnpm install --frozen-lockfile`, `pip install --require-hashes`, `poetry install --sync`, `go mod download`.
13. Pin Git SHAs, not branches.
14. Verify checksums for downloaded artifacts.
15. Emit SBOM/provenance.

## 3.5 Monolithic images and missing stage targets

1. Slow builds.
2. Large images.
3. More CVEs.
4. Runtime contains build tools.
5. Dev tools ship to prod.
6. Slow push/pull.
7. Harder scanning.
8. Harder SBOM review.
9. Larger attack surface.
10. Poor separation of concerns.

## 3.6 Expensive recursive operations

1. Slow on large repos.
2. Creates large layers.
3. Invalidates cache frequently.
4. Hides ownership problems.
5. Can make builds painfully slow on macOS/Windows bind mounts.

## 4.10 No resource limits

1. One container can starve the host.
2. Fork bombs can exhaust process table.
3. Memory leaks can kill unrelated workloads.
4. CPU spikes can collapse local dev or small hosts.
5. OOM behavior becomes unpredictable.
6. Noisy-neighbor problems.
7. CI jobs become flaky.
8. DoS blast radius increases.
9. Monitoring lacks expected bounds.
10. Autoscaling signals become noisy.

## 4.12 Bad restart policies

1. Crash loops are hidden.
2. Failed migrations repeat.
3. Corrupt state can be repeatedly mutated.
4. Logs fill disk.
5. Monitoring sees noisy restarts.
6. Security failures can keep retrying.
7. Local dev becomes confusing.
8. Masked dependency failures.
9. Can hammer external APIs.
10. Makes root-cause analysis harder.

## 4.13 Unbounded logs

1. Disk exhaustion.
2. CI runner failure.
3. Node outage.
4. Sensitive data retained longer.
5. Hard incident response.
6. Noisy services hide useful logs.
7. Log files grow without control.
8. Performance degradation.
9. Backups may include logs.
10. Developers “fix” by deleting Docker data.

## 4.2 Dangerous capabilities

1. ALL
2. SYS_ADMIN
3. NET_ADMIN
4. SYS_PTRACE
5. Capabilities can combine with kernel bugs.
6. Capabilities make host mounts more dangerous.
7. Many apps do not need any added capability.
8. Capabilities are often copied from Stack Overflow without justification.
9. Broad capabilities make runtime behavior harder to audit.
10. They are frequently used instead of fixing the root cause.
11. NET_BIND_SERVICE # only if genuinely needed

## 4.6 Startup ordering, health, and reliability mistakes

1. Using short-form `depends_on` and assuming the dependency is ready.
2. No healthcheck on databases, brokers, or APIs.
3. Healthchecks that only check “process exists” instead of “service is usable.”
4. Healthchecks that require network tools not installed in the image.
5. Healthchecks that mutate state.
6. Healthchecks that are too frequent and create load.
7. Healthchecks that leak credentials in command output.
8. Disabling healthchecks inherited from images.
9. `depends_on` cycles.
10. Migration jobs racing from multiple replicas.
11. Long-running services depend on one-shot migration jobs but do not check success.
12. `restart: always` on one-shot jobs.
13. No `restart` policy for production services that should recover.
14. `restart: always` on services that should fail visibly.
15. No graceful shutdown settings.
16. No `stop_grace_period` for apps that need time to drain.
17. No `init: true` for processes that spawn children and do not reap them.
18. Entrypoint scripts swallowing signals.
19. Containers kept alive with `sleep infinity` or `tail -f /dev/null`.
20. Services write critical data to ephemeral container filesystem.

## 4.7 Resource, logging, and denial-of-service mistakes

1. No memory limits.
2. No CPU limits.
3. No PID limits.
4. No ulimits.
5. No log rotation.
6. `oom_kill_disable: true` without strict memory limits.
7. Unlimited file descriptors.
8. Unlimited process creation.
9. Unlimited container logs filling disks.
10. Debug-level logs in production.
11. Verbose database logs on shared disks.
12. Heavy healthchecks that become self-inflicted DoS.
13. Resource-heavy builds on shared Docker hosts.
14. Not pruning builder cache on shared CI machines.
15. Pruning too aggressively and destroying useful cache.
16. Large image pulls on every deploy.
17. No registry mirror or dependency cache where appropriate.
18. No monitoring of container disk usage.
19. No monitoring of Docker daemon disk usage.
20. No alerts for restart loops.

## 4.8 Host PID, IPC, cgroup, and devices

1. /dev:/dev
2. Process visibility into host.
3. Potential signal/process interference.
4. Shared memory attacks.
5. Device access escalation.
6. Host observability leakage.
7. Kernel attack surface expansion.
8. Harder policy enforcement.
9. Non-portable behavior.
10. Dangerous with root.
11. Often unnecessary.

## 4.9 Data persistence mistakes

1. Database data stored in container writable layer.
2. No named volume for stateful services.
3. Named volumes with no backup.
4. Backups exist but restores are never tested.
5. `docker compose down -v` in README cleanup command without warning.
6. `docker volume prune` in automated cleanup.
7. Mixing dev and prod project names so volumes collide.
8. Using the same volume for incompatible app versions.
9. Mounting a single writable volume into multiple containers that are not designed for it.
10. Mounting host directories with wrong ownership, then running container as root.
11. Mounting NFS/SMB volumes without checking database compatibility.
12. No data migration plan.
13. No initialization idempotency.
14. `depends_on` used instead of migration orchestration.
15. Secret files stored in persistent app volumes.
16. Logs stored in app data volumes indefinitely.
17. Unbounded caches stored in named volumes.
18. Anonymous volumes created accidentally and forgotten.

## 5.1 Privileged Docker-in-Docker everywhere

1. docker:dind
2. Privileged CI containers weaken host isolation.
3. Untrusted PRs can attack the runner.
4. Registry credentials can be stolen.
5. Build secrets can be exfiltrated.
6. Cache poisoning becomes easier.
7. Host breakout blast radius increases.
8. Secrets from other jobs may be reachable on persistent runners.
9. Cloud metadata services may be reachable.
10. Agent “fixes” build errors by further widening permissions.
11. CI becomes a high-value attack surface.
12. Use rootless builders where practical.
13. Use restricted privileged mode only for explicitly allowed rootless DinD images.
14. Use isolated ephemeral runners for privileged builds.
15. Never run untrusted PRs with privileged mode and secrets.
16. Pin Docker CLI/DinD images instead of using `docker:latest`.

## 5.2 Performance mistakes in CI

1. Cold builds every time.
2. No Docker layer cache.
3. No dependency cache.
4. No registry-backed BuildKit cache.
5. No `.dockerignore`.
6. Huge monorepo build context.
7. Tests run after full production image build when an earlier test stage would fail faster.
8. Image scan downloads huge images repeatedly.
9. Multiple jobs build the same image independently.
10. No build matrix discipline.
11. Multi-platform builds on every branch.
12. QEMU emulation for normal PR checks.
13. Always using `--no-cache`.
14. Never using `--pull`, causing stale bases.
15. Always using `--pull` in inner-loop dev builds, causing unnecessary remote checks.
16. No remote registry mirror.
17. No package-manager cache mounts.
18. No parallelized independent stages.
19. Legacy builder instead of BuildKit.
20. Large base images in every job.

## 5.4 Ignoring CI provider Docker restrictions

1. Assuming Docker features work the same across local, GitHub Actions, GitLab, Bitbucket, and self-hosted runners.
2. Adding privileged mode until builds pass.
3. Disabling BuildKit without understanding why.
4. Copying old Compose binaries into CI.
5. Using features blocked by the CI provider for security.
6. Building with large contexts in ephemeral environments.
7. Not configuring registry cache.
8. Running Compose integration tests with public ports.
9. Leaking service container credentials into logs.
10. Using shared runners for sensitive builds.

## 6.1 No scanning

1. No image vulnerability scanning.
2. No dependency scanning.
3. No Dockerfile linting.
4. No secret scanning.
5. No SBOM.
6. No provenance.
7. No signature verification.
8. No policy gates.
9. No base-image refresh process.
10. No CI failure threshold for critical vulnerabilities.

## 6.2 No SBOM or provenance

1. Hard to know what is inside the image.
2. Hard to prove how it was built.
3. Hard to respond to CVEs.
4. Hard to verify source commit.
5. Hard to meet compliance requirements.
6. Hard to detect tampering.
7. Hard to audit dependency drift.
8. Hard to reproduce builds.
9. Hard to enforce deployment policy.
10. Consumers must trust tags.

## 6.3 No signing or verification

1. Pushing images without signatures.
2. Deploying images by mutable tags.
3. Not verifying signatures at deployment time.
4. Using one shared registry credential everywhere.
5. No separation between build and deploy credentials.
6. No protected branch/tag policy.
7. No trusted builder identity.
8. No admission/policy checks.
9. No audit of who pushed the image.
10. No rollback digest record.

## 7.10 Scan, lint, and enforce policy

1. **Dockerfile lint:** Hadolint parses Dockerfiles and integrates ShellCheck for shell in `RUN` instructions.
2. **Image vulnerability scanning:** Trivy, Grype, Docker Scout, Clair, and similar tools.
3. **Secret scanning:** Gitleaks and TruffleHog.
4. **Host benchmark:** Docker Bench for Security / CIS Docker Benchmark.
5. **Policy-as-code:** OPA/Conftest for blocking forbidden Compose fields.
6. **Supply chain:** SBOM generation, provenance, signing, trusted registries.

## 7.2 Agent makes performance worse while “optimizing”

1. Adds `--no-cache` to build instructions as a permanent fix.
2. Moves `COPY . .` before dependency installation.
3. Deletes lockfiles.
4. Uses `npm install` instead of deterministic install modes where a lockfile exists.
5. Installs dev dependencies into final production image.
6. Rebuilds base images from source unnecessarily.
7. Adds language caches into the final image.
8. Uses Docker-in-Docker in CI without layer caching.
9. Uses multiple independent service builds where one shared base stage would work.
10. Downloads large artifacts repeatedly instead of using cache mounts.
11. Adds test data and fixtures to production images.
12. Adds `pull_policy: always` everywhere.
13. Forces QEMU cross-platform builds without a reason.
14. Adds recursive `chown` of huge trees.
15. Leaves apt/package indexes and temporary archives in final layers.
16. Uses Compose bind mounts for dependency directories in ways that break cache or slow file I/O.

## 7.2 Build fast and reproducibly

1. Uses BuildKit syntax.
2. Pins base image by digest for reproducibility.
3. Uses a minimal base.
4. Creates a non-root user.
5. Copies dependency metadata before source code.
6. Uses multi-stage builds.
7. Keeps build tools out of the final image.
8. Uses cache mounts.
9. Cleans apt metadata in the same layer.
10. Uses exec-form entrypoint.
11. Avoids secrets in `ARG` or `ENV`.
12. Avoids root in runtime.

## 7.2 Treating the `docker` group as harmless

1. Docker group is effectively root-equivalent on many hosts.
2. Users can mount host paths.
3. Users can run privileged containers.
4. Users can access other containers.
5. Users can exfiltrate secrets.
6. Local compromise becomes host compromise.
7. Audit boundaries blur.
8. CI jobs inherit too much power.
9. Desktop convenience becomes server risk.
10. Group membership often persists forgotten.

## 7.3 Agent hides operational risk

1. Does not call out that `deploy:` settings may be ignored depending on the platform. Compose docs note the deploy section is optional and can be ignored if unsupported.
2. Uses Compose for production without explaining limitations, monitoring, backups, or update strategy.
3. Adds named volumes without backup expectations.
4. Adds new open ports without documenting exposure.
5. Adds new networks without documenting trust boundaries.
6. Adds `external: true` networks/volumes without lifecycle ownership.
7. Adds `depends_on` without healthchecks and readiness semantics.
8. Adds migration jobs into the main web service startup command.
9. Does not include resource, logging, or restart policies.
10. Does not provide a way to inspect effective config.
11. Does not produce a threat model for sensitive mounts or Docker socket usage.

## 7.3 Not keeping Docker updated

1. Old Docker Engine.
2. Old Docker Desktop.
3. Old Compose plugin.
4. Old BuildKit/buildx.
5. Old runc/containerd.
6. No patching schedule.
7. No CVE monitoring.
8. No daemon restart plan.
9. No base-image rebuild plan.
10. Assuming container isolation is enough.

## 7.8 Handle startup correctly

1. db
2. Use healthchecks for real readiness.
3. Use long-form `depends_on` with `condition: service_healthy` where needed.
4. Do not use sleeps as readiness logic.
5. Make migrations idempotent.
6. Ensure one-shot jobs have appropriate restart behavior.
7. Make containers fail clearly instead of hanging forever.

## 7.9 Use image tags and digests responsibly

1. Use immutable tags such as commit SHA.
2. Use release tags for humans.
3. Avoid deploying `latest`.
4. Pin digests for reproducibility-critical environments.
5. Use an update bot or scheduled rebuilds so digest pinning does not freeze vulnerabilities forever.
6. Rebuild regularly with updated bases.
7. Scan continuously after deployment, not only at build time.

## 8.1 A strong Dockerfile pattern

1. A trusted, minimal, pinned base image.
2. A `.dockerignore`.
3. Dependency manifests copied before application source.
4. Lockfile-based deterministic dependency install.
5. Multi-stage separation: dependency, test, build, runtime.
6. BuildKit cache mounts for package managers.
7. BuildKit secrets/SSH mounts for private dependencies.
8. A minimal final runtime image.
9. No package manager, compiler, VCS, or shell in final image unless required.
10. A non-root runtime user.
11. Exec-form `CMD`/`ENTRYPOINT`.
12. No secrets in layers, args, env, labels, or logs.
13. Metadata labels for source/revision/license when non-sensitive.
14. Rebuild/update/scanning process.

## 8.1 Never generate these without an explicit exception

1. ALL
2. seccomp=unconfined
3. apparmor=unconfined
4. /var/run/docker.sock:/var/run/docker.sock
5. /:/host
6. /dev:/dev

## 8.1 Permission error fixes

1. .:/app
2. Turns a simple ownership bug into a security problem.
3. Hides incorrect image design.
4. Makes production less secure.
5. Encourages future unsafe patches.
6. Can expose host files through bind mounts.

## 8.2 Build-speed fixes that make builds slower

1. Disables cache.
2. Slows every build.
3. Hides bad layer ordering.
4. Increases network dependency.
5. Makes CI expensive.
6. Add `.dockerignore`.
7. Copy lockfiles before source.
8. Use BuildKit cache mounts.
9. Use `cache_from`/`cache_to`.
10. Use smaller base images.

## 8.2 Require a written justification for these

1. Any `cap_add`.
2. Any host bind mount.
3. Any published port.
4. Any `devices` mapping.
5. Any `sysctls`.
6. Any `security_opt`.
7. Any root user.
8. Any Docker socket access.
9. Any secret file source.
10. Any public image from an unverified publisher.
11. Any digest-pinned image without an update process.
12. Any disabled healthcheck.
13. Any missing resource limits in production.
14. Any CI job requiring privileged mode.

## 8.3 Always do these when generating Dockerfiles

1. Add `.dockerignore`.
2. Use multi-stage builds.
3. Put dependency lockfile copy before app source copy.
4. Use BuildKit syntax.
5. Use cache mounts for package managers.
6. Avoid secrets in `ARG`/`ENV`.
7. Use non-root runtime user.
8. Avoid compilers in final image.
9. Clean package manager indexes in the same layer.
10. Use exec-form `ENTRYPOINT`/`CMD`.
11. Keep final image minimal.
12. Add a healthcheck when appropriate.
13. Pin base image versions; use digests for production reproducibility.
14. Add labels for source/revision where useful.
15. Sort multiline package installs.
16. Rebuild with `--pull` on a schedule or release pipeline.

## 8.5 Debug fixes left in production

1. "9229:9229"
2. "5678:5678"
3. Debug ports exposed.
4. App does not start normally.
5. Healthchecks meaningless.
6. Containers never exit.
7. Attack surface increases.
8. Secrets may be introspected.
9. Restart policies mask failure.
10. Production differs from CI.
11. Debug dependencies remain installed.
12. Logging and monitoring break.

## 8.8 CI/CD best practices

1. **Dockerfile linting**
2. **Compose validation**
3. **Build with cache**
4. **Generate SBOM**
5. **Scan vulnerabilities and misconfigurations**
6. **Sign images and attest provenance**
7. **Policy gate before deploy**
8. **Push immutable image references**
9. **Keep base images updated**
10. **Run Docker Bench/CIS checks on hosts**

## 9.1 Hardened Dockerfile pattern

1. Specific base version instead of `latest`.
2. Small runtime image.
3. Lockfile-based install.
4. Dependency cache mount.
5. No secrets in Dockerfile.
6. Non-root user.
7. Ownership fixed at copy time.
8. No package-manager cache copied from build stage.
9. Exec-form entrypoint.
10. Healthcheck.

## 9.2 Warn rules

1. "5432:5432"
2. "6379:6379"
3. "9200:9200"
4. "27017:27017"

## A. Dangerous privilege and kernel-surface settings

1. **`privileged: true`.**
2. **`cap_add: ["ALL"]`.**
3. **Adding powerful capabilities casually:** `SYS_ADMIN`, `NET_ADMIN`, `SYS_PTRACE`, `DAC_READ_SEARCH`, `SYS_MODULE`, `SYS_RAWIO`, `MKNOD`, `AUDIT_CONTROL`.
4. **No `cap_drop`.**
5. **No `security_opt: ["no-new-privileges:true"]`.**
6. **`security_opt: ["seccomp=unconfined"]`.**
7. **`security_opt: ["apparmor=unconfined"]`.**
8. **`security_opt: ["label:disable"]` on SELinux systems.**
9. **`pid: host`.**
10. **`ipc: host`.**
11. **`cgroup: host`.**
12. **`network_mode: host`.**
13. **`userns_mode: host`.**
14. **`uts: host`.**
15. **Mapping host devices unnecessarily.**
16. **Mounting `/dev`, `/dev/sda`, `/dev/mem`, `/dev/kmsg`, `/sys`, `/proc`, `/lib/modules`, or hardware devices without a strong reason.**
17. **Using `device_cgroup_rules` broadly.**
18. **Using Compose lifecycle hooks such as `post_start` with `user: root` or `privileged: true`.**
19. **`use_api_socket: true` without extreme care.**

## A. Docker daemon access and host isolation

1. **Adding broad users to the `docker` group.**
2. **Exposing Docker daemon TCP port without TLS.**
3. **Running rootful Docker where rootless mode is feasible.**
4. **Running untrusted workloads on the same Docker host as sensitive workloads.**
5. **Mixing containerized and non-containerized workloads on the same host.**
6. **Letting containers mount arbitrary host paths.**
7. **Letting CI jobs start sibling containers through the host socket.**
8. **Using Docker-in-Docker with `privileged` by default.**
9. **Using shared CI runners for privileged container builds from untrusted branches.**
10. **Allowing pull requests from forks to run privileged Docker builds.**
11. **Allowing unreviewed Dockerfiles to run on privileged builders.**
12. **Running old Docker Engine or stale host kernels.**
13. **No host-level Docker Bench / CIS-style hardening.**
14. **No daemon audit/logging.**
15. **Running daemon debug logging permanently.**
16. **No image/container pruning policy, causing disk exhaustion.**
17. **No registry mirror/cache strategy, causing slow builds and rate-limit failures.**
18. **Using host networking for convenience.**
19. **Running containers with host PID/IPC/cgroup namespace access unnecessarily.**
20. **No runtime monitoring or anomaly detection.**

## A. Secure, fast Dockerfile principles

1. **Use multi-stage builds.**
2. **Use a minimal trusted runtime base.**
3. **Pin versions deliberately.**
4. **Rebuild often.**
5. **Copy dependency manifests before source.**
6. **Use `.dockerignore`.**
7. **Use BuildKit cache mounts.**
8. **Use external cache in CI.**
9. **Use BuildKit secrets for build-time secrets.**
10. **Combine package index update and install.**
11. **Use `COPY` rather than `ADD` unless you need `ADD` features.**
12. **Use exec-form `ENTRYPOINT`/`CMD`.**
13. **Run as non-root.**
14. **Make filesystem writes explicit.**
15. **Inspect the final image.**

## A. Secure, fast Dockerfile standard

1. Dependency manifests are copied before source, so dependency layers remain cached when app code changes.
2. The image uses a multi-stage pattern to keep runtime smaller.
3. Packages are installed with `apt-get update && apt-get install` in the same layer.
4. The apt lists are removed to shrink the image.
5. Runtime uses a non-root user.
6. `CMD` is exec-form.
7. Health check is explicit.
8. No secrets are passed through `ARG` or `ENV`.

## AI & Vibe-Coding

1. Slow startup.
2. Network dependency at runtime.
3. Non-reproducible runtime.
4. Secrets needed at runtime just to install.
5. Restarts can mutate the container state.
6. Production containers differ from tested images.
7. Scaling creates package-registry load.
8. Harder incident response.
9. Healthchecks can fail during install.
10. Rollback does not guarantee previous dependency graph.
11. Broadly disables isolation.
12. Grants access to many devices/capabilities.
13. Can enable container escape paths.
14. Commonly used as a lazy fix for permission, networking, or Docker-in-Docker problems.
15. Makes local dev behavior much less like hardened production.
16. Breaks least privilege.
17. Can expose the host if combined with bind mounts.
18. Makes scanner/policy results fail.
19. Encourages future agents to build on a dangerous baseline.
20. Often masks the actual required capability.
21. no-new-privileges:true
22. NET_BIND_SERVICE
23. **No vulnerability scanning before push/deploy**. [S]
24. **Ignoring critical/high CVEs because “it still builds.”** [S]
25. **Never rebuilding images after base-image security updates**. [S/R]
26. **Using stale images for months**. [S] Reddit community anecdotes also show this as a common operator failure, though social sources should be treated as anecdotal.
27. **No SBOM**. [S/M/Ops]
28. **No provenance/attestation**. [S/R]
29. **No image signing or signature verification**. [S]
30. **Using mutable tags as deployment identifiers**. [R/S]
31. **Reusing the same tag for different builds without traceability**. [R/S]
32. **Pushing `latest` as the production deployment contract**. [R/S]
33. **Using private images on shared runners with cache policies that let later jobs reuse them**. [S] GitLab warns that `if-not-present` on shared/public runners can leak private images from runner cache.
34. **Allowing anyone to push to a production registry namespace**. [S/Ops]
35. **No registry retention policy**. [S/Ops]
36. **No image owner/team metadata**. [M/Ops]
37. **No deprecation/EOL policy for old images**. [S/Ops]
38. **No malware or secret scanning of image layers**. [S]
39. **Trusting Docker Hub popularity alone**. [S]
40. **Using images from personal namespaces for production**. [S/Ops]
41. **Copying binaries from arbitrary public images via `COPY --from=some/random:image`**. [S]
42. **Using remote Compose OCI artifacts without pinning digest**. [S/R] Docker warns tags are mutable and recommends pinning remote references to digests.
43. **Not reviewing transitive Compose `include` and `extends` dependencies**. [S/M]
44. **No admission/policy gate preventing untrusted images**. [S]
45. **No dependency review for base image changes**. [S/M]
46. **No lockfile for image references in production**. [R/S]
47. **Assuming Docker guarantees reproducibility by itself**. [R] A 2026 academic study specifically investigates Docker reproducibility limits across GitHub workflows.
48. **Using `privileged: true` as a lazy fix**. [S]
49. **Mounting docker.sock so the agent can “just build/run things.”** [S]
50. **Giving the agent host Docker daemon access instead of an isolated daemon**. [S]
51. **Running agents directly on the host with broad filesystem access**. [S]
52. **Letting agents edit `.github/workflows`, `.gitlab-ci.yml`, Git hooks, Makefiles, package scripts, shell scripts, or IDE tasks without review**. [S]
53. **Letting agents modify `.env`, credentials, or secret templates**. [S]
54. **Letting agents introduce `curl | sh` installers**. [S]
55. **Letting agents disable TLS/cert verification to “fix” network errors**. [S]
56. **Letting agents add `--no-sandbox` flags to browsers or runtimes without understanding the security implication**. [S]
57. **Letting agents add `chmod -R 777` or root runtime to fix permissions**. [S]
58. **Letting agents remove lockfiles**. [R/S]
59. **Letting agents switch from deterministic package install commands to floating installs**. [R/S]
60. **Letting agents copy the whole repository into images because it is easy**. [P/S]
61. **Letting agents remove `.dockerignore` entries to make a missing file appear**. [P/S]
62. **Letting agents add debug/admin ports to Compose and forget to remove them**. [S]
63. **Letting agents expose services on `0.0.0.0`**. [S]
64. **Letting agents silence scanners or linters globally**. [S/M]
65. **Letting agents add blanket ignores such as `# hadolint ignore=...` without explanation**. [S/M]
66. **Letting agents “fix” image vulnerabilities by pinning older packages**. [S/R]
67. **Letting agents “fix” build failures by using abandoned base images**. [S/R]
68. **Letting agents add random public images as helper services**. [S]
69. **Letting agents store generated secrets in Compose examples**. [S]
70. **Letting agents run huge build/test logs directly into their context window**. [P/R] Docker Agent docs recommend redirecting large output to files so agents can inspect relevant parts.
71. **Letting one agent own planning, Dockerfile writing, security review, and deployment approval with no separation of duties**. [S/M]
72. **Letting agents run destructive Docker cleanup commands**. [R/Ops]
73. **Letting agents treat Docker containers as a perfect security boundary for hostile code**. [S]
74. **No diff review after agent sessions**. [S/M]
75. **No policy prompt/tool guardrails rejecting dangerous Compose keys**. [S]
76. **No tests proving the hardened image still works as non-root/read-only**. [R]
77. **No exception process for risky settings introduced by agents**. [S/M]
78. **Add policy checks that fail on dangerous keys.** Block or require exception approval for `privileged`, docker socket mounts, `network_mode: host`, `pid: host`, `ipc: host`, `cap_add: ALL`, `seccomp=unconfined`, `user: root`, public admin ports, and secrets in env vars.
79. **Run Dockerfile linting.** Hadolint parses Dockerfiles and applies Hadolint/ShellCheck rules; its rules include avoiding root final users, avoiding latest tags, pinning package versions, avoiding sudo, deleting apt lists, and preferring `COPY` over `ADD`.
80. **Run Compose/IaC scanning.** KICS has Docker Compose queries for docker socket mounts, privileged containers, missing `no-new-privileges`, missing health checks, unrestricted capabilities, host namespaces, missing memory/CPU/PID limits, and ports not bound to a host interface.
81. **Scan final images.** OWASP recommends CI/CD checks including linting, static analysis, container scanning, secret detection, and Docker misconfiguration checks.
82. **Generate SBOM and provenance.** Treat image metadata as release artifacts.
83. **Sign images and deploy by digest.** Build once, scan once, sign once, then promote the immutable digest.
84. **Use ephemeral, isolated runners for untrusted code.** Avoid privileged shared runners.
85. **Do not expose production credentials to build jobs.** Separate build secrets from deploy secrets.
86. **Review agent changes like untrusted pull requests.** Docker’s workspace trust docs explicitly warn that agents can modify hidden files, Git hooks, CI configs, IDE configs, and build scripts that later execute code.
87. **Run agents in isolated environments.** Docker Sandboxes use microVM isolation, an isolated Docker Engine, network policy, and credential proxying so agents do not touch the host Docker daemon by default.
88. No `privileged: true`.
89. No Docker/container runtime socket mounts.
90. No host root or sensitive host path mounts.
91. No host namespaces unless explicitly approved.
92. No secrets in Dockerfile, Compose YAML, `.env`, build args, environment variables, logs, or image layers.
93. Images are pinned by version and production images by digest.
94. Dockerfile has `.dockerignore`, cache-aware ordering, and multi-stage build where appropriate.
95. Final runtime image does not contain compilers, build tools, package caches, test artifacts, or credentials.
96. Final container runs as non-root.
97. Compose services drop capabilities, use `no-new-privileges`, and use read-only root filesystems where feasible.
98. Published ports are explicitly bound to `127.0.0.1` unless public exposure is intentional.
99. Internal services use internal networks and are not published to the host.
100. Services have health checks; `depends_on` uses `condition: service_healthy` where readiness matters.
101. Memory, CPU, PID, ulimit, restart, stop-grace, and logging controls are set or consciously waived.
102. `docker compose config` output is reviewed in CI.
103. Dockerfile linter, Compose/IaC scanner, image vulnerability scanner, and secret scanner run in CI.
104. Risky exceptions include owner, reason, scope, expiration date, and compensating controls.
105. Agent-modified CI files, Git hooks, Makefiles, package scripts, Compose files, and Dockerfiles get human review before execution or merge.
106. **Runs a Compose file from the internet without inspection.**
107. **Follows remote `include` or `extends` chains without reviewing transitive files.**
108. **Does not run `docker compose config` before applying a generated Compose file.**
109. **Adds an override file that secretly weakens production settings.**
110. **Copies snippets from GitHub gists, Reddit, Stack Overflow, old blog posts, or LLM outputs without checking current Compose spec behavior.**
111. **Treats generated infrastructure files as “just config,” not code requiring security review.**
112. **Automatically accepts prompts from Docker Desktop / Compose about running untrusted resources.**
113. **Runs containers against the host’s real Docker daemon during analysis without understanding side effects.**
114. **Runs destructive cleanup commands such as `docker system prune -a`, `docker volume prune`, or `docker compose down -v` on a shared or production host.**
115. **Touches production Docker hosts from a development agent session.**
116. **Generates “works on my machine” Docker settings and ships them to production.**
117. **Conflates build-time, test-time, and production-time needs.**
118. `FROM ubuntu:latest`, `node:latest`, `python:latest`, or no tag.
119. Using mutable tags in production without digest pinning.
120. Pinning a digest but never updating it.
121. Pulling base images from unknown publishers.
122. Using abandoned images.
123. Using images with huge dependency surfaces when a smaller base is sufficient.
124. Copying binaries from untrusted external images.
125. Copying from an external build stage by tag rather than digest.
126. Installing packages from unsigned repositories.
127. Disabling package signature checks.
128. Using HTTP package mirrors where HTTPS/signatures are available.
129. `curl | sh` installation scripts.
130. `wget` or `curl` downloads without checksum/signature verification.
131. `git clone` from a moving branch instead of a pinned commit/tag.
132. Adding package repositories without pinning or validating keys.
133. Not rebuilding images regularly to pick up patched base layers. Docker recommends rebuilding often and using `--pull` to get updated base images.
134. No image scanning.
135. No SBOM.
136. No image signing.
137. No provenance/attestation.
138. No policy that defines allowed registries.
139. No trusted registry or private registry policy for internal workloads.
140. No exception process for vulnerabilities.
141. Scans only once at build time and never rescans deployed images.
142. No OCI labels for source, revision, license, vendor, description.
143. No versioned image tags tied to commit SHA or release.
144. Image cannot be traced back to source.
145. No build date or revision metadata when useful.
146. No ownership information.
147. No `STOPSIGNAL` when the app needs a non-default shutdown signal.
148. No documented ports.
149. Wrong `EXPOSE` values.
150. Treating `EXPOSE` as a security control. It is metadata; port publishing is controlled at runtime/Compose.
151. Ambiguous `WORKDIR`.
152. Writes files to `/` by accident.
153. Ignores `.dockerignore` drift.
154. Dockerfile comments explain nothing about security exceptions.
155. Has duplicated package lists or stale build steps.
156. Uses deprecated instructions/patterns without justification.
157. Hardcodes production URLs, credentials, tenants, or region-specific endpoints.
158. Enables debug mode by default.
159. Ships test certificates or self-signed dev CA into production image.
160. `cap_add: ["ALL"]`.
161. `cap_add: ["SYS_ADMIN"]` without a narrow reason.
162. `cap_add: ["NET_ADMIN"]` for ordinary web apps.
163. `cap_add: ["NET_RAW"]` when not needed.
164. No `cap_drop`.
165. `security_opt: ["seccomp=unconfined"]`.
166. `security_opt: ["apparmor=unconfined"]`.
167. Disabling SELinux labels.
168. Running as root by default.
169. `user: "0:0"`.
170. Using `userns_mode: host`.
171. `pid: host`.
172. `ipc: host`.
173. Sharing IPC namespaces between unrelated containers.
174. `network_mode: host`.
175. Broad `sysctls` that change network/kernel behavior.
176. Granting devices broadly: `/dev:/dev`, `/dev/sda`, `/dev/kvm`, `/dev/fuse`, GPU devices without need.
177. Using lifecycle hooks with `privileged: true`; Compose supports privileged lifecycle hooks, which should be scrutinized.
178. Giving monitoring/sidecar containers more privilege than the app.
179. Adding Docker socket access to Watchtower-like or management containers without considering host takeover risk.
180. Running Docker-in-Docker in privileged mode in shared CI.
181. Assuming “inside a container” means “safe.”
182. **Using `FROM ubuntu:latest`, `node:latest`, `python:latest`, etc. in production.**
183. **Pinning a digest forever with no update process.**
184. **Using untrusted community images without provenance review.**
185. **Using abandoned or end-of-life base images.**
186. **Using a full OS base when a slim, Alpine, distroless, or runtime-only base would work.**
187. **Using “kitchen sink” internal base images that include shells, compilers, package managers, cloud CLIs, SSH clients, test tools, and debug tools in every runtime image.**
188. **Using images from personal namespaces rather than official/vendor-controlled registries for production.**
189. **Not scanning base images, OS packages, language packages, or final images.**
190. **No SBOM, provenance, or dependency inventory.**
191. **No policy gate for critical/high CVEs.**
192. **Accepting unsigned/unverified images where signature verification is expected.**
193. **Using `curl | sh` from arbitrary URLs without checksum, signature, or pinned version.**
194. **Downloading binaries from GitHub releases by branch name or “latest” URL with no checksum.**
195. **Installing packages from unpinned third-party apt/yum/apk repositories.**
196. **Adding apt keys or package repositories blindly from the internet.**
197. **Disabling TLS certificate verification for package managers, curl, git, npm, pip, Maven, etc.**
198. **Using old language runtimes because “the app still works.”**
199. **Leaving package-manager metadata and caches in final images.**
200. **Using internal base images that are not rebuilt after upstream CVE fixes.**
201. **Trusting a base image because it is “small” without checking whether it receives security updates.**

## Agent-process review

1. Makes a security downgrade without calling it out.
2. Broadens privileges to fix a bug.
3. Removes constraints to fix a crash.
4. Silences linters/scanners.
5. Introduces secrets into files.
6. Changes public exposure of ports.
7. Changes persistence paths.
8. Changes user/root behavior.
9. Changes image tags from pinned to mutable.
10. Changes runtime from read-only to writable.
11. Adds Docker socket access.
12. Adds host mounts.
13. Adds debug/admin services to default startup.
14. Does not validate the rendered Compose file.
15. Does not provide a diff rationale.
16. **Pinned, trusted base images**, ideally with digest pinning and an update workflow.
17. **Multi-stage builds** with small runtime images.
18. **Correct `.dockerignore`**.
19. **Dependency manifests copied before source code**.
20. **BuildKit cache mounts** for slow dependency installs.
21. **No secrets in image layers, `ARG`, `ENV`, Compose environment, or committed `.env` files**.
22. **BuildKit secret/SSH mounts** for build-time credentials.
23. **Non-root runtime user**.
24. **No `privileged: true` unless formally approved**.
25. **No Docker socket mount unless the container is intentionally a trusted Docker controller**.
26. **Drop Linux capabilities by default** and add back only what is needed.
27. **Use `no-new-privileges`**.
28. **Avoid host namespaces**.
29. **Use read-only root filesystem where practical**.
30. **Use `tmpfs` for temporary writable paths**.
31. **Use read-only bind mounts for config**.
32. **Use named volumes for persistent service data**.
33. **Bind ports explicitly**, especially `127.0.0.1` for local-only services.
34. **Use separate networks** for frontend, backend, data, admin, and observability paths.
35. **Use Compose secrets for sensitive runtime values**.
36. **Set memory, CPU, PID, ulimit, and log limits**.
37. **Add real health checks**.
38. **Use restart policies intentionally**.
39. **Scan images and dependencies continuously**.
40. **Lint Dockerfiles and Compose files**.
41. **Generate SBOM/provenance for release images**.
42. **Keep debug/admin tooling behind profiles**.
43. **Document every exception** to least privilege.
44. **Review final rendered config with `docker compose config`**.
45. **Review image history and metadata before publishing**.

## B. Bad build-context behavior

1. **No `.dockerignore`.**
2. **`.dockerignore` exists but misses secrets:** `.env`, `.env.production`, credentials, kubeconfigs, SSH keys, cloud config, npm tokens, PyPI tokens, Maven settings, Terraform state, database dumps.
3. **Sending `.git/` into the build context unintentionally.**
4. **Sending `node_modules/`, `.venv/`, `vendor/`, `target/`, `dist/`, `build/`, `.gradle/`, `.m2/`, coverage, logs, caches, and artifacts into the context.**
5. **Using `build.context: .` from a monorepo root when the service only needs one subdirectory.**
6. **Using `COPY . .` when only a few files are needed.**
7. **Copying production secrets and then deleting them later in the Dockerfile.**
8. **Copying test fixtures containing real customer data.**
9. **Copying local developer config into production images.**
10. **Copying `.dockerignore` patterns from another project without checking whether they hide required files or leak sensitive files.**

## Bad behaviors

1. No `.dockerignore`.
2. `.dockerignore` exists but fails to exclude `.git`, `node_modules`, `.venv`, `target`, `dist`, `build`, `coverage`, logs, temporary files, local databases, test fixtures, caches, package artifacts, generated files, and large datasets.
3. `.dockerignore` fails to exclude sensitive files: `.env`, `.env.*`, `.aws`, `.ssh`, `.kube`, `.npmrc`, `.pypirc`, `id_rsa`, credentials, private keys, kubeconfigs, Terraform state, cloud provider files, and local secret stores.
4. The agent sets the build context to the repository root when only a subdirectory is needed.
5. The agent uses `COPY . .` as the first meaningful instruction.
6. The agent copies source code before dependency manifests, destroying cache reuse.
7. The agent copies local build output into the image unintentionally.
8. The agent copies test data, screenshots, coverage reports, notebooks, fixtures, and sample data into production images.
9. The agent includes Git history in the image or build context.
10. The agent includes Dockerfiles, Compose files, CI files, and internal documentation unnecessarily in the runtime image.
11. The agent uses monorepo-wide context for a small service.
12. The agent relies on local machine artifacts instead of reproducible build steps.
13. The agent does not check `docker build` context size.
14. Putting the most frequently changing files at the top of the Dockerfile.
15. Running dependency installation after `COPY . .`.
16. Not separating dependency manifest copy from application source copy.
17. Not using package lockfiles.
18. Not using Docker BuildKit cache mounts for package managers.
19. Running `apt-get update` in one layer and `apt-get install` in another.
20. Running package-manager updates every time app code changes.
21. Using `--no-cache` as a default CI strategy.
22. Not using remote/external cache in CI.
23. Not using `buildx` cache export/import for repeated CI builds.
24. Re-downloading npm, pip, Maven, Gradle, Go, Rust, or OS packages on every build.
25. Combining completely unrelated steps into a huge opaque `RUN` block that is hard to cache, audit, and debug.
26. Splitting tightly related package-manager operations into too many layers.
27. Creating files in one layer and deleting them in a later layer, leaving data recoverable from previous layers.
28. Repeatedly `chown -R` large directories after copying.
29. Not using `COPY --chown` where appropriate.
30. Doing expensive compilation in the final runtime stage.
31. Rebuilding the same dependencies independently across multiple service images.
32. No shared base stage for related images.
33. No multi-stage targets for `dev`, `test`, `build`, and `runtime`.
34. Running tests in the final runtime image instead of a test stage.
35. Building frontend assets in every backend image build when they could be built once.
36. Using shell scripts that always invalidate cache by touching files or embedding timestamps.
37. Downloading remote files with changing URLs without checksums.
38. Not pinning dependency versions, causing cache unpredictability and non-reproducible builds.
39. `FROM ubuntu`, `FROM debian`, `FROM node`, `FROM python`, or similar without a tag.
40. `FROM something:latest`.
41. Using mutable tags for production.
42. Using old, unmaintained, or end-of-life base images.
43. Using unofficial images when official, verified, or trusted images exist.
44. Using random Docker Hub images without checking publisher, update cadence, source repository, signatures, or vulnerability history.
45. Using a full OS image when a minimal runtime image would work.
46. Using heavy language images in production when only the runtime binary/files are needed.
47. Using Alpine blindly without checking libc compatibility, debugging requirements, DNS behavior, Python wheels, native dependencies, or performance implications.
48. Using distroless/scratch blindly without ensuring observability, CA certificates, timezone files, user IDs, and debugging strategy are covered.
49. Mixing incompatible distributions across build and runtime stages.
50. Pinning by digest and then never updating.
51. Not pinning at all, allowing surprise rebuild changes.
52. Not rebuilding base images regularly.
53. Not using `--pull` or an equivalent update process when rebuilding.
54. Trusting “small image size” as equivalent to “secure image.”
55. Using images from abandoned repositories.
56. Using base images with package managers, compilers, shells, and debug tools in the final production stage.
57. Installing dev dependencies in production images.
58. Installing test-only, lint-only, or build-only dependencies in runtime.
59. Not using lockfiles.
60. Ignoring `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `poetry.lock`, `Pipfile.lock`, `requirements.txt` hashes, `go.sum`, `Cargo.lock`, `Gemfile.lock`, or equivalent.
61. Using `npm install` instead of `npm ci` in CI/reproducible builds.
62. Using `pip install` without pinned versions or hashes for production.
63. Using `apt-get upgrade` casually inside images.
64. Installing packages without version constraints where determinism matters.
65. Installing recommended packages by default when not needed.
66. Not removing package-manager indexes and temporary files.
67. Leaving `/var/lib/apt/lists`, package caches, wheels, downloaded tarballs, and temporary installers in the final image.
68. Installing compilers, headers, `make`, `gcc`, `g++`, `cmake`, `pkg-config`, `git`, `openssh-client`, or build chains in the final image.
69. Using `curl | bash` or `wget | sh`.
70. Disabling certificate validation with `--insecure`, `-k`, `strict-ssl false`, `GIT_SSL_NO_VERIFY`, or equivalent.
71. Using unauthenticated package repositories.
72. Using deprecated key-management patterns without validating repository signing.
73. Installing from random PPAs, GitHub release URLs, or shell scripts without verification.
74. Not checking checksums or signatures for downloaded binaries.
75. Pulling dependencies from a moving branch or tag.
76. Installing private dependencies using embedded credentials.
77. Leaving `.npmrc`, `.pypirc`, Maven settings, NuGet configs, SSH keys, or Git credentials in the image.
78. Running package managers as root in ways that leave root-owned app artifacts, forcing the final image to run as root.
79. `ARG AWS_SECRET_ACCESS_KEY`.
80. `ENV DATABASE_PASSWORD=...`.
81. `RUN export TOKEN=...`.
82. `RUN echo "$TOKEN" > file`.
83. `RUN git clone https://token@...`.
84. `COPY .env .`.
85. `COPY id_rsa /root/.ssh/id_rsa`.
86. `COPY .npmrc .`.
87. `COPY .aws /root/.aws`.
88. `COPY kubeconfig /root/.kube/config`.
89. Passing secrets via `--build-arg`.
90. Writing secrets to intermediate files and deleting them later.
91. Assuming deleting a secret in a later layer removes it from image history.
92. Putting secrets in image labels.
93. Printing secrets in build logs.
94. Using secret values in commands that appear in shell history, Docker history, provenance metadata, or cache keys.
95. Using remote package registries with tokens embedded in URLs.
96. Using SSH private keys directly instead of BuildKit SSH mounts.
97. Using `.env` as a “secret manager.”
98. Committing `.env` files because Compose needs them.
99. Treating Compose environment interpolation as secret handling.
100. Relying on obscurity of private registries instead of real secret hygiene.
101. No `USER` instruction.
102. `USER root` in the final image.
103. Creating an app user but never switching to it.
104. Running package-manager, build, and runtime steps all as root.
105. Running web servers, workers, cron jobs, queues, and app processes as root.
106. Using UID `0` because of file permission problems.
107. Fixing permissions with `chmod -R 777`.
108. Making application files world-writable.
109. Making config directories world-writable.
110. Giving write access to executable directories.
111. Using setuid/setgid binaries unnecessarily.
112. Leaving `sudo` installed.
113. Requiring root only to bind to low ports instead of using a higher internal port or a narrow capability.
114. Using numeric UIDs inconsistently across image and Compose volumes.
115. Not setting ownership during `COPY`.
116. Not considering arbitrary UID execution for platforms that enforce random UIDs.
117. Running as non-root but leaving runtime directories unwritable, causing crash loops and later “fixes” that reintroduce root.
118. Shell-form `CMD` or `ENTRYPOINT` for long-running services.
119. `ENTRYPOINT service nginx start && tail -f /dev/null`.
120. Running multiple unrelated daemons in one container without supervision.
121. Using `tail -f /dev/null` to keep containers alive.
122. Ignoring SIGTERM.
123. No graceful shutdown path.
124. Not using `exec` in wrapper scripts.
125. PID 1 process does not reap zombies.
126. Using shell scripts as PID 1 without `exec`.
127. Not using `tini` or `--init` when the process model needs an init.
128. Long startup scripts that mutate the container filesystem.
129. Startup scripts that run migrations unconditionally.
130. Startup scripts that wait with fixed sleeps instead of health checks.
131. No `HEALTHCHECK`, or disabling a useful inherited health check.
132. Health checks that require external internet access.
133. Health checks that mutate data.
134. Health checks that are too frequent, too expensive, or too slow.
135. Health checks that authenticate with production credentials.
136. Health checks that return healthy before the service is actually ready.
137. No multi-stage build.
138. Final image includes source code when only compiled artifacts are needed.
139. Final image includes compilers, headers, build systems, package managers, caches, tests, docs, and examples.
140. Final image includes `.git`, CI metadata, coverage files, and build reports.
141. Installing OS packages without `--no-install-recommends` when using Debian/Ubuntu.
142. Not deleting package-manager metadata in the same layer.
143. Not pruning dev dependencies after build.
144. Vendoring huge dependency trees without pruning.
145. Copying frontend source plus built frontend output.
146. Copying both compressed archives and extracted content.
147. Including local databases, fixtures, media, and backups.
148. Using full `node`, `python`, `golang`, `rust`, or `maven` images as runtime images.
149. Using language package caches in final runtime.
150. No `docker image history` review.
151. No image-size budget.
152. Pulling from untrusted registries.
153. Pulling by mutable tag only.
154. Not pinning critical images by digest.
155. Blindly trusting public images.
156. No vulnerability scanning.
157. No SBOM.
158. No image signing or verification.
159. No provenance/attestation for production images.
160. No dependency-review gate.
161. No base-image update workflow.
162. No rebuild schedule.
163. Ignoring critical CVEs in base images.
164. Using abandoned images.
165. Using images with unknown Dockerfiles.
166. Building from private forks without review.
167. Using build scripts downloaded at build time.
168. Pulling Git repos by branch instead of commit.
169. Using package mirrors without integrity controls.
170. Allowing dependency confusion through public package registries.
171. Copying dependencies from the host machine into the image.
172. Not separating dev, CI, and production registries.
173. Publishing images with secrets accidentally baked in.
174. Publishing debug images publicly.
175. Not scanning images after build and before deploy.
176. Not scanning running images over time.
177. Treating “it built successfully” as supply-chain validation.
178. `privileged: true`.
179. `cap_add: ["ALL"]`.
180. Adding broad capabilities such as `SYS_ADMIN`, `NET_ADMIN`, `SYS_PTRACE`, `SYS_MODULE`, `DAC_READ_SEARCH`, `MKNOD`, or `SYS_TIME` without a tight reason.
181. Not using `cap_drop`.
182. Not using `security_opt: ["no-new-privileges:true"]`.
183. `security_opt: ["seccomp=unconfined"]`.
184. `security_opt: ["apparmor=unconfined"]`.
185. Disabling SELinux labeling.
186. `pid: host`.
187. `ipc: host`.
188. `network_mode: host`.
189. `cgroup: host`.
190. `uts: host`.
191. `userns_mode: host`.
192. `user: root`.
193. No `user` when the image also has no `USER`.
194. Device mounts without strict need.
195. Mounting `/dev` wholesale.
196. Giving containers access to host kernel surfaces.
197. Using privileged containers for CI convenience.
198. Using privileged containers to run Docker-in-Docker by default.
199. Running browser automation, Selenium, test harnesses, or CI runners as privileged when a narrower setup would work.
200. Using lifecycle hooks with privileged execution.
201. Adding `SYS_ADMIN` as a generic fix for permission errors.
202. Adding `NET_ADMIN` as a generic fix for networking errors.
203. Adding `SYS_PTRACE` in production for debugging and forgetting to remove it.
204. Mounting `/var/run/docker.sock:/var/run/docker.sock`.
205. Mounting Docker credentials such as `~/.docker:/root/.docker`.
206. Using Compose’s `use_api_socket` without a strong trust boundary.
207. Exposing Docker TCP API on `2375`.
208. Exposing Docker API without TLS.
209. Letting app containers control the Docker daemon.
210. Letting CI job containers control the host Docker daemon without isolation.
211. Letting web apps, dashboards, test runners, or admin UIs mount the Docker socket.
212. Combining Docker socket access with root, privileged mode, or writable host mounts.
213. Treating Docker socket access as less dangerous than SSH access.
214. Giving Docker group membership to untrusted users or processes.
215. Hardcoding passwords in `docker-compose.yml`.
216. Hardcoding tokens in `docker-compose.yml`.
217. Storing secrets in `.env`.
218. Committing `.env`.
219. Using `.env.example` with real-looking reusable credentials.
220. Using weak defaults like `${POSTGRES_PASSWORD:-password}`.
221. Using `environment:` for secrets.
222. Using `env_file:` for secrets.
223. Passing cloud keys, Git tokens, package registry tokens, private keys, database passwords, JWT secrets, and API keys as environment variables.
224. Assuming environment variables are hidden.
225. Forgetting that environment variables can appear in inspect output, process environments, crash dumps, logs, and debugging output.
226. Putting secrets in labels.
227. Putting secrets in command-line arguments.
228. Echoing secrets at startup.
229. Using the same secret in dev, staging, and prod.
230. Sharing one secret across all services.
231. Granting every service access to every secret.
232. Mounting a whole secrets directory into every container.
233. Not rotating secrets.
234. Not documenting secret ownership and rotation process.
235. Using Compose interpolation that fails open to insecure defaults.
236. Mixing configuration and secrets in the same file.
237. Letting debug/admin containers read production secrets.
238. Publishing database ports to the host unnecessarily.
239. Publishing Redis, Memcached, Elasticsearch, RabbitMQ, MongoDB, Postgres, MySQL, admin UIs, metrics endpoints, debug ports, and internal APIs to `0.0.0.0`.
240. Using `ports: ["5432:5432"]` when only internal service-to-service access is needed.
241. Not binding local-only services to `127.0.0.1`.
242. Assuming `ports` are local-only by default.
243. Using `network_mode: host` for convenience.
244. Putting every service on one flat network.
245. Putting frontend, backend, database, admin, monitoring, and debug tools on the same network.
246. Not using separate frontend/backend/internal networks.
247. Using external networks without checking who else can attach.
248. Naming networks globally in ways that collide across projects.
249. Relying on `links`.
250. Using static IPs unnecessarily.
251. Using `extra_hosts` to hardcode infrastructure IPs.
252. Using `host.docker.internal` as a production dependency.
253. Exposing health, metrics, profiler, debug, or admin endpoints publicly.
254. Publishing the Docker daemon, registry, or reverse-proxy dashboard accidentally.
255. Confusing Dockerfile `EXPOSE` with Compose `ports`.
256. Not quoting port mappings, which can trigger YAML parsing surprises.
257. Forgetting IPv6 exposure.
258. Not documenting which ports are public, local-only, and internal-only.
259. Bind-mounting the host root: `/:/host`.
260. Bind-mounting `/etc`.
261. Bind-mounting `/var`.
262. Bind-mounting `/home`.
263. Bind-mounting `/root`.
264. Bind-mounting `/proc`.
265. Bind-mounting `/sys`.
266. Bind-mounting `/dev`.
267. Bind-mounting `/var/lib/docker`.
268. Bind-mounting `/run`.
269. Bind-mounting `/var/run`.
270. Bind-mounting `/var/run/docker.sock`.
271. Bind-mounting `/etc/passwd`, `/etc/shadow`, or `/etc/group`.
272. Bind-mounting `~/.ssh`.
273. Bind-mounting `~/.aws`.
274. Bind-mounting `~/.kube`.
275. Bind-mounting `~/.docker`.
276. Bind-mounting cloud CLI config directories.
277. Bind-mounting the whole repository into production containers.
278. Bind-mounting source code read-write in production.
279. Bind-mounting config files read-write when read-only is enough.
280. Not using `:ro` for read-only host paths.
281. Using short volume syntax that creates missing host paths silently.
282. Accidentally creating empty host directories that hide image files.
283. Using anonymous volumes for important data.
284. Not backing up named volumes.
285. Storing databases on ephemeral container layers.
286. Storing production data in bind mounts under random relative paths.
287. Using relative host paths in production without a known working directory.
288. Sharing one writable volume across unrelated services.
289. Sharing a writable app directory between web and worker when only uploads/cache are needed.
290. Mounting host package caches into untrusted build containers.
291. Mounting credentials and caches into CI containers.
292. Not using `tmpfs` for temporary sensitive data.
293. Not documenting persistence and backup requirements.
294. No `read_only: true` for services that do not need to write to the root filesystem.
295. Not using `tmpfs` for `/tmp`, `/run`, or ephemeral runtime files.
296. Making the entire container filesystem writable by default.
297. Using writable root filesystem to hide bad app behavior.
298. Writing logs into the container filesystem instead of stdout/stderr or a managed volume.
299. Writing uploads, queues, databases, and caches into ephemeral container layers.
300. Not separating persistent data, ephemeral data, and secrets.
301. Giving write permission to directories that contain binaries.
302. Failing to make config mounts read-only.
303. Using world-writable temp directories without `noexec` or `nosuid` where appropriate.
304. Not cleaning temp files.
305. Using `chmod -R 777` because mounted volume ownership is wrong.
306. Relying on container-local state for clustered services.
307. No memory limit.
308. No CPU limit.
309. No PID limit.
310. No ulimits.
311. No file-descriptor limit.
312. No process-count control.
313. No swap policy.
314. Setting memory limits too low and causing constant OOM kills.
315. Setting memory limits too high and allowing host exhaustion.
316. `oom_kill_disable: true` without strong justification.
317. `pids_limit: -1` for services that do not need unlimited processes.
318. Running databases, browsers, and JVMs without memory planning.
319. Running untrusted workloads without CPU limits.
320. Allowing logging to fill disk.
321. Allowing temporary files to fill disk.
322. Allowing queues/cache directories to grow unbounded.
323. Not load-testing with configured limits.
324. Not setting reservations where the orchestrator supports them.
325. Assuming Compose `deploy.resources` always behaves identically across local Docker, Swarm, and other runtimes without verification.
326. No alerting on memory, CPU, PID, disk, or restart loops.
327. No health checks.
328. Health checks that only check “process exists.”
329. Health checks that return healthy before dependencies are ready.
330. Health checks that mutate state.
331. Health checks that leak credentials.
332. Health checks that are too noisy.
333. Health checks with unrealistic timeouts.
334. Disabling an inherited health check without replacement.
335. Relying on `depends_on` short syntax to mean “database is ready.”
336. Using `sleep 30` instead of readiness checks.
337. Startup scripts that crash when dependencies are not yet ready.
338. `restart: always` hiding crash loops.
339. No restart policy for services that should recover.
340. `restart: always` for one-shot jobs or migrations.
341. Restarting migration jobs until they corrupt or conflict.
342. No max retry for fragile services.
343. No backoff strategy outside Compose.
344. No clear separation between app, migration, worker, scheduler, and one-shot job behavior.
345. No log rotation.
346. Default `json-file` logs allowed to grow indefinitely.
347. App logs written only to files inside the container.
348. Logs include secrets, tokens, passwords, cookies, Authorization headers, or database URLs.
349. Debug logging enabled in production.
350. No structured logs.
351. No service labels/metadata for log routing.
352. No metrics endpoint protection.
353. Publishing metrics endpoints publicly.
354. Publishing tracing/debug endpoints publicly.
355. No health status visibility.
356. No restart-loop alerts.
357. No image/version labels.
358. No SBOM/provenance tracking.
359. No container name/version/revision labels.
360. No separation of access logs and application logs.
361. Using `tty: true` or `stdin_open: true` in production and confusing log behavior.
362. Not documenting how to inspect failures.
363. `image: app:latest`.
364. `image: postgres:latest`.
365. `image: redis`.
366. Using mutable tags for production services.
367. Using `build: .` in production Compose instead of deploying a known immutable image.
368. Building production from a developer’s local checkout.
369. No digest pinning for critical production images.
370. No registry namespace, causing accidental pulls from Docker Hub.
371. No image update process.
372. No vulnerability scan on pulled images.
373. Mixing `build:` and `image:` ambiguously.
374. Rebuilding images during deployment when deployment should only pull.
375. Using dev Dockerfile targets in production.
376. Using production Dockerfile targets in dev and compensating with bind mounts and root.
377. No `pull_policy` strategy.
378. No platform strategy for multi-arch images.
379. Building with secrets through `args`.
380. Compose build args contain tokens.
381. Compose build context includes secrets.
382. Compose build context is the whole monorepo.
383. Compose profiles are not used, so every debug/admin/test service starts by default.
384. `container_name` used everywhere.
385. Assuming services can scale while using fixed `container_name`.
386. Hardcoding hostnames that conflict across Compose projects.
387. One Compose file used for dev, test, staging, and production with many unsafe conditionals.
388. No override-file strategy.
389. No profiles for optional services.
390. Dev-only services included in production.
391. Adminer/phpMyAdmin/mailhog/browser-debug/profilers exposed by default.
392. Test containers share production secrets.
393. Migration service runs automatically every time without guardrails.
394. Cron/scheduler duplicated across scaled replicas.
395. Workers and web services share the same command accidentally.
396. Multiple processes crammed into one service instead of separate services.
397. No labels or ownership metadata.
398. No documented service dependencies.
399. No network segmentation.
400. No resource segmentation.
401. No clear persistent data ownership.
402. Assuming Compose is a production orchestrator without addressing host restart, backups, upgrades, monitoring, and secrets.
403. Running CI jobs with `privileged = true` by default.
404. Mounting the host Docker socket into CI job containers.
405. Using Docker-in-Docker privileged mode when a safer builder would work.
406. Using `docker:latest`.
407. Using `dind:latest`.
408. No pinning of builder images.
409. Passing registry credentials via CLI arguments.
410. Printing registry credentials in logs.
411. Mounting `~/.docker`, `~/.ssh`, `~/.aws`, `~/.kube`, Maven, npm, or pip credentials into build containers.
412. Sharing host caches with untrusted jobs.
413. Running untrusted pull-request builds with write-capable registry credentials.
414. Running untrusted builds with access to production secrets.
415. Publishing images from unreviewed branches.
416. Tagging every branch image as `latest`.
417. Overwriting production tags.
418. No immutable release tags.
419. No provenance.
420. No vulnerability scan gate.
421. No secret scan.
422. No Dockerfile lint.
423. No Compose config validation.
424. No `docker compose config` rendering check.
425. No diff review for Compose privilege changes.
426. No policy that blocks `privileged`, Docker socket, host namespaces, public database ports, or root users.
427. No cache strategy, causing slow CI builds.
428. Using `--no-cache` to hide nondeterminism.
429. Using `docker system prune -af` in shared CI runners and breaking other jobs.
430. No registry cleanup policy.
431. CI builds depend on developer machine state.
432. CI builds pull from random package mirrors without integrity controls.
433. Optimizes for “it runs” and ignores security.
434. Uses `latest` because it is easy.
435. Adds `privileged: true` to fix permissions, browser automation, DinD, networking, or device access.
436. Mounts `/var/run/docker.sock` to let a container control Docker.
437. Adds `user: root` when permissions fail.
438. Adds `chmod -R 777` to fix volume errors.
439. Disables seccomp/AppArmor/SELinux.
440. Uses `network_mode: host` to fix connectivity.
441. Publishes internal ports to the host.
442. Publishes ports to all interfaces instead of localhost.
443. Stores credentials in Compose `environment`.
444. Creates `.env` files with real secrets.
445. Copies `.env` into images.
446. Adds build args for secrets.
447. Echoes secrets into config files during build.
448. Adds `curl -k`, `GIT_SSL_NO_VERIFY`, or disables TLS checks to bypass certificate errors.
449. Adds `sleep 30` instead of health-aware startup.
450. Removes health checks because they fail.
451. Removes resource limits because containers OOM.
452. Removes read-only filesystem because the app writes somewhere.
453. Disables vulnerability/lint findings instead of fixing them.
454. Adds broad bind mounts instead of narrow named volumes.
455. Mounts the entire project into production containers.
456. Treats dev Compose as production Compose.
457. Adds debug/admin services without profiles.
458. Uses fixed `container_name` everywhere.
459. Breaks service scaling.
460. Combines build and runtime stages.
461. Installs compilers/debug tools into runtime.
462. Leaves package caches and source code in final images.
463. Adds `tail -f /dev/null` to keep failed services alive.
464. Uses shell-form entrypoints that mishandle signals.
465. Makes mutable tags and overwrites releases.
466. Does not explain security tradeoffs in the diff.
467. Does not produce rollback instructions.
468. Does not check generated YAML with `docker compose config`.
469. Does not check the resulting image with `docker history`, scanners, or linters.
470. Does not minimize privileges after making a service work.
471. Does not distinguish local development convenience from production safety.

## Bad user, permission, and filesystem choices

1. No `USER` instruction in final image.
2. `USER root` in final image “because permissions are hard.”
3. Creates a non-root user but leaves app files owned by root and writable only by root.
4. Fixes permissions with `chmod -R 777`.
5. Runs package managers at container startup.
6. Leaves `sudo` installed in runtime image.
7. Leaves SSH server installed in runtime image.
8. Leaves compilers and linkers installed in runtime image.
9. Leaves setuid/setgid binaries unnecessarily.
10. Makes application source writable at runtime when it only needs read access.
11. Stores runtime data inside the container writable layer instead of a volume.
12. Uses root-owned volumes and then switches service back to root.
13. Uses unpredictable user names rather than stable numeric UID/GID for production.
14. Uses `chown -R` during startup, slowing every container start.
15. Gives the container write access to configuration files that should be immutable.
16. Assumes non-root automatically fixes all privilege problems. It helps, but capabilities, mounts, devices, and Docker socket access can still dominate risk.

## Base image bad behavior

1. **`FROM ubuntu:latest`, `node:latest`, `python:latest`, etc.** [R/S] Mutable and non-reproducible.
2. **Untagged images**. [R/S]
3. **Using random Docker Hub images from unknown publishers**. [S]
4. **Using unofficial clones of official images**. [S]
5. **Using abandoned images**. [S/R]
6. **Using end-of-life OS bases**. [S/R]
7. **Using a full OS image when a slim, distroless, scratch, or runtime-only image would work**. [P/S]
8. **Using Alpine blindly for everything**. [R/M] It can be great, but musl/glibc differences can break native dependencies.
9. **Using `scratch` blindly without CA certificates, timezone data, or required runtime files**. [R]
10. **Pinning digests but never updating them**. [S/R] Digest pinning improves reproducibility but can freeze vulnerabilities forever.
11. **Not documenting base image ownership and update cadence**. [M/S]
12. **Using different base image versions across stages accidentally**. [R/S]
13. **Using a build-stage base as the production base**. [P/S]
14. **Using images with shells/package managers/debug tools in production when unnecessary**. [S/P]
15. **No multi-arch awareness**. [R/P] Pulls wrong platform, slow QEMU builds, native dependency failures.

## Build pipeline

1. **Use BuildKit / buildx.**
2. **Use external cache for CI.**
3. **Use secret mounts, never build args for secrets.**
4. **Build once, promote the same image digest across environments.**
5. **Scan before push and/or before deploy.**
6. **Generate SBOM/provenance where your tooling supports it.**
7. **Fail on critical policy violations.**
8. **Do not let untrusted PRs run privileged Docker jobs.**
9. **Do not mount the Docker socket into untrusted CI jobs.**
10. **Use isolated runners for privileged builds.**
11. **Use registry mirrors/caches to reduce build time and rate-limit failures.**
12. **Run Dockerfile linters and Compose policy checks.**

## Compose & Orchestration

1. Uses `privileged: true`.
2. Mounts Docker socket.
3. Uses `network_mode: host`.
4. Uses host PID/IPC/user namespace.
5. Adds broad capabilities.
6. Disables seccomp/AppArmor/SELinux.
7. Runs as root.
8. Stores secrets in `environment` or `.env`.
9. Publishes internal service ports.
10. Publishes ports to all interfaces accidentally.
11. Bind-mounts sensitive host paths.
12. Uses broad writable bind mounts.
13. Has no resource limits.
14. Has no log rotation.
15. Has no health checks.
16. Relies on `sleep`.
17. Uses `restart: always` to hide crashes.
18. Uses fixed `container_name` unnecessarily.
19. Does not use profiles for debug/admin services.
20. Uses mutable image tags.
21. Builds production from local source.
22. Puts every service on one network.
23. Does not separate public, private, admin, and data networks.
24. Does not document persistence and backups.
25. Compose may pull an existing image when the developer expected a build.
26. Local builds may differ from CI.
27. Registry state can affect local behavior.
28. `latest` hides which code is running.
29. Debugging becomes confusing.
30. Tests may run against stale images.
31. Developers may add `--no-cache` to compensate.
32. Supply-chain review becomes harder.
33. Rollbacks become tag-dependent.
34. Builds may be skipped unintentionally.
35. /tmp
36. no-new-privileges:true
37. "127.0.0.1:8080:8080"
38. frontend
39. backend
40. db_password
41. Digest-pinned image.
42. Non-root runtime.
43. Init process for signal handling and zombie reaping.
44. Read-only root filesystem.
45. Temporary writable storage is explicit.
46. All Linux capabilities dropped.
47. No new privileges.
48. Local-only port binding.
49. Network segmentation.
50. Compose secrets instead of environment secrets.
51. Healthcheck present.
52. Process, memory, and CPU limits.
53. Bounded restart policy.
54. type=registry,ref=ghcr.io/acme/web:buildcache
55. npmrc
56. default
57. security.insecure
58. network.host
59. seccomp:unconfined
60. apparmor:unconfined
61. /var/run/docker.sock:/var/run/docker.sock
62. /:/host
63. /etc:/etc
64. /proc:/proc
65. /sys:/sys
66. /dev:/dev
67. "5432:5432"
68. `user: "nonroot_uid:nonroot_gid"`.
69. `cap_drop: [ALL]`.
70. Minimal `cap_add`, only if justified.
71. `read_only: true` where practical.
72. `tmpfs` for writable temp paths.
73. No Docker socket mount.
74. No host root bind mount.
75. Read-only mounts when possible.
76. Compose secrets instead of env secrets.
77. Local-only port binding for dev-only services.
78. Internal networks for databases and caches.
79. Healthchecks.
80. No `network_mode: host` unless there is a documented exception.
81. SBOM/provenance for production images.
82. Specific image versions or digests.
83. Separate dev override files from production Compose files.
84. **Running a Compose file from the internet without reading it**. [S]
85. **Running Compose from a Git repo you do not trust**. [S]
86. **Using remote `include`/`extends` without pinning digests**. [S/R]
87. **Reviewing only the top-level Compose file, not the resolved output**. [S/M]
88. **Not running `docker compose config` before review/deploy**. [S/M] Docker recommends this to inspect resolved includes, extends, merged overrides, and interpolated variables.
89. **Using YAML anchors to hide dangerous settings**. [M/S]
90. **Using multiple override files where security settings are silently merged or appended**. [M/S]
91. **Keeping dev, test, debug, and production behavior in one file with unclear profiles**. [S/R/M]
92. **Accidentally enabling debug profiles in production**. [S]
93. **Leaving sample services in production Compose**. [S/R]
94. **No project name/namespace discipline, causing network/volume/container collisions**. [R]
95. **`container_name` everywhere**. [R] Compose cannot scale a service beyond one container when `container_name` is set.
96. **No comments or exception records for risky settings**. [M/S]
97. **Using deprecated Compose syntax assumptions without testing current Compose behavior**. [R/M]
98. **Building in production from local source instead of deploying reviewed images**. [S/R]
99. **`build: .` with no `image:` tag**. [R/Ops] Harder to push, scan, promote, and identify.
100. **`build.context: .` at monorepo root when only a subdirectory is needed**. [P/S]
101. **No Compose build secrets; using `args` for tokens**. [S]
102. **`build.privileged: true`**. [S]
103. **`build.network: host` without a narrow reason**. [S/R]
104. **`build.no_cache: true` in normal CI**. [P]
105. **No `pull: true` or base refresh strategy**. [S/R]
106. **No build target specified, so Compose builds a dev or fat stage**. [P/S/R]
107. **No SBOM/provenance when your pipeline expects them**. [S/M] Compose build spec supports `sbom` and `provenance`.
108. **`image: app:latest` in production**. [R/S]
109. **Unqualified image names**. [S/R] Ambiguous registry/namespace.
110. **No digest pinning for production images**. [R/S]
111. **`pull_policy: never` on production hosts**. [S/R]
112. **Relying on cached images on shared runners**. [S]
113. **No controlled update cadence**. [S/R]
114. **No clear distinction between build-time images and deploy-time images**. [M/Ops]
115. **Using short-form `depends_on` as if it means “ready.”** [R]
116. **No `healthcheck` for dependencies that need readiness**. [R]
117. **No app-level retry/backoff**. [R]
118. **Using `sleep` wrappers instead of health checks and retries**. [R]
119. **Health checks that require internet access**. [R]
120. **Health checks that depend on another service rather than local process health**. [R]
121. **Health checks that mutate data**. [R/S]
122. **Disabling health checks from base images without replacement**. [R]
123. **No `restart` policy for services expected to recover**. [R]
124. **`restart: always` for migration jobs**. [R]
125. **No `service_completed_successfully` for one-shot init/migration jobs where appropriate**. [R]
126. **Ignoring shutdown order and graceful termination**. [R]
127. **Assuming `deploy:` always works with local Compose**. [R] Compose spec says `deploy` support is optional and ignored if not implemented.
128. **Putting resource limits only under `deploy.resources` and never testing enforcement on the target runtime**. [R]
129. **No `mem_limit` / `mem_reservation` where supported**. [R]
130. **No `cpus` / CPU controls for heavy services**. [P/R]
131. **No `pids_limit`**. [S/R]
132. **No `ulimits` for services that can exhaust descriptors/processes**. [R/S]
133. **No `logging` rotation configuration**. [R/Ops]
134. **`oom_kill_disable: true` without deep review**. [R/S]
135. **No `shm_size` for browsers/Postgres/ML workloads that need it, causing flaky failures**. [R]
136. **Too-large `shm_size` or tmpfs with no size cap**. [R/S]
137. **No `init: true` for process trees**. [R]
138. **No `stop_grace_period` for stateful services**. [R]
139. **No environment-specific override validation**. [R/M]
140. /run:rw,noexec,nosuid,size=64m
141. "127.0.0.1:8080:3000"
142. pgdata:/var/lib/postgresql/data
143. **Treat every Compose file as code with host-level implications.** Docker’s trust model says Compose applies elevated privileges, host filesystem access, and other requests as written.
144. **Run `docker compose config` in CI and review the resolved output.** This catches merged overrides, interpolated variables, `include`, `extends`, and hidden risky fields.
145. **Separate dev and production files.** Dev Compose can have bind mounts and debug ports; prod Compose should not inherit them accidentally.
146. **Bind local-only ports to `127.0.0.1`.** Never rely on host firewall assumptions for Docker-published ports.
147. **Use Compose secrets, not environment variables, for sensitive values.** Docker says Compose secrets avoid env-var exposure and are granted per service.
148. **Use long-form `depends_on` with health checks for readiness.** Short-form `depends_on` starts containers in order but does not wait for health.
149. **Quote port mappings.** Compose recommends quoted `HOST:CONTAINER` strings to avoid YAML base-60 float parsing issues.
150. **Do not assume `deploy:` constraints apply everywhere.** Test the target runtime; Compose spec allows unsupported `deploy` sections to be ignored.
151. **Pin remote Compose references and images by digest for production.** Docker recommends digest pinning for remote Compose dependencies because tags are mutable.
152. Running untrusted Compose files.
153. Running remote Compose includes without review.
154. Trusting nested `include` or `extends` chains.
155. Not reviewing `docker compose config`.
156. Using override files that silently add debug ports or root privileges.
157. Keeping `docker-compose.override.yml` enabled in production.
158. Assuming `version: "3"` gives Swarm semantics in modern Compose. The Compose spec merged legacy 2.x and 3.x formats; current Compose uses the latest implemented schema.
159. Using `deploy:` settings locally and assuming every Compose implementation enforces them. Compose says `deploy` is optional and ignored if unsupported.
160. Duplicate YAML keys where one silently overrides another.
161. Unquoted port mappings such as `80:80`; Docker recommends quoting ports to avoid YAML base-60 ambiguity.
162. Unquoted boolean-like environment values such as `true`, `false`, `yes`, `no`; Compose warns booleans should be quoted.
163. Using old `docker-compose` v1 assumptions with Docker Compose v2.
164. Generating Compose that works only on Docker Desktop but not Linux servers.
165. Generating Linux-only host paths for cross-platform developer teams.
166. Overusing `container_name`, which can cause name collisions and scaling problems.
167. Hardcoding project names and network names that collide on shared hosts.
168. Not separating dev, test, CI, and production Compose files.
169. `build.context: .` at a monorepo root without `.dockerignore`.
170. `build.context: ../..` pulling huge unrelated context.
171. Build context contains secrets.
172. Build context contains `.git`.
173. Build context contains test data, dumps, logs, `node_modules`, virtualenvs, or compiled artifacts.
174. No `target` specified for multi-stage Dockerfiles.
175. Using production Compose to build dev stages.
176. Using dev Compose to deploy production.
177. `args` used for secrets.
178. Compose build secrets not used when needed.
179. No BuildKit cache.
180. No platform awareness.
181. Forcing `platform: linux/amd64` on ARM Macs, causing slow emulation.
182. Building images from remote Git URLs without pinning commit.
183. Using `image: myapp:latest` for every local and CI build.
184. Race conditions where multiple CI jobs push the same tag.
185. Not tagging images with commit SHA.
186. Not pushing immutable release tags.
187. No `pull_policy` or update policy.
188. Not using `--pull` or equivalent periodic base refresh.
189. Not separating image build from service orchestration in CI.
190. Running builds on production hosts.
191. Using Docker-in-Docker privileged mode where a rootless or remote builder would work.
192. Not using Bitbucket/GitLab/GitHub cache features where available; GitLab and Bitbucket both document Docker/cache patterns for faster CI.
193. Debug ports published.
194. Hot reload enabled in production.
195. Source bind mounts enabled in production.
196. Development credentials in production Compose.
197. Test databases seeded with realistic sensitive data.
198. `NODE_ENV=development`, `FLASK_DEBUG=1`, `DJANGO_DEBUG=True`, `RAILS_ENV=development`.
199. Browser automation containers shipped with production stack.
200. Local-only mock services active in staging/prod.
201. Profilers exposed without auth.
202. Dev-only reverse proxy rules.
203. Self-signed dev certs in production.
204. `extra_hosts` to internal resources copied from developer machines.
205. `.env.local` accidentally loaded.
206. Compose `profiles` not used to isolate dev services.
207. Profiles used but CI/prod enables all profiles accidentally.
208. Require code review for `Dockerfile`, `.dockerignore`, `compose.yaml`, override files, and CI build scripts.
209. Require review for any addition of:
210. `privileged`
211. `security_opt`
212. `pid: host`
213. `ipc: host`
214. `devices`
215. Docker socket mounts
216. host filesystem bind mounts
217. `user: root`
218. secrets/environment changes
219. Always run:
220. Review the fully rendered output, not just source YAML.
221. For remote includes, review all transitive includes.
222. Do not run untrusted Compose files directly.
223. Keep dev and prod Compose separate.
224. Use `profiles` for optional debug/admin/dev services.
225. Maintain an exception register for privileged containers.
226. Expire exceptions automatically.
227. app_net
228. Use `compose.yaml` or `compose.yml`.
229. Do not rely on obsolete `version` behavior.
230. Separate dev and prod files.
231. Use profiles for optional services.
232. Use private/internal networks.
233. Use service names instead of container IPs.
234. Use runtime secrets rather than env secrets.
235. Use non-root user.
236. Use `read_only: true` where possible.
237. Use `tmpfs` for writable temp directories.
238. Drop capabilities.
239. Add resource limits.
240. Add log rotation.
241. Use long-form `depends_on` where readiness matters.
242. Avoid host network, host PID, host IPC.
243. Avoid broad host bind mounts.
244. Review rendered config with `docker compose config`.
245. [ ] Untrusted remote include
246. [ ] No `docker compose config` review
247. [ ] `privileged: true`
248. [ ] `cap_add: ["ALL"]`
249. [ ] `SYS_ADMIN`
250. [ ] `network_mode: host`
251. [ ] No non-root user
252. [ ] No `cap_drop`
253. [ ] No `no-new-privileges`
254. [ ] `seccomp=unconfined`
255. [ ] `apparmor=unconfined`
256. [ ] Docker socket mount
257. [ ] Broad host filesystem mount
258. [ ] Sensitive credential mounts
259. [ ] Writable config bind mounts
260. [ ] Database/cache ports published to all interfaces
261. [ ] Ports missing `127.0.0.1` for local-only services
262. [ ] Secrets in `environment`
263. [ ] Secrets in committed `.env`
264. [ ] All services receive all secrets
265. [ ] No healthchecks
266. [ ] Short `depends_on` used as readiness
267. [ ] No resource limits
268. [ ] No log rotation
269. [ ] Source bind mounts in production
270. [ ] `docker compose down -v` in routine scripts
271. [ ] Unquoted ports
272. [ ] Unquoted boolean env values
273. [ ] `deploy:` settings assumed to be enforced locally
274. [ ] `latest` image tags
275. [ ] No immutable tag or digest
276. **Breaking isolation:** Docker socket, privileged containers, host networking, host PID/IPC, broad devices, broad bind mounts.
277. **Leaking secrets:** `ARG`, `ENV`, `.env`, build logs, copied credential files, committed secret files.
278. **Destroying reproducibility and supply-chain trust:** mutable tags, untrusted images, unpinned downloads, no scanning, no SBOM/signing/provenance.
279. **Making builds painfully slow:** huge contexts, bad layer ordering, no BuildKit, no cache mounts, no external CI cache, bloated runtime images.
280. /run:rw,noexec,nosuid,size=16m
281. Broad `cap_add`, especially `ALL` or `SYS_ADMIN`.
282. No `cap_drop`.
283. No `no-new-privileges`.
284. Docker socket mount or `use_api_socket`.
285. Host root/sensitive bind mounts.
286. Read-write mounts where read-only is enough.
287. Bind-mounted source in production.
288. Published databases/caches/admin UIs.
289. Ports not bound to `127.0.0.1` when local-only.
290. Assumes firewall blocks Docker-published ports.
291. Overuse of external shared networks.
292. Secrets in `environment`, `.env`, `env_file`, labels, commands.
293. Gives all secrets to all services.
294. Misuses `depends_on`.
295. No `pids_limit` or `ulimits`.
296. No restart policy, or wrong restart policy.
297. No graceful stop configuration.
298. No `init` for child-process workloads.
299. Build context too broad.
300. No `target` for multi-stage builds.
301. No Compose config validation.
302. `deploy:` settings assumed to work everywhere.
303. Anonymous volumes with important data.
304. No backup plan for volumes.
305. Hard-coded host paths.
306. Debug/dev overrides used in production.
307. **No `tmpfs` for required write paths.**
308. **Mounting `/` from the host.**
309. **Mounting `/etc`, `/root`, `/home`, `/var`, `/var/lib/docker`, `/var/run`, `/run`, `/proc`, `/sys`, `/boot`, or cloud metadata/config paths.**
310. **Mounting `/var/run/docker.sock`.**
311. **Using `:ro` on `docker.sock` and thinking it is safe.**
312. **Using `volumes_from` to inherit unknown volumes.**
313. **Bind-mounting source code into production containers.**
314. **Bind-mounting local `node_modules`, `.venv`, Maven cache, or build artifacts into production containers.**
315. **Using anonymous volumes that mask image contents and make upgrades confusing.**
316. **Mounting a named volume over `/app`, hiding freshly deployed application files.**
317. **Using relative host paths in production Compose files.**
318. **Using developer-machine absolute paths in committed Compose files.**
319. **Not marking config and certificate mounts as read-only.**
320. **Putting secrets on writable shared volumes.**
321. **Sharing the same writable volume across unrelated services.**
322. **No backup/restore ownership strategy for stateful volumes.**
323. **Using `image: app:latest` in production.**
324. **Using `pull_policy: missing` with mutable tags and assuming the service updates.**
325. **Using `pull_policy: always` with unpinned mutable tags, causing surprise changes.**
326. **Using `pull_policy: never` in production, causing stale local images.**
327. **Using `build:` in production Compose instead of deploying immutable images.**
328. **Using `build.args` for secrets.**
329. **Using `build.context: .` from a monorepo root with no `.dockerignore`.**
330. **No `target:` for multi-stage builds when Compose should build a specific stage.**
331. **No build cache strategy in Compose/CI.**
332. **No image scan after Compose build.**
333. **Building different images locally and in CI from different Compose overrides.**
334. **Shipping dev-only Compose overrides into production.**
335. **Using debug images in production Compose.**
336. **Overriding image `command:` in Compose in a way that bypasses the hardened image entrypoint.**
337. **Running migrations, package installs, or compilers in `command:` at container startup.**
338. **Using `command: sh -c "npm install && npm start"` in production.**
339. **Using `stdin_open: true` and `tty: true` in production without reason.**
340. **Using short-form `depends_on` and assuming it waits for readiness.**
341. **No `healthcheck`.**
342. **Disabling healthchecks with `disable: true` to hide failures.**
343. **Healthchecks that require external services rather than local readiness.**
344. **Healthchecks that run expensive database queries every few seconds.**
345. **Healthchecks that leak credentials in `CMD-SHELL`.**
346. **Using `restart: always` for crash loops without limits or alerting.**
347. **No max retry limit for one-shot jobs.**
348. **Restarting migration jobs repeatedly.**
349. **No graceful shutdown settings.**
350. **No `stop_grace_period` for slow shutdown services.**
351. **Using `depends_on` as a substitute for application-level retry logic.**
352. **No readiness/backoff behavior in the app.**
353. **No memory limit.**
354. **No CPU limit.**
355. **No `pids_limit`.**
356. **No `ulimits`.**
357. **`oom_kill_disable: true` without hard memory controls.**
358. **Unlimited logs.**
359. **Logging secrets.**
360. **Using `logging.driver: none` in production, losing forensic visibility.**
361. **Using verbose/debug logging permanently.**
362. **No metrics or observability.**
363. **No alerting on container restarts, healthcheck failure, or resource saturation.**
364. **No disk-space monitoring for volumes and Docker storage.**
365. **Letting build cache grow forever on shared CI hosts.**
366. **“Fixes” permission errors by adding `privileged: true`.**
367. **“Fixes” permission errors by switching the whole container to `root`.**
368. **“Fixes” permission errors with `chmod -R 777`.**
369. **“Fixes” networking by publishing every port to `0.0.0.0`.**
370. **“Fixes” service discovery by using `network_mode: host`.**
371. **“Fixes” Docker-in-Docker by mounting `/var/run/docker.sock`.**
372. **“Fixes” CI by enabling privileged runners for untrusted branches.**
373. **Adds `ARG TOKEN` or `ENV TOKEN` because a private package install failed.**
374. **Copies `.env` into the image because the app cannot find config.**
375. **Adds `--no-cache` to make stale dependency bugs disappear, slowing every build.**
376. **Deletes `.dockerignore` entries because a file was missing from the build.**
377. **Uses `COPY . .` to avoid reasoning about required files.**
378. **Disables healthchecks because Compose waits too long.**
379. **Replaces long-form `depends_on.condition: service_healthy` with short-form `depends_on`.**
380. **Adds `restart: always` to hide crashes.**
381. **Installs missing packages at container startup.**
382. **Adds curl/bash/git/ssh/debug tools to the final runtime image.**
383. **Changes a pinned base image to `latest` to “update dependencies.”**
384. **Changes a slim/distroless runtime image to a full distro because debugging was easier.**
385. **Turns off TLS verification to make package download work.**
386. **Adds broad capabilities instead of identifying the one required capability.**
387. **Adds host device mappings to fix hardware access without scoping.**
388. **Adds `extra_hosts` or custom DNS entries that bypass internal controls.**
389. **Hardcodes developer paths into Compose.**
390. **Commits local override files that expose dev databases/admin tools.**
391. **Silences Docker build warnings rather than fixing them.**
392. **Suppresses `SecretsUsedInArgOrEnv` checks globally.**
393. **Runs `apt-get upgrade` in every build to “make it secure,” causing nondeterministic builds.**
394. **Mixes dev/test/prod dependencies in one image.**
395. **Removes resource limits because the app was OOMing instead of diagnosing memory behavior.**
396. **Disables seccomp/AppArmor/SELinux because something failed.**
397. **Makes all services share one network and one volume.**
398. **Does not inspect final image history, size, layers, users, capabilities, or exposed ports.**
399. Use immutable images, ideally digest-pinned.
400. Avoid `build:` in production deploy files.
401. Run as non-root.
402. Enable `no-new-privileges`.
403. Add `tmpfs` for writable temporary paths.
404. Publish only necessary ports.
405. Segment networks.
406. Use Compose secrets instead of environment variables for sensitive values.
407. Use long-form `depends_on` with `service_healthy` only where startup ordering is truly needed.
408. Add logging rotation.
409. Avoid host namespace modes.
410. Avoid host device mounts.
411. Keep debug/admin tools behind profiles, authentication, and non-public networks.
412. type: volume
413. Ensure the secret file is readable only by the appropriate local user or deployment mechanism.
414. For production, prefer a real secret manager or orchestrator-integrated secret system where available.
415. Use a reverse proxy or ingress for public traffic rather than publishing every app/admin port directly.
416. Some official images need write access to specific directories; use targeted writable volumes or `tmpfs`, not a fully writable root filesystem.

## Critical

1. Docker socket mounted into container.
2. `privileged: true`.
3. Host root bind mount.
4. Secrets copied into image.
5. Secrets in Compose env for production.
6. Exposed Docker daemon TCP without TLS.
7. CI privileged builds for untrusted code.
8. Root containers plus host mounts.
9. Disabled seccomp/AppArmor with elevated capabilities.
10. Publicly exposed databases/caches/admin consoles.

## Dockerfile bad behavior checklist

1. No `.dockerignore`.
2. `COPY . .` before dependency install.
3. Copies `.git`, `.env`, keys, tokens, package registry auth files.
4. Uses `ARG`/`ENV` for secrets.
5. Deletes secrets after writing them into layers.
6. Uses `curl | sh`.
7. Downloads binaries without checksum/signature.
8. Uses random/untrusted base images.
9. Uses `latest` without policy.
10. Pins digest but never updates.
11. No vulnerability scanning.
12. No SBOM/provenance/signing.
13. No multi-stage build.
14. Copies whole builder into runtime.
15. Leaves compilers/package managers/debug tools in runtime.
16. Installs unnecessary packages.
17. Splits `apt-get update` and `apt-get install`.
18. Does not clean apt lists.
19. Disables TLS/package verification.
20. Uses broad `chmod 777`.
21. Runs as root.
22. Uses shell-form entrypoint for long-running process.
23. Multiple `CMD`s.
24. Starts multiple unrelated daemons.
25. No health support.
26. Logs only to files.
27. Relies on `EXPOSE` as if it publishes or protects ports.
28. Bakes dev/test config into production image.
29. Does not handle signals.
30. Writes mutable state into image filesystem.
31. Uses `--no-cache` as standard workflow.
32. No CI cache.
33. No deterministic lockfile install.
34. No build secret mounts.
35. No rootless/userns consideration.

## Dockerfile checklist

1. [ ] `FROM ...:latest`
2. [ ] No digest pin for production-critical base images
3. [ ] No update process for pinned digests
4. [ ] No `.dockerignore`
5. [ ] `COPY . .` before dependency installation
6. [ ] Secrets in `ARG`
7. [ ] Secrets in `ENV`
8. [ ] `.env` copied into image
9. [ ] `.git` copied into image
10. [ ] SSH keys copied into image
11. [ ] `curl | sh`
12. [ ] Remote downloads without checksum/signature
13. [ ] `apt-get update` alone
14. [ ] Package install without cleanup
15. [ ] Build tools in final image
16. [ ] Final image runs as root
17. [ ] `chmod -R 777`
18. [ ] No multi-stage build for compiled apps
19. [ ] Huge base image without reason
20. [ ] No cache mounts in slow dependency builds
21. [ ] No healthcheck for long-running service
22. [ ] Shell-form entrypoint where signals matter
23. [ ] Startup script downloads dependencies
24. [ ] No image scanning
25. [ ] No SBOM/provenance/signing for production

## Dockerfile command, shell, and process bad behavior

1. **Pipelines without `pipefail` where earlier failures matter**. [R/S]
2. **Assuming `/bin/sh` is Bash**. [R]
3. **Using Bash-specific syntax without installing Bash or using exec-form Bash**. [R]
4. **No `set -e` / `set -u` discipline in complex shell scripts**. [R]
5. **Unquoted shell variables**. [S/R]
6. **Ignoring command failures with `|| true`**. [R/S]
7. **Using shell-form `CMD` or `ENTRYPOINT` for long-running services without understanding signal handling**. [R]
8. **No proper PID 1 behavior**. [R] Processes may not receive signals or reap zombies.
9. **No `init: true` in Compose when a service spawns child processes**. [R] Compose supports `init` to forward signals and reap processes.
10. **Multiple `CMD` instructions**. [M/R] Only the last one is effective.
11. **Multiple `ENTRYPOINT` instructions**. [M/R]
12. **ENTRYPOINT scripts that swallow signals**. [R]
13. **ENTRYPOINT scripts that run migrations/destructive actions unconditionally**. [R/S]
14. **ENTRYPOINT scripts that `chown -R` large directories on every startup**. [P/R]
15. **Using `sleep 30` instead of health checks/retries**. [R]
16. **Using `tail -f /dev/null` to keep broken containers alive**. [R/M]
17. **Running SSHD inside an app container unnecessarily**. [S/M]
18. **Running systemd inside a container just to manage one process**. [R/S/M]
19. **Bundling web server, worker, database, cache, scheduler, and admin UI into one image**. [R/S/M]
20. **No `WORKDIR` or relying on inherited `WORKDIR`**. [R/M]
21. **Relative `WORKDIR` chains that become unclear**. [M/R]
22. **Using `sudo` in containers**. [S/M]
23. **Switching back to root near the end of the Dockerfile**. [S]
24. **Creating root-owned app files then needing root at runtime**. [S/R]
25. **Using `chmod -R 777` as a permission fix**. [S]
26. **Not using `COPY --chown` when copying files for a non-root runtime user**. [S/R/P]
27. **Using low or conflicting UIDs/GIDs for app users**. [R/S]
28. **No deterministic UID/GID when bind mounts or Kubernetes security contexts need stable ownership**. [R]

## Dockerfile instruction misuse

1. **Using `ADD` when `COPY` is sufficient**. [M/S] OWASP’s CI checks include avoiding `ADD` in favor of `COPY`.
2. **Using `ADD` for remote URLs casually**. [S/R]
3. **Relying on `ADD` auto-extraction without making it obvious**. [M/S]
4. **Using `EXPOSE` as if it secures or publishes ports**. [M/R] It documents intent; it does not enforce access control.
5. **Misleading `EXPOSE` values**. [M/Ops]
6. **No `HEALTHCHECK` for long-running services**. [R]
7. **A health check that calls external systems instead of local readiness**. [R]
8. **A health check that mutates state**. [R/S]
9. **A health check that is too expensive**. [P/R]
10. **A health check with unrealistic intervals/timeouts/retries**. [R]
11. **Disabling a base image’s health check without replacing it**. [R]
12. **Using labels to store secrets or internal sensitive URLs**. [S]
13. **No useful labels for ownership, source repo, revision, license, or maintainer metadata**. [M/Ops]
14. **Using `ONBUILD` surprises in base images**. [M/S/R]
15. **Using `VOLUME` in app images in ways that hide files or make data lifecycle unclear**. [R/M]
16. **Relying on `docker commit` rather than source-controlled Dockerfiles**. [R/M/S]
17. **Building images manually outside CI with undocumented flags**. [R/M]

## Dockerfile review

1. Uses `latest` or untagged base images.
2. Uses an untrusted base image.
3. Omits `.dockerignore`.
4. Copies the whole repo before installing dependencies.
5. Does not use multi-stage builds for compiled/build-heavy apps.
6. Installs dev dependencies in runtime.
7. Leaves compilers/build tools in runtime.
8. Leaves package caches in runtime.
9. Uses `curl | sh`.
10. Disables TLS verification.
11. Does not verify downloaded binaries.
12. Uses `ARG` or `ENV` for secrets.
13. Copies `.env`, `.ssh`, `.aws`, `.kube`, `.docker`, `.npmrc`, or private keys.
14. Runs as root.
15. Uses `chmod -R 777`.
16. Uses shell-form `CMD`/`ENTRYPOINT` for long-running services.
17. Uses `tail -f /dev/null`.
18. Has no health check strategy.
19. Has no vulnerability/lint/scanning story.
20. Produces an image much larger than expected.
21. Cannot be rebuilt reproducibly.

## E. Bad CI/CD, runner, and automation behavior

1. **Privileged CI runners for untrusted code**. [S]
2. **Docker socket binding in CI jobs from forks or external contributors**. [S]
3. **Long-lived shared runners with privileged Docker-in-Docker**. [S]
4. **Non-ephemeral runners reused across trust boundaries**. [S]
5. **Production deploy credentials available in build jobs**. [S]
6. **Build secrets available to pull request jobs**. [S]
7. **No separation between build, test, scan, sign, and deploy permissions**. [S/Ops]
8. **No policy gate on `privileged`, docker socket, host mounts, host namespaces, latest tags, or root user**. [S]
9. **Ignoring Hadolint/KICS/Checkov/Trivy/Grype/Docker Scout findings without documented exceptions**. [S/M]
10. **Global “disable scanner” changes to make CI green**. [S]
11. **Only scanning source dependencies, not final image filesystem**. [S]
12. **Only scanning image, not Compose/IaC runtime config**. [S]
13. **No `docker compose config` artifact in CI for review**. [M/S]
14. **No SBOM/provenance artifacts**. [S/Ops]
15. **No image signing step**. [S]
16. **Deploying images by tag without confirming digest**. [R/S]
17. **No promotion workflow from dev → staging → prod by immutable digest**. [R/S]
18. **Building again separately for prod instead of promoting the tested image**. [R/S]
19. **Pushing from developer laptops directly to production registry**. [S/Ops]
20. **Running cleanup commands that delete volumes/caches in shared CI hosts**. [R/Ops]
21. **No audit logs for who built/pushed/deployed images**. [S/Ops]
22. **Letting agents auto-merge Docker/Compose changes without human review**. [S/M]
23. **Letting agents modify CI, Git hooks, build scripts, or Compose files without focused review**. [S] Docker’s sandbox docs also warn that agent-modified Git hooks, CI configs, IDE configs, and build scripts execute code later and must be reviewed.

## E. Bad runtime-user and filesystem behavior

1. **No `USER`; container runs as root.**
2. **Using `USER root` in final stage because a previous command failed under non-root.**
3. **Switching back and forth between root and non-root casually.**
4. **Using usernames without fixed UID/GID where host-mounted volume ownership matters.**
5. **Creating users without considering UID/GID collisions.**
6. **Making application directories world-writable.**
7. **Using `chmod 777` on app, config, log, or secret paths.**
8. **Leaving setuid/setgid binaries that are not required.**
9. **Installing `sudo` in application containers.**
10. **Making the entire root filesystem writable when the app only needs `/tmp` or one data directory.**
11. **Writing application state into the image filesystem instead of volumes or external services.**
12. **Writing logs only to files inside the container instead of stdout/stderr or managed log paths.**
13. **Assuming rootless inside the container equals rootless Docker daemon.**
14. **Using host UID 0 mappings without understanding user namespaces.**
15. **Mounting sensitive host directories into containers.**

## Entrypoint and process behavior mistakes

1. Shell-form `ENTRYPOINT` that swallows signals.
2. Shell-form `CMD` when signal behavior matters.
3. Entrypoint script that does not `exec "$@"`.
4. PID 1 process that does not reap children.
5. Running multiple unrelated daemons in one container.
6. Running `supervisord` to avoid designing proper services.
7. `tail -f /dev/null` to keep a broken container alive.
8. No `HEALTHCHECK` for long-lived services.
9. `HEALTHCHECK NONE` without a reason.
10. Healthcheck depends on tools not installed in final image.
11. Healthcheck mutates state.
12. Healthcheck logs secrets.
13. Startup script downloads dependencies from the internet every boot.
14. Startup script performs database migrations from every replica concurrently.
15. Startup script silently ignores failures.
16. Startup script runs as root then drops privileges incorrectly.
17. Entrypoint modifies mounted source code.
18. Entrypoint waits forever instead of failing clearly.

## F. Bad process, signal, and entrypoint behavior

1. **Shell-form `CMD` / `ENTRYPOINT` when exec form is needed.**
2. **Entrypoint scripts that do not `exec "$@"`.**
3. **Wrapper scripts that swallow SIGTERM and cause slow or unsafe shutdowns.**
4. **No `STOPSIGNAL` when the application needs a specific graceful signal.**
5. **Running multiple unrelated daemons in one container.**
6. **Running SSH server inside application containers for debugging.**
7. **Using `tail -f /dev/null` or `sleep infinity` as production command.**
8. **Starting background processes and exiting the main process.**
9. **Using process supervisors unnecessarily because the image is doing too many things.**
10. **No healthcheck for long-running services.**
11. **Disabling an upstream image healthcheck without replacing it.**
12. **Healthchecks that require external internet access rather than checking local readiness.**
13. **Healthchecks that mutate state.**
14. **Healthchecks that reveal secrets in command arguments or logs.**

## Filesystem and data lifecycle

1. **Writable root filesystem by default for services that do not need it**. [S/R]
2. **No `read_only: true` in Compose for stateless services**. [S]
3. **No `tmpfs` for `/tmp`, `/run`, or other writable ephemeral paths when root FS is read-only**. [R/S]
4. **No `noexec`, `nosuid`, size limits, or ownership options for tmpfs where appropriate**. [S/R]
5. **Writing application state into the container layer**. [R/Ops]
6. **Database data stored in container filesystem instead of a named volume or managed storage**. [R/Ops]
7. **Using anonymous volumes accidentally**. [R/Ops]
8. **No backup/restore plan for named volumes**. [R/Ops]
9. **Running `docker compose down -v` in production automation**. [R/Ops]
10. **Running `docker system prune -a --volumes` in shared/prod automation**. [R/Ops]
11. **Sharing one writable volume across unrelated trust boundaries**. [S/R]
12. **Letting containers `chown` host bind-mounted paths as root**. [R/S]
13. **Bind-mounting source code in production**. [S/R]
14. **Bind-mounting over important image paths and hiding built artifacts**. [R]
15. **Assuming bind mounts are portable across Docker Desktop, Linux, CI, and remote engines**. [R]

## G. Bad Dockerfile instruction behavior

1. **Using relative `WORKDIR` or relying on inherited working directories.**
2. **Using `RUN cd /somewhere` in one layer and expecting it to persist.**
3. **Multiple `CMD` instructions, expecting all of them to run.**
4. **Using `ENV` for temporary build values.**
5. **Using `ONBUILD` in base images without clear documentation.**
6. **Using `VOLUME` in application images in ways that mask files or make upgrades confusing.**
7. **Using `EXPOSE` as if it publishes a port.**
8. **Installing application dependencies globally when project-local installation works better.**
9. **Mixing dev/test/prod dependencies in the final image.**
10. **Leaving test frameworks, linters, and build systems in runtime images.**
11. **Hardcoding environment-specific URLs, credentials, regions, tenants, and feature flags in the image.**
12. **Not labeling images with source, revision, license, and build metadata where required by the organization.**
13. **Using `MAINTAINER` instead of modern labels.**
14. **Ignoring multi-architecture builds and accidentally shipping wrong-arch images.**
15. **Building on Apple Silicon and deploying to amd64 without explicit platform strategy.**
16. **Using `--platform` blindly, causing emulation-slow builds or wrong native dependencies.**
17. **Not testing final runtime image, only the builder stage.**
18. **Not checking final image size.**
19. **Not checking final image contents.**

## High

1. Mutable `latest` tags in production.
2. No image scanning.
3. No `.dockerignore`.
4. Running as root.
5. `network_mode: host`.
6. `cap_add: ALL` or `SYS_ADMIN`.
7. Missing resource limits.
8. Missing log rotation.
9. No SBOM/provenance/signing.
10. Unpinned installers or `curl | bash`.

## Image Layers & Caching

1. Full rebuild every time.
2. Slow CI.
3. More registry downloads.
4. More package-manager downloads.
5. More flaky builds due to transient network dependency.
6. More carbon and infrastructure cost.
7. Prevents BuildKit cache benefits.
8. Makes developers distrust Docker because every build is slow.
9. Hides incorrect Dockerfile layer ordering.
10. Can unintentionally pull changed dependencies unless versions are pinned.
11. type=registry,ref=ghcr.io/acme/app:buildcache
12. Using full OS base image when a slim runtime works.
13. Leaving package-manager caches.
14. Leaving source code in runtime image.
15. Leaving tests in runtime image.
16. Leaving docs, examples, `.git`, coverage, and build outputs.
17. Installing dev dependencies in production.
18. Installing compilers in production.
19. Using multiple package managers in the same image.
20. Creating artifacts in one layer and deleting them in a later layer.
21. Not using multi-stage builds.
22. **No `.dockerignore`**. [P/S] Sends `.git`, `node_modules`, `dist`, logs, caches, local secrets, test artifacts, Terraform state, SSH keys, and other junk to the builder.
23. **Using `COPY . .` as the first meaningful step**. [P/R] Any source change invalidates dependency-install cache.
24. **Copying the entire repository before `npm ci`, `pip install`, `go mod download`, `bundle install`, `cargo fetch`, etc.** [P/R] Causes slow rebuilds.
25. **Putting volatile files before stable expensive layers**. [P] Example: copying source before package manifests.
26. **Putting package installation after source code copy when dependencies rarely change**. [P]
27. **Adding huge directories to the build context unnecessarily**. [P/S] `node_modules`, `.venv`, `.m2`, `.gradle`, `.cache`, `target`, `build`, `coverage`, `tmp`, media assets, database dumps.
28. **Including `.git` in the image**. [S/P] Leaks history and secrets; bloats context and image.
29. **Including `.env`, `.npmrc`, `.pypirc`, `.netrc`, cloud credentials, SSH keys, kubeconfigs, or Docker credentials**. [S]
30. **Relying on host-local files not committed or not declared**. [R] Build only works on one developer’s machine.
31. **Using absolute local paths in `COPY` or Compose build context assumptions**. [R/M]
32. **Failing to use Dockerfile-specific `.dockerignore` files for multiple Dockerfiles**. [P/M]
33. **Using `--no-cache` by default in CI**. [P] Guarantees slow builds unless specifically needed.
34. **Never busting cache for security-sensitive dependency updates**. [S/R] The opposite failure: stale vulnerable packages.
35. **Not using cache mounts for package managers**. [P] BuildKit cache mounts can avoid redownloading unchanged package caches.
36. **Not using external build cache in CI**. [P]
37. **Using Docker-in-Docker without persistent cache and then blaming Docker for slow builds**. [P/S]
38. **Using remote builders while sending giant contexts**. [P/S]
39. **Using bind mounts during build in a way that hides inputs from reproducibility and review**. [R/M]
40. **Building from remote Git contexts without pinning commit hashes**. [S/R]
41. **Building from branches or tags controlled by someone else**. [S/R]
42. **Splitting `apt-get update` and `apt-get install` into separate `RUN` layers**. [S/R]
43. **Running `apt-get update` without installing anything in the same layer**. [S/R]
44. **No `--no-install-recommends` for Debian/Ubuntu packages**. [P/S]
45. **Not removing `/var/lib/apt/lists/*` in the same layer**. [P]
46. **Installing unnecessary packages “just in case”**. [P/S]
47. **Leaving compilers, headers, linkers, package managers, `git`, `curl`, `wget`, `bash`, `sudo`, `ssh`, `vim`, `nano`, `netcat`, `tcpdump`, or cloud CLIs in runtime images unnecessarily**. [S/P]
48. **Using `apt-get upgrade` or `dist-upgrade` casually inside app images**. [R/S] Can create unpredictable images and duplicate base-image responsibility.
49. **Not pinning critical package versions where reproducibility matters**. [R]
50. **Over-pinning everything without an update workflow**. [S/R]
51. **Installing packages from HTTP repositories**. [S]
52. **Disabling signature verification for package managers**. [S]
53. **Using `--allow-unauthenticated`, `--nogpgcheck`, `trusted=yes`, or equivalent**. [S]
54. **Adding third-party package repositories without pinning keys and scopes**. [S/R]
55. **Using deprecated `apt-key` patterns without repository scoping**. [S/M]
56. **Using `curl | sh` installers**. [S/R]
57. **Downloading binaries without checksum or signature verification**. [S/R]
58. **Using `ADD https://...` for remote downloads without checksum discipline**. [S/R]
59. **Installing language dependencies from floating branches or Git HEAD**. [R/S]
60. **Removing lockfiles to “fix” install problems**. [R/S]
61. **Using `npm install` where `npm ci` is appropriate**. [R/P]
62. **Installing dev dependencies in production images**. [P/S]
63. **Not clearing package manager caches where they persist into layers**. [P]
64. **Creating many package-install layers that duplicate metadata**. [P]
65. **Not sorting multi-line package lists**. [M] Makes duplicate packages and malicious additions harder to review.
66. **Using package manager commands in container startup scripts**. [P/R/S] Slow startup, network dependency, surprise changes at runtime.
67. **Single-stage builds that ship the whole compiler/toolchain**. [P/S]
68. **Copying the entire builder filesystem into runtime**. [P/S]
69. **Copying `/root`, package caches, build caches, test reports, coverage, `.gradle`, `.m2`, `.cargo`, `.cache`, or source trees into runtime unintentionally**. [P/S]
70. **Running tests in the final runtime stage instead of a dedicated test stage**. [P/M]
71. **Leaving test fixtures, mocks, seed databases, or sample credentials in production images**. [S/P]
72. **No named build stages**. [M/R] Stage-number references break when Dockerfile is reordered.
73. **Using `COPY --from=0` everywhere instead of named stages**. [M/R]
74. **Not using `--target` for CI test/build stages**. [P/M]
75. **Compiling in production container at startup**. [P/R]
76. **Shipping source code when only compiled artifacts are needed**. [S/P]
77. **Not using `.dockerignore` with multi-stage builds**. [P/S]
78. **Using huge base image plus huge build context and then trying to fix only the last layer**. [P]
79. **Always maintain a `.dockerignore`.** Include `.git`, `.env*`, `node_modules`, `.venv`, `dist`, `build`, `target`, `.cache`, logs, coverage, test output, local databases, Terraform state, cloud credentials, SSH keys, and editor files. Docker says `.dockerignore` avoids sending unwanted files and improves build speed, especially with remote builders.
80. **Order Dockerfile layers from least-changing to most-changing.** Copy lockfiles/manifests first, install dependencies, then copy source. Docker’s cache docs show this exact pattern to avoid reinstalling dependencies when project files change.
81. **Use multi-stage builds.** Compile/test in builder stages; copy only runtime artifacts into the final image. Docker notes multi-stage builds let you leave behind build tools and intermediate artifacts.
82. **Use BuildKit cache mounts for package managers.** Cache npm, pip, Maven, Gradle, Cargo, Go, Bundler, Composer, apt, and other caches where appropriate.
83. **Do not use `--no-cache` except for a reason.** Prefer targeted cache busting, scheduled clean builds, and `--pull` for base refresh.
84. **Combine package manager update/install/cleanup in one layer.** For Debian/Ubuntu, use `apt-get update && apt-get install -y --no-install-recommends ... && rm -rf /var/lib/apt/lists/*`. Docker explicitly documents why splitting `apt-get update` causes cache problems.
85. **Avoid unnecessary packages.** Every shell, compiler, network tool, package manager, or debug utility in a runtime image increases size and attack surface.
86. **Sort multiline package lists.** This reduces duplicate packages and makes reviews easier.
87. **Prefer reproducible install commands.** `npm ci`, lockfiles, pinned dependency manifests, and deterministic build commands beat floating installs.
88. **Measure image size and layer contents.** Use image inspection, SBOMs, and layer analyzers in CI for large services.
89. **Putting frequently changing files before dependency-install layers.**
90. **Not copying lockfiles separately.**
91. **Not using package lockfiles at all.**
92. **Using nondeterministic dependency installs such as `npm install` instead of `npm ci` for Node production builds.**
93. **Using `pip install -r requirements.txt` without pinned versions or a lock strategy.**
94. **Using `apt-get update` in one layer and `apt-get install` in a later layer.**
95. **Running `apt-get upgrade` blindly inside application images.**
96. **Installing recommended/suggested OS packages by default when unnecessary.**
97. **Not cleaning package-manager caches.**
98. **Not using BuildKit cache mounts for npm, pip, apt, Maven, Gradle, Cargo, Go, pnpm, yarn, etc.**
99. **Not using external build cache in CI.**
100. **Using `--no-cache` for every CI build by default.**
101. **Putting `ARG BUILD_DATE`, `ARG GIT_SHA`, or timestamp metadata early in the Dockerfile, invalidating all later layers.**
102. **Generating files with current timestamps before dependency installation.**
103. **Running expensive tests before cheap deterministic dependency/cacheable steps.**
104. **Doing dependency resolution during container startup rather than image build.**
105. **Building frontend assets at runtime on every container start.**
106. **Doing database migrations as part of image build.**
107. **Downloading large artifacts repeatedly instead of using cache mounts or prebuilt artifacts.**
108. **Not splitting build and runtime stages.**
109. **Overusing multi-stage builds in a way that defeats cache locality.**
110. **Not naming build stages.**
111. **Copying from stage numbers instead of named stages, making later edits fragile.**
112. **Copying entire builder filesystem into runtime image instead of explicit artifact paths.**
113. **Using `RUN chown -R` on large trees after `COPY` instead of `COPY --chown`.**
114. **Using `chmod -R 777` to fix build errors.**
115. **Using huge recursive operations late in the build that invalidate often.**
116. **Using `ADD` for local files when `COPY` is sufficient.**
117. **Using remote `ADD` without checksum.**
118. **Using shell commands that hide errors, for example pipelines without `pipefail`.**
119. **Not using the Dockerfile syntax directive when relying on BuildKit-only features.**
120. **Using BuildKit secrets/cache features without enabling or pinning a compatible Dockerfile frontend.**
121. **Using separate Dockerfiles per environment that drift wildly from one another.**
122. **Using Compose `build:` in production deploy paths instead of prebuilt, scanned, immutable images.**

## Low but worth fixing

1. Unsorted package lists.
2. Extra files in build context.
3. Non-portable paths.
4. Missing labels.
5. Overly broad Compose files combining dev/prod.
6. Anonymous volumes that accumulate.
7. Debug ports left in dev Compose.
8. Inconsistent image naming.
9. No comments explaining security exceptions.
10. No automated linting.
11. Make the build context smaller.
12. Make dependency layers cacheable.
13. Keep secrets out of layers, env, logs, and Compose files.
14. Use non-root containers.
15. Drop capabilities.
16. Avoid host namespaces and host mounts.
17. Bind ports narrowly.
18. Pin versions and produce digests.
19. Scan, sign, and attest images.
20. Keep dev convenience out of production Compose.

## Medium

1. Poor Dockerfile cache ordering.
2. No BuildKit cache mounts.
3. No multi-stage build.
4. Overly large base image.
5. Dev dependencies in runtime.
6. No healthcheck.
7. `restart: always` everywhere.
8. Absolute Compose build paths.
9. No `init: true` for processes that need child reaping.
10. Compose `build` + `image` behavior not understood.

## Networking & Ports

1. "127.0.0.1:5432:5432"
2. "0.0.0.0:443:8443"
3. "5432:5432"
4. "6379:6379"
5. "9200:9200"
6. "8080:8080"
7. Databases exposed to LAN or internet.
8. Redis/Elasticsearch/admin panels exposed accidentally.
9. Firewall expectations can be bypassed.
10. Local dev services become reachable by other machines.
11. Cloud security groups may expose the port.
12. Internal-only services become public.
13. Default credentials become internet-facing.
14. Attack surface expands.
15. Port collisions encourage unsafe workarounds.
16. Incident scope grows.
17. "127.0.0.1:8080:8080"
18. backend
19. Container shares host network namespace.
20. Port isolation is removed.
21. Localhost means host localhost.
22. Network monitoring/egress controls are harder.
23. Service can bind unexpected host ports.
24. SSRF blast radius can increase.
25. Local-only host services may become reachable.
26. Cross-platform behavior differs.
27. It prevents normal Compose network isolation.
28. Often used to avoid understanding Docker DNS.
29. Remote unauthenticated Docker control.
30. Host filesystem mount via Docker API.
31. Privileged container creation.
32. Container escape by design.
33. Full image/secret exposure.
34. Lateral movement.
35. Public internet compromise.
36. CI runner compromise.
37. No meaningful container isolation.
38. Root-equivalent control.
39. Bypasses Compose network isolation.
40. Exposes host network.
41. Makes behavior platform-specific.
42. Increases blast radius.
43. **Publishing internal database/cache/message-broker ports to the host**. [S]
44. **Publishing admin consoles to `0.0.0.0`**. [S]
45. **Publishing Docker daemon, metrics, debug, pprof, JMX, Redis, Postgres, MySQL, MongoDB, Elasticsearch, RabbitMQ, Prometheus, Grafana, Traefik dashboard, phpMyAdmin, Adminer, MailHog, etc. publicly**. [S]
46. **Using `ports` where `expose` or internal networks would suffice**. [S]
47. **Assuming `expose` publishes to the host**. [M/R]
48. **Using host networking to avoid understanding port mapping**. [S/R]
49. **No network segmentation; everything on the implicit default network**. [S]
50. **Using legacy `links` instead of networks**. [M/R]
51. **Using `extra_hosts` to override important names without documenting why**. [S/R]
52. **Using `host.docker.internal` or host gateway broadly**. [S]
53. **Depending on container IP addresses instead of service DNS names**. [R]
54. **No TLS or auth on service-to-service traffic when crossing trust boundaries**. [S]
55. **Using insecure registries or HTTP service URLs by default**. [S]
56. **Disabling certificate verification with `curl -k`, `NODE_TLS_REJECT_UNAUTHORIZED=0`, `GIT_SSL_NO_VERIFY=true`, etc.** [S]
57. **`ports: ["80:80"]` when service should be local-only**. [S]
58. **Unquoted port mappings**. [R/M] Compose recommends quoting `HOST:CONTAINER` to avoid YAML base-60 float issues.
59. **Using `ports` with `network_mode: host`**. [R] Compose warns this causes a runtime error because host mode already exposes ports directly.
60. **Exposing privileged host ports unnecessarily**. [S/Ops]
61. **Publishing random host ports and not documenting them**. [Ops/M]
62. **No explicit networks; relying on implicit default network**. [S/M] Compose connects services to an implicit `default` network when none are declared.
63. **Putting admin/debug/observability services on public frontend networks**. [S]
64. **`network_mode: host` as a shortcut**. [S/R]
65. **`network_mode: service:...` hiding namespace sharing**. [S/M]
66. **`external_links` / legacy `links` instead of explicit networks**. [M/R]
67. **No egress controls for agents or untrusted code**. [S]
68. **Assuming Docker Desktop, Linux Engine, Swarm, ECS, and CI network behavior are identical**. [R]
69. `ports: ["3306:3306"]` for MySQL/MariaDB.
70. Publishing Elasticsearch, MongoDB, RabbitMQ, Prometheus, Grafana, admin UIs, mail catchers, or debug dashboards by default.
71. Publishing ports without `127.0.0.1`.
72. Assuming Docker only binds localhost by default; it does not.
73. Using `network_mode: host` to avoid learning Compose networking.
74. Using `links` instead of normal service-name DNS. Docker says links are not required for services to communicate on the default network.
75. Hardcoding container IPs. Compose service containers can be recreated with different IPs; use service names.
76. Putting every service on one flat default network.
77. Giving every service outbound internet access when not needed.
78. Using `extra_hosts: host.docker.internal:host-gateway` for every service.
79. Adding custom DNS servers unnecessarily.
80. Exposing metrics endpoints publicly.
81. Exposing health endpoints with sensitive details.
82. Exposing admin consoles with default credentials.
83. Treating `expose` as host publishing. Compose `expose` is internal service exposure; `ports` publishes to the host.
84. Using host networking and `ports` together incorrectly; Compose notes port mapping must not be used with host network mode.
85. Binding to IPv6 unintentionally.
86. Publishing debug ports such as `9229`, `5005`, `5678`, `2345`.
87. Publishing ephemeral random host ports in production without firewall awareness.
88. Not documenting expected ingress paths.
89. "5432"
90. Do not publish databases by default.
91. Do not publish caches by default.
92. Bind local-only services to `127.0.0.1`.
93. Use reverse proxies for public ingress.
94. Use service names, not hardcoded IPs.
95. Avoid `network_mode: host`.
96. Treat every `ports:` entry as an exposure decision.
97. "443:443"
98. Public ingress service joins public network.
99. App joins only the networks it needs.
100. Database is not on the public network.
101. Avoid a single flat default network for everything.
102. Avoid `links`; Compose service DNS is enough for normal communication.
103. app_net
104. db_net
105. Publish only the reverse proxy or public API.
106. Bind developer-only ports to `127.0.0.1`.
107. Do not publish databases, queues, caches, admin consoles, or metrics endpoints unless access is intentionally controlled.
108. Avoid shared `external: true` networks unless there is explicit lifecycle ownership.
109. Prefer service-name DNS over static IPs.
110. **Publishing ports unnecessarily.**
111. **Publishing databases, Redis, Elasticsearch, RabbitMQ, admin consoles, metrics, mail catchers, or debug UIs to the internet.**
112. **Assuming host firewall rules still protect Docker-published ports.**
113. **Using `network_mode: host` for convenience.**
114. **Putting every service on the default network.**
115. **Using `expose` as if it prevents access.**
116. **Using legacy `links` instead of networks/service discovery.**
117. **Using broad `extra_hosts` to override DNS in dangerous ways.**
118. **Using public DNS resolvers in regulated/corporate environments where internal DNS policy matters.**
119. **Allowing containers to reach cloud metadata endpoints when not required.**
120. **No egress controls for services that should not access the internet.**
121. **Putting reverse proxy and databases on the same unrestricted network.**
122. **Using `container_name` everywhere.**
123. **Relying on fixed container names for service discovery instead of Compose service names.**

## Performance

1. [ ] `.dockerignore` is strict.
2. [ ] Dependency manifests copied before source.
3. [ ] Lockfiles are used.
4. [ ] Package-manager cache mounts are used where valuable.
5. [ ] External CI cache is configured.
6. [ ] Multi-stage build keeps runtime small.
7. [ ] Build metadata/timestamps do not invalidate early layers.
8. [ ] Expensive steps are placed after stable cached steps.
9. [ ] Final image size is measured and reasonable.

## Privilege and host access

1. **Granting untrusted users membership in the `docker` group**. [S] Docker states the daemon runs as root and the Docker group grants root-level privileges.
2. **Exposing Docker daemon over TCP without TLS/authentication**. [S]
3. **Leaving Docker API on `0.0.0.0:2375`**. [S]
4. **Mounting the Docker socket into app, CI, monitoring, or admin containers**. [S]
5. **Mounting containerd, CRI-O, Kubernetes, or Podman sockets casually**. [S]
6. **Mounting `/` from the host**. [S]
7. **Mounting `/etc`, `/root`, `/home`, `/var`, `/var/lib/docker`, `/proc`, `/sys`, `/dev`, `/boot`, `/run`, `/run/secrets`, cloud metadata sockets, kubeconfigs, SSH agent sockets, or host credential directories**. [S]
8. **Writable bind mounts where read-only would work**. [S] Docker says bind mounts are read-write by default and can change host files.
9. **Using broad device mounts like `/dev:/dev`**. [S]
10. **Using `devices: ["...:rwm"]` when only read or write is needed**. [S]
11. **Granting GPUs, KVM, FUSE, TUN/TAP, raw disks, USB buses, or serial devices without isolation review**. [S]
12. **`pid: host`**. [S]
13. **`ipc: host`**. [S]
14. **`network_mode: host`**. [S/R]
15. **`cgroup: host` or `cgroup_parent` misuse**. [S/R]
16. **`userns_mode: host` when user namespace isolation is expected**. [S]
17. **Disabling user namespace remapping when it would be feasible**. [S]
18. **Not considering rootless Docker where appropriate**. [S] OWASP recommends evaluating rootless mode for high-security environments.
19. **Assuming rootless Docker makes root-in-container harmless**. [S] It helps, but non-root inside the container is still preferable.

## Reliability and performance

1. [ ] No production `build:` unless intentionally building locally.
2. [ ] No `latest` for production.
3. [ ] `pull_policy` is deliberate.
4. [ ] Healthchecks exist.
5. [ ] `depends_on` uses `service_healthy` only where appropriate.
6. [ ] App has retry logic; Compose ordering is not the only readiness mechanism.
7. [ ] Memory, CPU, PID, and ulimit controls exist.
8. [ ] Logs have rotation.
9. [ ] Restart policy is appropriate and does not hide crash loops.
10. [ ] Volumes are named and intentional.
11. [ ] `container_name` is avoided unless there is a specific reason.
12. [ ] Compose overrides are separated by environment.
13. [ ] Production does not include dev bind mounts.
14. Add privilege instead of narrowing the root cause.
15. Add root instead of fixing ownership.
16. Add world-writable permissions.
17. Add host networking.
18. Add host namespace sharing.
19. Mount the Docker socket.
20. Publish ports to all interfaces.
21. Put secrets in `ARG`, `ENV`, `.env`, labels, or commands.
22. Use `latest`.
23. Disable healthchecks.
24. Disable seccomp/AppArmor/SELinux.
25. Remove resource limits.
26. Remove `.dockerignore`.
27. Use broad `COPY . .` before dependency installation.
28. Install dependencies at runtime.
29. Add build tools to runtime images.
30. Disable TLS verification.
31. Suppress build/security warnings rather than fixing them.
32. Mix dev Compose settings into production.
33. Make builds nondeterministic for convenience.

## Resource exhaustion and reliability

1. **No memory limit**. [S/R]
2. **No CPU limit or CPU shares on noisy workloads**. [P/R]
3. **No PID limit**. [S/R]
4. **No file descriptor limits**. [R]
5. **No process limits**. [S/R]
6. **No log size limit**. [R/Ops]
7. **Using the default `json-file` logging driver without rotation on busy services**. [R/Ops]
8. **Disabling OOM killer without a memory limit**. [R/S]
9. **Setting memory limits too low and causing crash loops**. [R]
10. **Setting CPU too low and causing timeout cascades**. [R]
11. **No `stop_grace_period` for databases, queues, or stateful services**. [R]
12. **No `stop_signal` when the app needs a non-default signal**. [R]
13. **No bounded restart policy**. [R]
14. **`restart: always` for one-shot jobs, migrations, or broken services**. [R]
15. **Restart storms that hide real failures**. [R/Ops]
16. **No monitoring of memory, CPU, disk, restarts, or health status**. [R/Ops]
17. **No host disk monitoring for Docker volumes, layers, logs, and build cache**. [R/Ops]

## Runtime hardening best practices

1. **Default to no privilege.** No `privileged`, no host namespaces, no host devices, no docker socket, no broad bind mounts.
2. **Drop Linux capabilities.** Start with `cap_drop: ["ALL"]`; add back only what the process truly needs. OWASP explicitly recommends this model.
3. **Use `no-new-privileges`.** In Compose: `security_opt: ["no-new-privileges:true"]`.
4. **Keep default seccomp/AppArmor/SELinux unless you have a narrow, reviewed need.**
5. **Use read-only root filesystems for stateless services.** Add tmpfs or named volumes only for specific writable paths.
6. **Make bind mounts read-only whenever possible.** Docker says bind mounts are writable by default and can modify host files.
7. **Set resource limits.** At minimum: memory, CPU, PIDs, file descriptors, and log rotation.
8. **Do not disable the OOM killer casually.** Docker warns this can be dangerous without memory limits.
9. **Use rootless Docker or user namespace remapping where feasible.** OWASP recommends evaluating rootless mode for strong isolation needs.
10. **Keep host kernel, Docker Engine, Compose, BuildKit, and runtimes updated.** OWASP notes containers share the host kernel, so host/kernel vulnerabilities affect containers.

## Scope and caveat

1. **Running containers as root** or omitting `USER`.
2. **Using `privileged: true`** without a narrowly justified reason.
3. **Mounting `/var/run/docker.sock`** into containers.
4. **Putting secrets in `ARG`, `ENV`, `.env`, image layers, logs, labels, or Compose files**.
5. **Using unpinned `latest` images** or untrusted base images.
6. **Publishing ports to all interfaces by accident**, for example `8080:8080` instead of `127.0.0.1:8080:8080`.
7. **Using host namespaces** such as `network_mode: host`, `pid: host`, `ipc: host`, or `userns_mode: host`.
8. **Bind-mounting sensitive host paths** like `/`, `/etc`, `/root`, `/home`, `/var/lib/docker`, `/proc`, `/sys`, `.ssh`, `.aws`, `.kube`, or `.docker`.
9. **Building giant images** by using no `.dockerignore`, no multi-stage build, and copying the whole repo too early.
10. **Installing build tools, shells, package managers, and debug utilities into production images**.
11. **No memory, CPU, PID, ulimit, log, or health constraints**.
12. **Disabling TLS, certificate verification, seccomp, AppArmor, SELinux, or vulnerability scanning**.
13. **Silencing lint/security findings instead of fixing the Dockerfile or Compose file**.

## Security & Secrets

1. /tmp:rw,noexec,nosuid,size=64m
2. /run:rw,nosuid,size=16m
3. ALL
4. no-new-privileges:true
5. "127.0.0.1:8080:8080"
6. frontend
7. backend
8. db_password
9. .env
10. No `USER` instruction.
11. Creating a user but switching back to root later.
12. Using `sudo` inside the container.
13. Granting passwordless sudo to the app user.
14. Running root because of file ownership mistakes.
15. Using UID `0` in Compose: `user: "0:0"`.
16. Mapping the container user to a sensitive host UID.
17. Using root to bind low ports instead of using a reverse proxy or capability-specific approach.
18. “Fixing” permissions with root instead of fixing ownership at build time.
19. Making the image depend on root-owned writable directories.
20. `ARG` and `ENV` values can persist in image metadata or build history.
21. Files copied in one layer can remain recoverable even if removed in a later layer.
22. Compose `environment:` values are not secrets.
23. Build logs can leak secret values.
24. CI caches can leak tokens.
25. Secret names themselves can reveal infrastructure details.
26. seccomp:unconfined
27. apparmor:unconfined
28. security.insecure
29. Removes defense-in-depth.
30. Increases kernel attack surface.
31. Often used to “make tests pass.”
32. Weakens local security posture.
33. Makes container breakout bugs more severe.
34. Can violate policy baselines.
35. Harder to reason about least privilege.
36. Dangerous when combined with root and bind mounts.
37. Values can appear in `docker inspect`.
38. Values may appear in logs or crash dumps.
39. Values are available to every process in the container.
40. Shell history can leak values.
41. Compose interpolation can surprise reviewers.
42. Environment precedence can override expected values.
43. Secrets may be inherited by child processes.
44. Debug endpoints often expose env.
45. Rotation is harder.
46. Attackers can modify app files after compromise.
47. Persistence inside container becomes easier.
48. Runtime writes hide packaging errors.
49. Containers become less ephemeral.
50. Debugging differs from immutable production patterns.
51. Malware can drop tools.
52. Log files may fill image layers.
53. Read-only violations are not caught early.
54. Makes incident response harder.
55. network.host
56. Entire host as build context.
57. Non-portable absolute paths.
58. Build has elevated privileges.
59. Build can access host network.
60. Cache disabled.
61. Secrets may enter context.
62. Reproducibility suffers.
63. CI behavior differs from local.
64. Reviewers miss build security hidden in Compose.
65. Build output may include host files.
66. npmrc
67. type=registry,ref=ghcr.io/acme/app:buildcache
68. **`ARG NPM_TOKEN`, `ARG AWS_SECRET_ACCESS_KEY`, `ARG GITHUB_TOKEN`, etc.** [S]
69. **`ENV PASSWORD=...`, `ENV API_KEY=...`, `ENV TOKEN=...`** [S]
70. **Writing secrets into files during one layer and deleting them in a later layer**. [S] The secret can remain in lower layers/history.
71. **Using SSH private keys directly in the build context**. [S]
72. **Cloning private repos by embedding credentials in URLs**. [S]
73. **Using `docker history`-visible commands that include secrets**. [S]
74. **Printing secrets in build logs**. [S]
75. **Letting SBOM/provenance metadata include sensitive repository or build information unnecessarily**. [S]
76. **Building untrusted pull requests with production secrets available**. [S]
77. **Using Compose `build.args` for secrets**. [S]
78. **Not rotating secrets after accidental image-layer exposure**. [S/Ops]
79. **`privileged: true`**. [S]
80. **Adding `SYS_ADMIN` as a generic fix**. [S]
81. **No `cap_drop` for services that can run with fewer capabilities**. [S]
82. **`security_opt: ["seccomp=unconfined"]`**. [S]
83. **No `user:` and image has no `USER`**. [S]
84. **`user: root` to fix file permissions**. [S]
85. **`group_add` to sensitive host groups without review**. [S]
86. **`use_api_socket: true` without understanding that it lets the container interact with the underlying container engine API socket**. [S] Compose documents this field as enabling container-engine API interactions.
87. **Privileged `post_start` hooks**. [S] Compose lifecycle hooks can run privileged commands.
88. **`stdin_open: true` / `tty: true` in production services without need**. [M/R/S]
89. **Putting passwords/API keys directly in `environment:`**. [S]
90. **Putting secrets in `env_file:`**. [S]
91. **Using Compose interpolation from a developer shell containing secrets, then logging resolved config**. [S]
92. **Defining top-level secrets and assuming that alone grants/denies correctly without reviewing service grants**. [S/M]
93. **Using secrets with world-readable defaults without checking runtime user and filesystem permissions**. [S]
94. **Logging environment variables on startup**. [S]
95. **Using `_FILE` conventions incorrectly, causing apps to print or ignore secrets**. [S/R]
96. **Using sample/default passwords in Compose examples copied into production**. [S]
97. **Hardcoding JWT secrets, encryption keys, database roots, or OAuth client secrets**. [S]
98. **No secret rotation path**. [S/Ops]
99. **Pin production base images by digest, but automate updates.** Digest pinning gives reproducibility; update bots/scanners keep it from becoming “vulnerable forever.”
100. **Run as non-root.** Use `USER` in the Dockerfile and avoid switching back to root after setup. OWASP and Docker both emphasize unprivileged users.
101. **Use `COPY --chown` and precise permissions.** Avoid `chmod -R 777`.
102. **Never put secrets in images.** Use BuildKit secrets for build-time access and runtime secret stores for runtime access.
103. **Avoid `curl | sh`.** Download artifacts over TLS, verify checksums/signatures, and pin versions.
104. **Keep the final image boring.** It should contain the app, runtime libraries, CA certificates if needed, and little else.
105. **Use exec-form `CMD`/`ENTRYPOINT`.** This improves signal behavior and avoids shell quoting surprises.
106. **Add a health check for services.** Keep it local, fast, and side-effect-free.
107. Replaces file-permission fixes with `USER root`.
108. Fixes permission errors with `chmod -R 777`.
109. Adds `privileged: true` to fix device, networking, or filesystem problems.
110. Adds `network_mode: host` because service discovery is misconfigured.
111. Mounts the entire project root into production.
112. Mounts `/`, `/etc`, `/var`, `/home`, `/root`, `/proc`, `/sys`, `/run`, or `/var/lib/docker`.
113. Mounts SSH agent sockets, cloud credential directories, kubeconfigs, or Docker credentials into general app containers.
114. Changes `security_opt` to `seccomp=unconfined` or `apparmor=unconfined`.
115. Adds `cap_add: ["SYS_ADMIN"]` as a universal fix.
116. Disables healthchecks instead of making them correct.
117. Disables TLS verification, package signature checks, or GPG verification.
118. Stores a copied token in `ENV` “temporarily” and never removes it.
119. `ARG TOKEN=...`
120. `ENV NPM_TOKEN=...`
121. `ENV DATABASE_URL=postgres://user:password@...`
122. `RUN echo "$SECRET" > file`
123. `RUN npm config set //registry.npmjs.org/:_authToken=$NPM_TOKEN`
124. Copying `.git` into the image, including secret history.
125. Logging tokens in build output.
126. Using `--build-arg` for secrets.
127. Assuming deleting a secret later removes it from prior layers.
128. Using build cache layers that already captured secret-derived files.
129. Using remote `ADD`/`curl` with credentialed URLs.
130. Using package manager auth files in a layer instead of BuildKit secret mounts.
131. Forgetting that `ENV` persists in the image and descendant stages. Docker documents that `ENV` values persist in built images.
132. Putting passwords in `environment`.
133. Putting API keys in `environment`.
134. Putting tokens in `env_file`.
135. Committing `secrets/*.txt`.
136. Storing production secrets next to `compose.yaml`.
137. Using the same secret for dev, staging, and prod.
138. Passing secrets as command-line arguments.
139. Passing secrets as labels.
140. Putting credentials in image names or registry URLs.
141. Logging environment variables during startup.
142. Using `env_file` with interpolation surprises.
143. Using `required: false` on important `env_file` entries and silently running with missing config. Compose supports optional env files; that is dangerous for required production config.
144. Not using `_FILE` variants when images support them.
145. Not using Compose secrets.
146. Using Compose secrets but storing the source files unencrypted in Git.
147. Assuming Compose secrets are a complete enterprise secret manager.
148. Reusing database root passwords in app services.
149. Building images with runtime secrets.
150. Using boolean-like unquoted env values and getting YAML coercion surprises.
151. Exposing the host Docker socket to untrusted CI jobs.
152. Running privileged Docker-in-Docker for pull requests from forks.
153. Letting generated Compose mount CI secrets into containers.
154. Building untrusted Dockerfiles with access to production registry credentials.
155. Pushing images from unreviewed branches.
156. Pushing `latest` from multiple branches.
157. Reusing the same tag for different code.
158. No immutable tag such as commit SHA.
159. No provenance metadata.
160. No SBOM.
161. No image signing.
162. No image scanning gate.
163. Scanning only the Dockerfile, not the final image.
164. Scanning only once at build time, never after new CVEs are published.
165. Printing `docker login` credentials.
166. Passing secrets as build args.
167. Running `docker compose up` in CI with host network or host mounts.
168. Running untrusted integration tests against shared Docker daemon.
169. No policy-as-code for forbidden Compose fields.
170. No approval step for privileged/device/socket changes.
171. No separation between CI builder identity and deployer identity.
172. No registry retention policy.
173. No cleanup, causing disk exhaustion.
174. Over-cleanup, destroying cache and slowing every build.
175. Running production deploys from mutable tags.
176. No secrets in Dockerfile.
177. No secrets in `ARG`.
178. No secrets in `ENV`.
179. No secrets in image labels.
180. No secrets in build logs.
181. Use BuildKit secret mounts for build-time access.
182. Use Compose secrets or a real secret manager for runtime.
183. Grant each service only the secrets it needs.
184. Secret source files should not be committed unencrypted.
185. Adds `USER root` to “fix permissions.”
186. Adds `chmod -R 777`.
187. Adds `privileged: true` to “fix Docker/permission/device issues.”
188. Disables seccomp/AppArmor/SELinux.
189. Uses `network_mode: host` to “fix networking.”
190. Publishes internal services to `0.0.0.0`.
191. Copies the whole repo into the image, including `.git` and local credential files.
192. Switches from a pinned image to `latest` for convenience.
193. Adds `apt-get upgrade` without explaining reproducibility impact.
194. Adds broad capabilities like `SYS_ADMIN`.
195. Removes healthchecks because they are “failing.”
196. Removes resource limits because the app OOMs.
197. Hides errors with `|| true`.
198. Uses `restart: always` to mask a crashing service.
199. Moves application code into a bind mount in production.
200. Generates one giant Dockerfile stage with build tools and runtime combined.
201. Generates Compose that only works on the agent’s host path.
202. Does not run or recommend `docker compose config` to inspect the final merged file.
203. Does not explain why a privileged exception is needed.
204. Does not include rollback or update strategy for digest pins.
205. Does not distinguish dev Compose from prod Compose.
206. jwt_private_key
207. Do not commit real secret files.
208. Grant secrets only to services that need them.
209. Prefer `_FILE` environment variables where images support them.
210. Do not put secrets in `environment`, `env_file`, labels, command args, image tags, or healthchecks.
211. Use `${VAR:?required}` for required non-secret config to fail fast.
212. Inspect final config with `docker compose config`, but remember that rendered config can expose non-secret values and sometimes environment-derived values.
213. **`ARG TOKEN`, `ARG PASSWORD`, `ARG AWS_SECRET_ACCESS_KEY`, `ARG NPM_TOKEN`.**
214. **`ENV TOKEN=...`, `ENV PASSWORD=...`, `ENV AWS_SECRET_ACCESS_KEY=...`.**
215. **Writing secrets into files during build and deleting them later.**
216. **Embedding private Git credentials in dependency URLs.**
217. **Using `RUN git clone https://token@...`.**
218. **Printing secrets in build logs.**
219. **Failing builds with commands that echo full environment variables.**
220. **Putting secrets in Docker labels.**
221. **Putting secrets in image tags.**
222. **Putting secrets in build cache exported to shared registries.**
223. **Using fake “redaction” that only removes secrets from final filesystem but not from history or logs.**
224. **Giving every build job access to production secrets.**
225. **Using the same secret for build-time package access and runtime production access.**
226. **Not rotating secrets after accidental image publication.**
227. **Not checking `docker history --no-trunc` or image metadata after suspected leaks.**
228. **Putting passwords/tokens in `environment:`.**
229. **Using `env_file:` for production secrets without access controls.**
230. **Using `required: false` on critical `env_file` entries, silently ignoring missing secrets.**
231. **Giving every service access to every secret.**
232. **Storing Compose secret source files inside the repo.**
233. **World-readable secret files on the host.**
234. **Using environment variables ending in `_FILE` incorrectly.**
235. **Using unquoted YAML booleans in `environment`.**
236. **Using empty environment variable declarations that unset critical values unexpectedly.**
237. **Putting secrets in labels, container names, image names, network aliases, commands, healthchecks, or logs.**
238. **Passing secrets as CLI args in `command:` because args are often visible in process listings.**
239. **No secret rotation procedure.**
240. [ ] Base image is trusted and maintained.
241. [ ] Base image is pinned to a deliberate version or digest.
242. [ ] Image is rebuilt regularly.
243. [ ] No secrets in `ARG`, `ENV`, files, labels, logs, or history.
244. [ ] Build secrets use BuildKit secret mounts.
245. [ ] Final image runs as non-root.
246. [ ] Final image does not contain compilers/build tools unless required.
247. [ ] Final image does not contain package-manager caches.
248. [ ] No `curl | sh` without checksum/signature.
249. [ ] No disabled TLS verification.
250. [ ] No world-writable application directories.
251. [ ] No unnecessary setuid/setgid binaries.
252. [ ] Entrypoint uses exec form or `exec "$@"`.
253. [ ] Healthcheck exists and does not leak secrets.
254. [ ] Final image has been scanned.
255. [ ] Image history has been checked for secret leakage.
256. [ ] No `privileged: true`.
257. [ ] No Docker socket mount.
258. [ ] No host namespace modes unless explicitly approved.
259. [ ] No host network unless explicitly approved.
260. [ ] No broad host path mounts.
261. [ ] No broad device mounts.
262. [ ] `user:` is non-root where possible.
263. [ ] Only required capabilities are added.
264. [ ] Root filesystem is read-only where possible.
265. [ ] Writable paths are explicit volumes or `tmpfs`.
266. [ ] Secrets use `secrets:`, not `environment:`.
267. [ ] Secrets are granted only to services that need them.
268. [ ] Ports are not published unnecessarily.
269. [ ] Local-only ports bind to `127.0.0.1`.
270. [ ] Debug/admin services are behind profiles and not public.
271. [ ] Images are pinned and scanned.

## Testing

1. docker:dind
2. CI behavior changes unexpectedly.
3. Builds fail when Docker changes defaults.
4. BuildKit behavior can change.
5. Compose plugin behavior can change.
6. Rollback becomes difficult.
7. Security review cannot tie behavior to version.
8. Agent debugging becomes inconsistent.
9. Reproducibility suffers.
10. CI cache compatibility can break.
11. “Fixed by pinning later” can be too late.
12. docker:26.1.4-dind

## Volumes & State

1. pgdata:/var/lib/postgresql/data
2. type: bind
3. .:/app
4. /:/host
5. ~/.ssh:/root/.ssh
6. /var/run/docker.sock:/var/run/docker.sock
7. Container can control Docker.
8. Container can start privileged sibling containers.
9. Container can mount host filesystems through Docker.
10. Read-only socket mounts are not a sufficient fix.
11. Any RCE in the app can become host compromise.
12. Breaks container isolation assumptions.
13. Dangerous in CI with untrusted code.
14. Dangerous in web dashboards.
15. Dangerous in local dev if browser-exposed app has RCE.
16. Creates root-equivalent access on many hosts.
17. Use a narrow service API instead of raw Docker.
18. Use rootless BuildKit or remote builders for builds.
19. Use Docker API over SSH/TLS only for trusted admin workflows.
20. Use a socket proxy with strict allowlists only when unavoidable.
21. Keep such containers isolated from untrusted input.
22. /etc:/etc
23. /proc:/proc
24. /sys:/sys
25. /dev:/dev
26. ~/.aws:/root/.aws
27. ./secrets:/secrets
28. Host filesystem exposure.
29. Credential theft.
30. Host config tampering.
31. Device access.
32. Kernel interface exposure.
33. Persistence from container compromise.
34. Secret files can be copied into logs or artifacts.
35. Host-specific paths break portability.
36. Root in container may map dangerously to host files.
37. Read-write mounts can become host compromise paths.
38. ./config/app.yml:/app/config/app.yml:ro
39. Read-only bind mounts where possible.
40. Named volumes for app-owned data.
41. Compose secrets for secrets.
42. Narrow file mounts instead of directory mounts.
43. No host credential directories.
44. Equivalent to host Docker control.
45. CI job can start sibling privileged containers.
46. Job can mount host root.
47. Untrusted build scripts become host-risk scripts.
48. Secrets from other builds can be exposed.
49. Runner cleanup mistakes become severe.
50. Container boundary is misleading.
51. Supply-chain attacks can compromise build hosts.
52. Debugging convenience becomes permanent infrastructure.
53. Least privilege is impossible.
54. **Mounting `/var/run/docker.sock`**. [S]
55. **Mounting host root or sensitive host paths**. [S]
56. **Short-syntax bind mounts with accidental read-write defaults**. [S]
57. **Omitting `:ro` / `read_only: true` for config mounts**. [S]
58. **Using relative bind mounts that resolve differently in CI**. [R]
59. **Using bind mounts for production application code**. [S/R]
60. **Mounting over `/app`, `/usr`, `/bin`, `/lib`, or other important image directories unintentionally**. [R]
61. **Using anonymous volumes that mask image content**. [R]
62. **No named top-level volume declaration for persistent state**. [M/R]
63. **No volume labels/backup policy**. [Ops]
64. **Sharing database volume with non-database containers**. [S/R]
65. **Mounting secrets as normal bind-mounted files to broad paths**. [S]
66. **Mounting the same writable config into multiple containers**. [R/S]
67. **Using host paths in a Compose file intended for remote/non-local deployment**. [R] Compose notes relative host paths are only supported for local runtimes.
68. Mounting `/var/run/docker.sock`.
69. Mounting `/`.
70. Mounting `/etc`.
71. Mounting `/root`.
72. Mounting `/home`.
73. Mounting `/var/lib/docker`.
74. Mounting `/proc`.
75. Mounting `/sys`.
76. Mounting `/run`.
77. Mounting cloud credential directories: `~/.aws`, `~/.gcloud`, `~/.azure`.
78. Mounting `~/.kube`.
79. Mounting SSH keys or SSH agent sockets.
80. Mounting browser profiles.
81. Mounting password-manager exports.
82. Mounting host source code read-write in production.
83. Mounting config files read-write when read-only is sufficient.
84. Using broad bind mounts instead of narrow file mounts.
85. Relying on relative host paths that resolve differently across machines.
86. Mounting secrets from a repository directory.
87. Mounting the same read-write volume into multiple services that are not designed for concurrent writes.
88. Storing databases in the container writable layer.
89. Using anonymous volumes accidentally, then losing track of data.
90. Running `docker compose down -v` in scripts that may be executed against valuable data.
91. No backup strategy for named volumes.
92. No restore test for named volumes.
93. No ownership plan for volumes when running non-root.
94. Using root-owned data directories to justify running the service as root.
95. Mounting host timezone, certificates, or Docker config files without validating the trust implications.
96. Using `volumes_from` to inherit more mounts than intended.
97. Mounting build caches into runtime containers.

## References

[1] https://docs.docker.com/build/building/best-practices/ "Building best practices | Docker Docs"
[2] https://docs.docker.com/build/cache/optimize/ "Optimize cache usage in builds | Docker Docs"
[3] https://github.com/hadolint/hadolint/wiki/DL3006 "DL3006 · hadolint/hadolint Wiki · GitHub"
[4] https://cheatsheetseries.owasp.org/cheatsheets/NodeJS_Docker_Cheat_Sheet.html "NodeJS Docker - OWASP Cheat Sheet Series"
[5] https://docs.docker.com/reference/build-checks/secrets-used-in-arg-or-env/ "SecretsUsedInArgOrEnv | Docker Docs"
[6] https://docs.docker.com/engine/security/ "Docker Engine security | Docker Docs"
[7] https://github.com/hadolint/hadolint/wiki/DL3025 "DL3025 · hadolint/hadolint Wiki · GitHub"
[8] https://csrc.nist.gov/pubs/sp/800/190/final "SP 800-190, Application Container Security Guide | CSRC"
[9] https://docs.docker.com/reference/compose-file/services/ "Define services in Docker Compose | Docker Docs"
[10] https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html?utm_source=chatgpt.com "Docker Security Cheat Sheet"
[11] https://gitlab.com/gitlab-org/gitlab-runner/-/issues/4660?utm_source=chatgpt.com "[Docker runner] permission denied while trying to connect ..."
[12] https://docs.docker.com/compose/how-tos/environment-variables/best-practices/ "Best practices for working with environment variables in Docker Compose | Docker Docs"
[13] https://www.reddit.com/r/docker/comments/1oozdyo/how_are_docker_secrets_more_secure_than_env_files/?utm_source=chatgpt.com "How are docker secrets more secure than .env files?"
[14] https://docs.docker.com/engine/containers/resource_constraints/ "Resource constraints | Docker Docs"
[15] https://docs.docker.com/engine/install/linux-postinstall/ "Linux post-installation steps for Docker Engine | Docker Docs"
[16] https://docs.gitlab.com/ci/docker/using_docker_build/?utm_source=chatgpt.com "Use Docker to build Docker images | GitLab Docs"
[17] https://github.com/hadolint/hadolint "GitHub - hadolint/hadolint: Dockerfile linter, validate inline bash, written in Haskell · GitHub"
[18] https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html "Docker Security - OWASP Cheat Sheet Series"
[19] https://docs.gitlab.com/runner/executors/docker/ "Docker executor | GitLab Docs"
[20] https://docs.docker.com/build/building/secrets/?utm_source=chatgpt.com "Build secrets"
[21] https://arxiv.org/abs/2307.03958 "[2307.03958] Secrets Revealed in Container Images: An Internet-wide Study on Occurrence and Impact"
[22] https://docs.gitlab.com/ci/docker/using_docker_build/ "Use Docker to build Docker images | GitLab Docs"
[23] https://x.com/brankopetric00/status/2016261683174637916?utm_source=chatgpt.com "No .dockerignore file - Missing health checks"
[24] https://docs.docker.com/build/cache/optimize/?utm_source=chatgpt.com "Optimize cache usage in builds"
[25] https://docs.docker.com/reference/compose-file/build/ "Compose Build Specification | Docker Docs"
[26] https://docs.docker.com/engine/security/protect-access/ "Protect the Docker daemon socket | Docker Docs"
[27] https://docs.docker.com/compose/how-tos/use-secrets/ "Manage secrets securely in Docker Compose | Docker Docs"
[28] https://support.atlassian.com/bitbucket-cloud/docs/run-docker-commands-in-bitbucket-pipelines/ "Run Docker commands in Bitbucket Pipelines | Bitbucket Cloud | Atlassian Support"
[29] https://github.com/tiangolo/docker-with-compose/issues/32 "Bitbucket Pipelines: Error response from daemon: authorization denied by plugin pipelines: --privileged=true is not allowed · Issue #32 · tiangolo/docker-with-compose · GitHub"
[30] https://dl.acm.org/doi/10.1145/3696410.3714653?utm_source=chatgpt.com "A Large-Scale Security Measurement of Docker Image ..."
[31] https://docs.sigstore.dev/cosign/verifying/verify/?utm_source=chatgpt.com "Verifying Signatures - Cosign"
[32] https://docs.docker.com/security/security-announcements/ "Docker security announcements | Docker Docs"
[33] https://github.com/docker/docker-bench-security?utm_source=chatgpt.com "Docker Bench for Security"
[34] https://csrc.nist.gov/pubs/sp/800/190/final?utm_source=chatgpt.com "SP 800-190, Application Container Security Guide - NIST CSRC"
[35] https://docs.docker.com/reference/cli/docker/container/run/ "docker container run | Docker Docs"
[36] https://docs.docker.com/build/building/secrets/ "Build secrets | Docker Docs"
[37] https://docs.docker.com/compose/trust-model/ "Trust model for Compose files | Docker Docs"
[38] https://www.reddit.com/r/docker/?utm_source=chatgpt.com "r/docker"
[39] https://docs.gitlab.com/runner/security/ "Security for self-managed runners | GitLab Docs"
[40] https://arxiv.org/abs/2601.12811?utm_source=chatgpt.com "Docker Does Not Guarantee Reproducibility"
[41] https://docs.docker.com/engine/storage/bind-mounts/ "Bind mounts | Docker Docs"
[42] https://docs.docker.com/ai/sandboxes/security/workspace/ "Workspace trust | Docker Docs"
[43] https://docs.docker.com/ai/sandboxes/ "Docker Sandboxes | Docker Docs"
[44] https://docs.docker.com/ai/docker-agent/best-practices/ "Best practices | Docker Docs"
[45] https://docs.docker.com/build/concepts/context/ "Build context | Docker Docs"
[46] https://docs.docker.com/build/building/multi-stage/ "Multi-stage builds | Docker Docs"
[47] https://docs.docker.com/engine/containers/run/ "Running containers | Docker Docs"
[48] https://docs.kics.io/latest/queries/dockercompose-queries/ "Docker Compose - KICS"
[49] https://docs.docker.com/ai/sandboxes/security/isolation/ "Isolation layers | Docker Docs"
[50] https://docs.docker.com/engine/network/port-publishing/ "Port publishing and mapping | Docker Docs"
[51] https://docs.docker.com/reference/compose-file/ "Compose file reference | Docker Docs"
[52] https://docs.docker.com/build/cache/invalidation/ "Build cache invalidation | Docker Docs"
[53] https://docs.docker.com/build/buildkit/ "BuildKit | Docker Docs"
[54] https://www.reddit.com/r/docker/comments/1o8rykq/docker_size_is_too_big/?utm_source=chatgpt.com "Docker size is too big"
[55] https://arxiv.org/abs/2312.13888?utm_source=chatgpt.com "Empirical Study of the Docker Smells Impact on the Image Size"
[56] https://docs.docker.com/compose/how-tos/networking/ "Networking in Compose | Docker Docs"
[57] https://docs.gitlab.com/ci/docker/docker_layer_caching/?utm_source=chatgpt.com "Cache Docker layers in Docker-in-Docker builds"
[58] https://www.reddit.com/r/docker/comments/1pri6hi/dockersock_security_concerns_in_2025/?utm_source=chatgpt.com "docker.sock: Security concerns in 2025"
[59] https://github.com/hadolint/hadolint?utm_source=chatgpt.com "Hadolint - Haskell Dockerfile Linter"
[60] https://www.cisecurity.org/benchmark/docker?utm_source=chatgpt.com "CIS Docker Benchmarks"
[61] https://gitlab.com/gitlab-org/gitlab-foss/-/issues/17861 "Caching for docker-in-docker builds (#17861) · Issues · GitLab.org / GitLab FOSS · GitLab"
[62] https://docs.docker.com/reference/dockerfile/ "Dockerfile reference | Docker Docs"
[63] https://www.reddit.com/r/programming/comments/bcumt6/secure_secrets_in_docker_builds/ "Secure Secrets in Docker Builds : r/programming"
[64] https://docs.docker.com/compose/how-tos/environment-variables/set-environment-variables/ "Set environment variables within your container's environment | Docker Docs"
[65] https://docs.docker.com/scout/ "Docker Scout | Docker Docs"
[66] https://docs.sigstore.dev/about/overview/ "Overview - Sigstore"
[67] https://docs.docker.com/scout/policy/ "Get started with Policy Evaluation in Docker Scout | Docker Docs"
[68] https://nvlpubs.nist.gov/nistpubs/specialpublications/nist.sp.800-190.pdf "Application Container Security Guide"
[69] https://docs.docker.com/compose/how-tos/networking/?utm_source=chatgpt.com "Networking in Compose"
[70] https://docs.docker.com/reference/compose-file/networks/ "Define and manage networks in Docker Compose | Docker Docs"
[71] https://x.com/secboyuk?utm_source=chatgpt.com "Ste Watts (SecBoyUK)"
[72] https://docs.docker.com/compose/how-tos/environment-variables/variable-interpolation/ "Set, use, and manage variables in a Compose file with interpolation | Docker Docs"
[73] https://docs.docker.com/compose/how-tos/environment-variables/envvars-precedence/ "Environment variables precedence in Docker Compose | Docker Docs"
[74] https://www.reddit.com/r/docker/comments/1kduvl3/how_do_you_architecturally_handle_secrets_defined/ "How do you architecturally handle secrets defined in .env when you have a lot of optional services? : r/docker"
[75] https://docs.docker.com/compose/how-tos/startup-order/ "Control startup and shutdown order in Compose | Docker Docs"
[76] https://docs.docker.com/engine/security/rootless/ "Rootless mode | Docker Docs"
[77] https://hub.docker.com/r/docker/docker-bench-security "docker/docker-bench-security - Docker Image"
[78] https://docs.docker.com/dhi/core-concepts/cis/ "CIS Benchmark | Docker Docs"
[79] https://docs.docker.com/build/cache/backends/ "Cache storage backends | Docker Docs"
[80] https://www.reddit.com/r/docker/comments/gdfys3/six_things_to_keep_in_mind_when_working_with/?utm_source=chatgpt.com "Six things to keep in mind when working with Dockerfiles"
[81] https://docs.docker.com/build/concepts/dockerfile/?utm_source=chatgpt.com "Dockerfile overview"
[82] https://gitlab.com/gitlab-org/gitlab-foss/-/issues/17769 "Consider binding to Docker socket rather than using Docker-in-Docker (#17769) · Issues · GitLab.org / GitLab FOSS · GitLab"
[83] https://forums.docker.com/t/docker-compose-mount-secret-file-content-as-environment-variable-content/143646?utm_source=chatgpt.com "Mount secret file content as environment variable content"
[84] https://www.reddit.com/r/Python/comments/13plqqj/managing_gitlab_secrets_in_requirements_for_docker/ "Managing gitlab secrets in requirements for docker : r/Python"
