# Bad CI Behavior: Comprehensive Guide

This document organizes the worst CI behaviors that are inexcusable in production.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core CI best practices:

- **Treat CI as source of truth**: Local 'works on my machine' is irrelevant; the CI pipeline is the definitive test.
- **Keep pipelines fast**: Parallelize independent jobs, utilize caching aggressively, and fail fast.
- **Run comprehensive checks**: Include linting, formatting, type-checking, unit tests, and security scanning.
- **Protect the main branch**: Require passing CI checks and approvals before any merge.
- **Use ephemeral and isolated environments**: Build artifacts once and promote them through environments.
- **Pin pipeline dependencies**: Explicitly version CI/CD actions and runners to prevent sudden breakages.

## A. Bad CI runner/agent behavior

1. Executes code from an untrusted branch with privileged credentials.
2. Executes untrusted code on a persistent host.
3. Reuses workspace state across trust boundaries.
4. Leaves secrets in files after job completion.
5. Leaves containers, images, volumes, or processes running after jobs.
6. Allows one job to read another job’s workspace.
7. Allows one repository to read another repository’s checkout.
8. Runs as root unnecessarily.
9. Mounts host paths unnecessarily.
10. Has access to Docker socket.
11. Has access to Kubernetes cluster credentials.
12. Has access to production network segments.
13. Has access to internal package registries with write permissions.
14. Has access to cloud metadata credentials.
15. Uses broad service-account credentials.
16. Does not enforce timeout/resource limits.
17. Does not enforce network egress restrictions.
18. Does not attest runner image or configuration.
19. Does not log job-to-runner assignment.
20. Does not clean caches safely.
21. Does not separate protected and unprotected jobs.
22. Does not separate public and private repositories.
23. Does not separate build, test, publish, and deploy trust zones.
24. Accepts arbitrary labels from jobs.
25. Accepts arbitrary scripts from comments or issue events.
26. Auto-registers runners without approval.
27. Runs obsolete runner versions.
28. Runs unpatched host OS images.
29. Stores SSH keys on disk.
30. Stores cloud keys on disk.
31. Shares credentials between runners.
32. Can be reached inbound from the internet.
33. Can initiate arbitrary outbound connections.
34. Can access secrets manager without job identity constraints.
35. Can deploy without human approval.
36. Fails open when a security scanner fails.
37. Retries suspicious jobs automatically without quarantine.
38. Provides no forensic evidence after compromise.
39. Cannot be rapidly reimaged.
40. Cannot revoke its own credentials.

## A. Bad identity, permissions, and privilege behavior

1. **Running all CI jobs with write permissions by default.**
2. **Sharing one powerful token across unrelated steps.**
3. **Allowing CI agents to approve, merge, or create privileged PRs.**
4. **Using long-lived personal access tokens instead of short-lived scoped credentials.**
5. **Using personal SSH keys in CI.**
6. **Giving untrusted contributors access to protected variables or protected runners.**
7. **Disabling token allowlists or broadening job-token access.**
8. **Letting CI tokens push code without strong controls.**
9. **Using global credentials instead of scoped credentials.**
10. **Allowing anyone with write access to change CI behavior without review.**

## A. Bad pipeline structure

1. **Long serial pipelines with no DAG.**
2. **No parallelization for independent tests.**
3. **Over-parallelization that starves runners.**
4. **No path filters.**
5. **No change-based test selection.**
6. **No smoke tests before expensive tests.**
7. **Running slow integration/e2e tests before fast unit/lint checks.**
8. **No fail-fast behavior.**
9. **Failing late because quality gates are placed after expensive jobs.**
10. **Rebuilding the same artifact in every job.**
11. **Downloading dependencies separately in every job.**
12. **No dependency cache, or a cache key that never hits.**
13. **Cache keys that are too broad and restore stale/incompatible dependencies.**
14. **Cache keys that are too narrow and create a new cache every run.**
15. **Caching huge directories with little reuse value.**
16. **Uploading/download large artifacts unnecessarily.**
17. **Using artifacts where caches should be used, or caches where artifacts should be used.**
18. **No timeout on jobs or steps.**
19. **Timeouts set so low that normal work flakes.**
20. **No cancellation of superseded runs.**
21. **No interruptible/cancel behavior for obsolete branches.**
22. **Running full CI on every push to a branch while a PR is already open.**
23. **Duplicated pipelines for push and PR/MR events.**
24. **Scheduled jobs running during peak developer hours.**
25. **Cron storms.**

## A. Start with a trust model

1. **Untrusted validation**
2. **Trusted build**
3. **Release/publish**
4. **Deploy**
5. **Administrative automation**

## A. Trigger and pipeline-explosion problems

1. Running full CI on every push, every PR, every tag, every schedule, and every file change.
2. Running the same pipeline twice for both branch push and merge request/pull request.
3. Running expensive integration or end-to-end suites for documentation-only changes.
4. Running deployment preparation on every branch.
5. Creating broad schedules that overlap with normal push pipelines.
6. Triggering downstream pipelines recursively.
7. Creating child pipelines from every job without guardrails.
8. Running all language/runtime/platform combinations on every change.
9. Running large matrix jobs without risk-based pruning.
10. No concurrency groups or auto-cancel for superseded commits.
11. No `interruptible`/cancel-safe marking for non-deployment jobs.
12. Retrying entire pipelines instead of failed jobs.
13. Retrying flaky jobs indefinitely.
14. Allowing bot commits to trigger bot commits to trigger more pipelines.
15. Letting an AI agent repeatedly push tiny commits that each trigger full CI.
16. No maximum runtime per job.
17. No maximum number of parallel jobs per branch/project.
18. No queue/backpressure controls.
19. No separate lightweight pre-merge and heavier post-merge pipelines.
20. No skip logic for generated files, docs, comments, or non-runtime assets.

## A. Trust-boundary violations

1. Running **untrusted pull request code** with production secrets available.
2. Running untrusted PR code with a write-capable repository token.
3. Running forked PRs on self-hosted runners that contain long-lived credentials.
4. Treating branch names, PR titles, commit messages, issue comments, release names, tags, and artifact names as trusted shell input.
5. Using GitHub `pull_request_target` and then checking out or executing code from the untrusted PR head.
6. Triggering privileged jobs from comments such as `/test`, `/deploy`, `/approve`, or `/retest` without robust authorization and immutable commit binding.
7. Approving a PR once, then allowing the author to push new commits that run under the earlier approval.
8. Running old PRs against old, vulnerable workflow definitions after a workflow security fix.
9. Running build/test scripts supplied by the PR before reviewing changes to package scripts, Makefiles, Gradle/Maven plugins, shell scripts, test harnesses, or CI YAML.
10. Allowing generated pipeline definitions from an untrusted branch to create child pipelines or dynamic jobs.
11. Treating test output, coverage output, linter output, or AI-agent feedback as trusted instructions.
12. Letting an AI/automation agent read malicious PR content and then follow embedded instructions such as “print environment variables,” “disable security checks,” or “upload logs.”
13. Running `npm install`, `pip install`, `bundle install`, `go generate`, `mvn`, Gradle plugins, or similar commands with secrets present when package lifecycle scripts may execute arbitrary code.
14. Using privileged deploy workflows to process artifacts produced by an unprivileged PR workflow without validation.
15. Letting untrusted code write cache entries consumed later by privileged workflows.
16. Allowing the CI agent to fetch and run scripts from arbitrary URLs.
17. Allowing branch or tag names to select deployment targets.
18. Allowing a build matrix or job definition to be generated from untrusted user input.
19. Treating “tests passed” as proof that the CI YAML itself is safe.
20. Allowing pipeline configuration changes to merge without special review.

## AI & Vibe-Coding

1. Running all jobs with admin-equivalent repository permissions.
2. Using `write-all` or broad default permissions in GitHub Actions.
3. Giving build jobs permission to push code, create releases, publish packages, or modify workflows.
4. Giving test jobs permission to deploy.
5. Giving deployment jobs permission to change source code.
6. Allowing CI bots to approve their own pull requests.
7. Allowing CI bots to bypass branch protection.
8. Allowing the same token to modify code, modify CI configuration, publish artifacts, and deploy to production.
9. Using stale bot accounts with unknown ownership.
10. Leaving old deploy keys, webhooks, GitHub Apps, OAuth apps, GitLab tokens, Bitbucket app passwords, or runner tokens active.
11. Assigning workspace/group/org-level permissions when repo/project-level permissions would suffice.
12. Allowing all maintainers to edit production deployment configuration.
13. Granting all repositories access to the same production runner pool.
14. Granting broad cloud IAM roles to CI without conditions on repository, branch, environment, or workflow.
15. Trusting tag names without protecting tag creation.
16. Trusting branch names without branch protections.
17. Allowing external contributors to trigger privileged workflows indirectly.
18. Not expiring temporary access for contractors, vendors, or test accounts.
19. Not separating human identity, CI identity, deployment identity, and artifact-publishing identity.
20. Not maintaining an inventory of all identities used by CI/CD.
21. Using unpinned GitHub Actions such as `@main`, `@master`, or mutable tags.
22. Using unpinned GitLab includes from another project.
23. Using unpinned Bitbucket pipes.
24. Using Docker images tagged `latest`.
25. Using base images without digest pinning.
26. Using package ranges that allow unexpected updates in CI.
27. Running package manager install commands without lockfiles.
28. Ignoring package-lock, pnpm-lock, yarn.lock, poetry.lock, Pipfile.lock, Gemfile.lock, go.sum, Cargo.lock, or equivalent.
29. Installing tools from random URLs.
30. Using `curl | bash` without pinning, signature verification, and hash verification.
31. Trusting transitive dependencies without scanning.
32. Allowing dependency lifecycle scripts to run with secrets present.
33. Allowing dependency scripts to run in deploy jobs.
34. Not separating dependency-fetch jobs from secret-bearing jobs.
35. Pulling CI tools from personal accounts.
36. Pulling third-party actions from abandoned repositories.
37. Pulling third-party actions by tag when the tag can be moved.
38. Not reviewing action source before use.
39. Letting third-party integrations have org-wide admin access.
40. Leaving stale OAuth apps, GitHub Apps, GitLab integrations, Bitbucket app passwords, and webhooks installed.
41. Trusting remote CI templates maintained by another team without change control.
42. Not using dependency review, SCA, SBOM, or vulnerability alerting.
43. Not detecting dependency confusion, typosquatting, or malicious package replacement.
44. Publishing packages from CI without provenance.
45. Publishing packages from unprotected branches or tags.
46. Deploying artifacts that were not produced by the reviewed commit.
47. Rebuilding during deployment instead of promoting the exact artifact built and tested earlier.
48. Deploying mutable image tags such as `latest`.
49. Deploying artifacts without checksums, signatures, provenance, or attestations.
50. Consuming artifacts from untrusted PR jobs inside privileged jobs.
51. Uploading executable artifacts from untrusted jobs and then executing them in trusted jobs.
52. Letting artifacts be overwritten or replaced.
53. Not verifying artifact digest before deploy.
54. Not recording builder identity, source commit, dependency set, or build parameters.
55. Publishing container images without digest promotion.
56. Allowing artifact repositories to accept unsigned or unauthenticated uploads.
57. Using artifact names derived from untrusted input.
58. Downloading all prior-stage artifacts by default even when a job needs only one file.
59. Retaining sensitive artifacts indefinitely.
60. Making artifacts publicly downloadable.
61. Uploading logs, test reports, memory dumps, screenshots, or coverage reports that contain secrets.
62. Not separating build artifacts from test artifacts.
63. Not scanning release artifacts.
64. Not preserving enough metadata to investigate an incident.
65. No branch protection.
66. No required reviews.
67. No required passing checks.
68. No CODEOWNERS for CI files.
69. No special approval for pipeline-file changes.
70. No deployment environment protection.
71. No protected branches or protected tags.
72. No protected variables for production secrets.
73. No protected runners for production jobs.
74. Letting feature branches deploy to production.
75. Letting unreviewed tags trigger releases.
76. Letting scheduled workflows deploy without review.
77. Letting a single compromised developer account change code, change CI, publish artifacts, and deploy.
78. Letting automation merge code that changes its own permissions.
79. Letting bots create and approve PRs.
80. Letting “manual” jobs be run by users who should not deploy.
81. Failing open when required security scans fail.
82. Marking security jobs `continue-on-error`, `allow_failure`, or `|| true`.
83. Using broad path filters that accidentally skip security-relevant changes.
84. Not locking down who can modify deployment targets.
85. Not separating build, release, and deploy responsibilities.
86. Not reviewing remote CI template changes.
87. Not requiring human approval when new secrets, runners, or third-party actions are introduced.
88. Letting force-push rewrite reviewed code before merge.
89. Letting old approvals remain valid after CI files change.
90. Not collecting CI audit logs.
91. Not alerting on workflow-file changes.
92. Not alerting on variable/secret changes.
93. Not alerting on runner registration.
94. Not alerting on new third-party integrations.
95. Not alerting on permission changes for CI bots.
96. Not logging artifact publication and download events.
97. Not logging deployment approvals.
98. Not monitoring unusual job duration, CPU, network, or outbound connections.
99. Not retaining logs long enough for incident response.
100. Retaining logs too publicly or too long when they may contain sensitive data.
101. Not correlating CI activity with source-control, cloud, registry, and deployment logs.
102. Not tracking cache hits/misses, artifact sizes, and queue time.
103. Not tracking flaky tests separately from real failures.
104. Not detecting suspicious commands such as `printenv`, `env`, `curl $SECRET`, `nc`, crypto-mining tools, credential scraping, or exfiltration attempts.
105. Not scanning CI logs/artifacts for secrets.
106. Not having an incident runbook for CI compromise.
107. Not having an inventory of runners, tokens, variables, webhooks, actions, includes, and third-party services.
108. Not preserving provenance for releases.
109. Not knowing which pipeline built a production artifact.
110. Pin GitHub Actions to full-length commit SHAs.
111. Pin GitLab `include:project` to full SHAs.
112. Pin Docker images by digest for sensitive jobs.
113. Avoid `latest`.
114. Avoid branch refs for CI templates.
115. Maintain an allowlist of approved actions/pipes/includes.
116. Review source code of third-party actions before use.
117. Prefer verified/official actions but still pin them.
118. Avoid abandoned actions.
119. Use Dependabot or equivalent for action/dependency updates.
120. Use lockfiles and reproducible installs.
121. Use `npm ci` rather than unconstrained install behavior where applicable.
122. Disable dependency lifecycle scripts in secret-bearing jobs where possible.
123. Run dependency installation before secrets are introduced.
124. Use private registry protections against dependency confusion.
125. Scan dependencies.
126. Generate SBOMs.
127. Monitor advisories.
128. Remove unused integrations.
129. Scope third-party app permissions narrowly.
130. Review webhooks and app tokens regularly.
131. **Using unpinned third-party Actions, includes, pipes, plugins, or Docker images.**
132. **Trusting third-party CI components with secrets.**
133. **Installing dependencies without lockfiles.**
134. **Running package-manager lifecycle scripts from untrusted branches.**
135. **Using dependency caches as trust anchors.**
136. **Ignoring vulnerable Actions/plugins.**
137. **No dependency review or SBOM.**
138. **Building release artifacts from untrusted or non-reviewed code paths.**
139. **Using public package registries directly in release jobs without pinning, lockfiles, or integrity checks.**
140. **Using remote CI templates from mutable branches.**
141. **Not scanning CI configuration itself.**
142. **Default AI agents to read-only repository permissions.**
143. **Do not give agents production secrets.**
144. **Do not let agents modify CI workflows without human review.**
145. **Do not let agents approve or merge their own PRs.**
146. **Block agents from weakening required checks.**
147. **Require agents to explain CI config changes in PR descriptions.**
148. **Treat issue/PR text as untrusted input to the agent.**
149. **Never let agent instructions from repo text override security policy.**
150. **Redact secrets from logs before agent summarization.**
151. **Do not let agents upload full raw logs to public comments.**
152. **Give agents resource quotas and command allowlists where possible.**
153. **Require deterministic tests and local reproduction commands.**
154. **Require security review for agent-added dependencies, Actions, Pipes, plugins, or Docker images.**
155. **Record agent commands and tool calls as audit data.**
156. **Prevent agents from editing release/signing/deploy workflows without protected review.**
157. Adds `permissions: write-all` to “fix” permission errors.
158. Adds broad secrets to top-level `env`.
159. Switches from `pull_request` to `pull_request_target` to get access to secrets.
160. Checks out PR code inside privileged workflows.
161. Adds `continue-on-error: true` to make CI green.
162. Adds `|| true` after failing tests.
163. Deletes failing tests.
164. Converts assertions into smoke checks.
165. Marks flaky tests as skipped without owner/expiry.
166. Broadens branch triggers until deploys run from unsafe branches.
167. Adds production deploys to test workflows.
168. Adds static cloud credentials instead of OIDC.
169. Stores secrets in YAML.
170. Uses `curl | bash` from a vendor install page.
171. Uses unpinned actions/components/images.
172. Uses `@main`, `@master`, `@latest`, or mutable image tags.
173. Uses abandoned marketplace actions.
174. Adds third-party actions to jobs with secrets without review.
175. Ignores official platform security guidance.
176. Generates enormous matrices with no cost control.
177. Runs full E2E on every documentation change.
178. Removes caches rather than fixing cache keys.
179. Adds overbroad caches including home directories.
180. Uploads entire workspaces as artifacts.
181. Uploads logs that include env vars.
182. Adds self-hosted runners for PR jobs to access internal resources.
183. Adds Docker socket mounting to make Docker builds work.
184. Adds privileged Docker-in-Docker without isolation.
185. Uses production databases for tests.
186. Hardcodes staging/prod URLs and credentials.
187. Uses untrusted workflow inputs as shell commands.
188. Uses PR titles or branch names directly in shell.
189. Generates self-modifying workflows that commit back to the branch.
190. Creates CI loops where a bot pushes changes that retrigger itself indefinitely.
191. Adds scheduled workflows that run expensive full scans too frequently.
192. Silences security scanners to avoid false positives.
193. Makes security gates advisory without documenting risk acceptance.
194. Fails to add CODEOWNERS for CI changes.
195. Fails to segment trusted/untrusted workflows.
196. Fails to document why privileges are needed.
197. Fails to produce reproducible builds.
198. Fails to pin toolchain versions.
199. Fails to use lockfiles.
200. Fails to collect test reports.
201. Fails to define timeout and concurrency.
202. Fails to ensure deployment artifacts are the same artifacts that were tested.
203. Over-collects repository, user, issue, or secret context into prompts/logs.
204. Copies community YAML snippets without threat modeling.
205. Treats “green CI” as the only objective.
206. Optimizes for speed by removing security, determinism, or isolation.
207. Pin GitHub Actions to full commit SHA.
208. Pin GitLab components to SHA or release tag.
209. Pin Docker images by digest or immutable version.
210. Avoid `@main` and `@master`.
211. Avoid `curl | bash`.
212. Verify checksums/signatures for downloaded tools.
213. Review third-party action/component/pipe source before use.
214. Maintain an inventory of CI dependencies.
215. Use Dependabot/Renovate to update pinned references through reviewable PRs.
216. Require CODEOWNERS review for CI dependency changes.
217. Prefer official/vendor-maintained actions where possible.
218. Minimize third-party actions in jobs with secrets.
219. Use SLSA-style provenance/attestations for build outputs.
220. Generate SBOMs for release artifacts.
221. Sign release artifacts and container images.
222. Verify artifacts before deployment.
223. Promote the same artifact across environments instead of rebuilding.
224. Untrusted PR code runs with secrets.
225. `pull_request_target` runs PR code.
226. Self-hosted internal runner executes public PR code.
227. CI YAML contains hardcoded credentials.
228. Test job has production deploy credentials.
229. Third-party action with secrets is unpinned.
230. Docker socket or privileged runner is exposed to untrusted jobs.
231. Deployment from unprotected branches/tags.
232. Release artifacts are created from unreviewed code.
233. CI logs print secrets.
234. Missing explicit token permissions.
235. Overbroad cloud role trust.
236. Global secrets in `env`.
237. Mutable image/action/component refs.
238. Cache shared across trust boundaries.
239. Artifact from untrusted job executed by trusted job.
240. `allow_failure` or `continue-on-error` on required tests.
241. No timeout on expensive jobs.
242. No CODEOWNERS review for CI changes.
243. No protected variables/runners/environments.

## B. Bad job graph design

1. Everything is placed in one giant job.
2. Everything is placed in one giant stage.
3. Fast checks wait behind slow checks.
4. Lint/unit tests wait behind image builds.
5. Integration tests wait for unrelated jobs.
6. Deployment jobs rebuild artifacts instead of depending on build outputs.
7. Jobs use implicit dependencies rather than explicit `needs`.
8. A job downloads all prior artifacts by default.
9. Fan-in jobs gather huge artifacts from every parallel job.
10. Fan-out jobs each repeat identical setup.
11. Database migrations run before basic compile or unit tests.
12. Security scans run after deployment packaging rather than early enough to fail fast.
13. Every job installs the same dependencies from scratch.
14. Every job rebuilds the same Docker image.
15. Every job pulls large base images.
16. No separation between fast smoke tests and slow full suites.
17. No test sharding.
18. No fail-fast behavior for matrix jobs.
19. No dependency graph visibility.
20. Hidden dependencies encoded in global `before_script`.

## B. Bad runner/resource behavior

1. **Using undersized runners for heavy builds.**
2. **Using oversized runners for small lint/test jobs.**
3. **Not separating CPU-bound, memory-bound, IO-bound, and network-bound jobs.**
4. **No memory limits for service containers.**
5. **Running Docker builds and test suites on the same small runner.**
6. **No disk cleanup.**
7. **Persistent runners accumulating stale state.**
8. **Cold-starting large images repeatedly.**
9. **Using bloated CI images.**
10. **Installing system packages every run instead of using maintained base images.**
11. **Pulling large Docker images without layer caching.**
12. **Using `latest` images that break unpredictably.**
13. **No runner autoscaling or poor autoscaling.**
14. **Using one shared runner pool for all priorities.**
15. **Allowing low-priority jobs to block release/hotfix jobs.**
16. **No concurrency limits for expensive jobs.**
17. **No resource locks for shared environments.**

## B. Bad trigger behavior and poisoned pipeline execution

1. **Running privileged workflows on untrusted pull requests.**
2. **Confusing `pull_request` and `pull_request_target`.**
3. **Running `workflow_run` on artifacts produced by untrusted code.**
4. **Trusting PR titles, branch names, issue bodies, labels, comments, commit messages, or usernames inside shell scripts.**
5. **Letting comments trigger privileged commands.**
6. **Using broad event triggers.**
7. **Using scheduled workflows that run mutable external code.**
8. **Allowing `workflow_dispatch` inputs to become shell commands.**
9. **Using labels as authorization.**
10. **Trusting fork branch names.**
11. **Triggering deployment or release jobs from test events.**

## B. GitHub Actions-specific bad behavior

1. Missing top-level `permissions`.
2. `permissions: write-all`.
3. Giving `contents: write` to test jobs.
4. Giving `pull-requests: write` to jobs that process untrusted PR content.
5. Giving `id-token: write` to jobs that do not need cloud federation.
6. `pull_request_target` plus checkout of PR head.
7. `pull_request_target` plus dependency install from PR.
8. `pull_request_target` plus test execution from PR.
9. `issue_comment` workflow that checks out and runs code from a PR.
10. `workflow_run` workflow that trusts artifacts from an unprivileged workflow.
11. Unsafe interpolation of `${{ github.event.pull_request.title }}`, `${{ github.head_ref }}`, `${{ github.event.comment.body }}`, or similar values into `run`.
12. Actions referenced as `owner/action@main`.
13. Actions referenced only by mutable major tag.
14. Third-party action not pinned to a full commit SHA.
15. `actions/checkout` leaves credentials persisted when not needed.
16. Self-hosted runner labels available to fork PR workflows.
17. Cache from untrusted branch consumed by privileged workflow.
18. Secrets exposed to composite actions or Docker actions that do not need them.
19. Bot token can push directly to protected branch.
20. Bot token can approve its own PR.
21. Workflow file changes do not require CODEOWNERS review.
22. OIDC cloud trust policy does not constrain repository, branch, workflow, or environment.
23. Deployments run from tags that anyone can create.
24. No environment protection for production.
25. No Dependabot/security updates for actions.
26. No CodeQL/code scanning for workflow injection patterns.
27. No audit on workflow changes or token permission changes.
28. Using public reusable workflows without pinning.
29. Passing secrets to reusable workflows without strict trust.
30. AI agent granted a write token and allowed to modify `.github/workflows`.

## B. Use least privilege everywhere

1. **Set default token permissions to read-only or none.**
2. **Do not use `write-all`.**
3. **Use job-level permissions, not workflow-wide write permissions.**
4. **Use separate credentials for separate systems.**
5. **Use short-lived credentials.**
6. **Prefer OIDC/federated identity to static cloud keys.**
7. **Use token allowlists/scopes.**
8. **Prevent CI from approving or merging its own changes.**
9. **Require human review for CI config changes.**
10. **Use CODEOWNERS for workflow files.**

## Bad behavior

1. **Running untrusted pull-request code with secrets available.**
2. **Using GitHub `pull_request_target` to build, install, test, or execute code from the pull request.**
3. **Using `workflow_run` as a privilege bridge without validating artifacts.**
4. **Checking out attacker-controlled code in a privileged workflow.**
5. **Treating “internal PR” as trusted by default.**
6. **Allowing untrusted code to influence privileged jobs through files.**
7. **Running default-branch scheduled jobs that execute repository code after unreviewed changes.**
8. **Allowing PR titles, branch names, issue comments, commit messages, labels, or usernames to flow directly into shell scripts.**
9. **Using comment-triggered workflows such as `/test`, `/deploy`, or `/rerun` without authorization checks.**
10. **Letting CI jobs from low-trust branches write to high-trust locations.**
11. **Using a single pipeline for both untrusted validation and privileged release work.**
12. **Trusting CI configuration generated by code from the same untrusted branch.**
13. **Allowing pull requests to modify CI policy without mandatory review from CI/security owners.**
14. **Assuming “tests only” jobs are safe.**
15. **Using untrusted test data as executable configuration.**
16. **Hardcoding secrets in CI YAML.**
17. **Setting secrets at workflow/global scope.**
18. **Providing secrets to jobs that only need to run tests.**
19. **Using long-lived cloud credentials instead of short-lived identity federation.**
20. **Using personal access tokens where a scoped automation token would work.**
21. **Using admin/org-owner tokens in CI.**
22. **Using deploy keys with write access when read-only is sufficient.**
23. **Sharing one credential across projects, environments, or runners.**
24. **Failing to mask, hide, protect, or scope CI variables.**
25. **Assuming masked secrets cannot leak.**
26. **Printing environment variables.**
27. **Using `set -x` or shell debug mode in jobs with secrets.**
28. **Writing secrets into artifacts.**
29. **Writing secrets into caches.**
30. **Passing secrets through child-pipeline inputs or shared variables that are logged in plaintext.**
31. **Using structured secrets that masking systems cannot reliably redact.**
32. **Failing to rotate secrets after CI compromise.**
33. **Keeping logs and artifacts forever.**
34. **Letting any user with write access read or indirectly exfiltrate repository/workspace variables.**
35. **Using production secrets for test environments.**
36. **Using real customer data in tests.**
37. **Giving CI direct access to production databases, production message queues, production object stores, or production Kubernetes clusters.**
38. **Allowing test jobs to mint deployment credentials.**
39. **Allowing secrets in local runner host files.**
40. **Using broad default token permissions.**
41. **Using `permissions: write-all`.**
42. **Granting `contents: write` to jobs that only read code.**
43. **Granting `pull-requests: write` to jobs that run untrusted code.**
44. **Granting package publish permissions to test jobs.**
45. **Allowing Actions or bots to create and approve pull requests.**
46. **Using one CI service account for everything.**
47. **No environment-level approvals for production secrets or deploys.**
48. **No protected branches or protected tags.**
49. **Deployment jobs runnable from arbitrary branches.**
50. **Release jobs runnable from unprotected tags.**
51. **No CODEOWNERS or required reviewers for CI files.**
52. **Letting bot-authored PRs bypass CI/security review.**
53. **Allowing workflow dispatch inputs to select arbitrary refs, scripts, images, environments, or commands.**
54. **Allowing user-controlled paths in upload/download artifact steps.**
55. **Using overly broad cloud trust policies.**
56. **Missing separation between staging and production deployment credentials.**
57. **Letting one compromised test job affect all later jobs.**
58. **Running untrusted code on self-hosted runners.**
59. **Using persistent self-hosted runners without complete cleanup.**
60. **Using the same runner for trusted and untrusted jobs.**
61. **Using the same runner for different tenants, organizations, business units, or sensitivity levels.**
62. **Using GitLab shell executor for untrusted jobs.**
63. **Leaving SSH keys or deployment credentials on the runner host.**
64. **Mounting the Docker socket into jobs.**
65. **Adding CI users to the `docker` group.**
66. **Using privileged Docker-in-Docker for untrusted builds.**
67. **Running CI containers as root unnecessarily.**
68. **Allowing hostPath mounts in Kubernetes runners.**
69. **Using cluster-admin Kubernetes service accounts for CI pods.**
70. **Running CI pods in the same Kubernetes namespace as production workloads.**
71. **No Kubernetes NetworkPolicy or egress control for runners.**
72. **Allowing CI jobs to reach internal networks by default.**
73. **Allowing CI jobs to reach cloud metadata endpoints.**
74. **Allowing arbitrary outbound internet from privileged jobs.**
75. **No CPU, memory, disk, process, or runtime limits.**
76. **No job timeout.**
77. **No cleanup of Docker images, volumes, buildx builders, or service containers.**
78. **No runner patching process.**
79. **No runner version pinning or upgrade tracking.**
80. **No runner attestation or inventory.**
81. **No logging of which runner executed which job.**
82. **No quarantine or reprovisioning after suspicious jobs.**
83. **Using public shared runners for sensitive closed-source or regulated workloads without understanding data exposure and compliance constraints.**
84. **Using internal self-hosted runners for public pull requests.**
85. **Using generic runner labels such as `self-hosted` without more restrictive labels.**
86. **Letting jobs choose arbitrary runner labels from user-controlled input.**
87. **Running CI directly on the Git hosting server.**
88. **Using third-party GitHub Actions without pinning to a full commit SHA.**
89. **Using `@main`, `@master`, `@latest`, or mutable major-version tags for critical actions.**
90. **Using third-party actions with write tokens or secrets unless absolutely necessary.**
91. **Using actions from unverified, abandoned, newly created, or typo-squatted maintainers.**
92. **Not reviewing action source code.**
93. **Not reviewing transitive dependencies of actions.**
94. **Not using Dependabot/Renovate or equivalent to manage pinned action updates.**
95. **Using public Docker images without digest pinning.**
96. **Using `image: latest` in GitLab or Bitbucket pipelines.**
97. **Pulling remote scripts with `curl | bash`, `wget | sh`, PowerShell `iwr | iex`, or similar.**
98. **Downloading installers without checksum or signature verification.**
99. **Using GitLab `include: remote` without pinning to an immutable ref.**
100. **Using GitLab CI/CD components with `@latest`.**
101. **Using Bitbucket Pipes without version pinning or source review.**
102. **Trusting package-manager lifecycle scripts from untrusted branches.**
103. **Letting test dependencies run arbitrary postinstall/build scripts before security controls are applied.**
104. **Using untrusted GitHub Actions in privileged `pull_request_target` or release workflows.**
105. **Using actions that read all environment variables when they only need one token.**
106. **Allowing third-party uploaders to access all CI environment variables.**
107. **Ignoring known CI supply-chain incidents.**
108. **Restoring caches created by untrusted branches into trusted jobs.**
109. **Using cache keys that allow cross-branch or cross-PR poisoning.**
110. **Using cache keys based only on branch name.**
111. **Using cache keys based on attacker-controlled files without isolation.**
112. **Caching directories that contain secrets.**
113. **Caching build outputs that later privileged jobs execute.**
114. **Downloading artifacts from untrusted jobs and executing them.**
115. **Publishing artifacts from untrusted branches as release artifacts.**
116. **Uploading entire workspaces as artifacts.**
117. **Uploading `.git` directories.**
118. **Uploading test reports that include secrets, request headers, cookies, or screenshots of sensitive systems.**
119. **No artifact integrity verification.**
120. **No artifact expiration.**
121. **Artifact promotion replaced by environment rebuilds.**
122. **Using caches as a deployment channel.**
123. **Using cache fallback keys that make trusted jobs fall back to untrusted caches.**
124. **Storing dependency lockfiles in caches instead of version control.**
125. **Not invalidating caches after dependency, compiler, image, or toolchain changes.**
126. **Cache keys too specific, causing constant misses and slow builds.**
127. **Cache keys too broad, causing stale or contaminated builds.**
128. **Changing GitLab hashed cache-key settings inconsistently across runners.**
129. **Letting tests pass when the test command failed.**
130. **Marking required test jobs as optional.**
131. **Continuing deployment after failed tests.**
132. **Running tests but not collecting reports.**
133. **Treating “no tests found” as success.**
134. **Using broad `continue-on-error` to hide flakiness.**
135. **Rerunning failing tests until they pass without tracking flake rate.**
136. **Quarantining flaky tests forever.**
137. **Disabling flaky tests with no owner, ticket, expiry date, or re-enable policy.**
138. **Overusing end-to-end tests for basic validation.**
139. **No deterministic seed handling.**
140. **No isolation between tests.**
141. **Tests depend on order.**
142. **Tests depend on wall-clock time, timezone, locale, or current date without control.**
143. **Tests depend on external network services without mocks, fixtures, retries, or contract boundaries.**
144. **Tests use production APIs.**
145. **Tests mutate shared staging environments without cleanup.**
146. **Tests race because CI parallelism was added without resource isolation.**
147. **No timeout for tests.**
148. **No per-test timeout.**
149. **No slow-test reporting.**
150. **No failure classification.**
151. **No test ownership.**
152. **No test-impact analysis in large monorepos.**
153. **Path filters skip tests that are actually impacted by shared code changes.**
154. **Unit, integration, E2E, performance, and security tests all run in one opaque job.**
155. **Coverage is uploaded from untrusted PRs using privileged tokens.**
156. **Coverage or quality gates are advisory only but branch protection treats them as required.**
157. **Snapshot tests are auto-updated in CI.**
158. **Tests commit generated changes back to the branch without human review.**
159. **CI agent deletes, rewrites, or weakens tests to make the pipeline green.**
160. **CI agent changes assertions into smoke checks.**
161. **CI agent marks failing tests as skipped instead of fixing the product or test.**
162. **No negative/security tests in pipeline.**
163. **No regression tests for previously exploited CI weaknesses.**
164. **Running every possible job on every push.**
165. **No path filters or change detection.**
166. **No test selection or test-impact analysis.**
167. **No pipeline stages by feedback speed.**
168. **Monolithic pipelines with long sequential stage waterfalls.**
169. **Not using DAG/`needs` dependencies where supported.**
170. **Excessive matrix expansion.**
171. **No cap on matrix size.**
172. **No cancellation of superseded runs.**
173. **No `interruptible` jobs in GitLab for superseded pipelines.**
174. **No queue visibility.**
175. **Over-parallelizing beyond runner capacity.**
176. **Under-parallelizing expensive independent tests.**
177. **Serializing jobs that could safely run independently.**
178. **Running expensive security scans on generated/vendor directories.**
179. **Running full historical secret scans on every commit.**
180. **No dependency cache.**
181. **Bad dependency cache keys.**
182. **Caching too much.**
183. **Caching unstable build outputs.**
184. **No Docker layer caching or image reuse where appropriate.**
185. **Building the same image in multiple jobs.**
186. **Installing the same system packages in every job.**
187. **Downloading the same toolchain in every job.**
188. **Not using prebuilt CI images for stable toolchains.**
189. **Using giant CI images for small jobs.**
190. **Always doing full Git clone.**
191. **Always fetching submodules, LFS files, or full history when not needed.**
192. **No artifact reuse between build and test.**
193. **Rebuilding per environment instead of promoting an artifact.**
194. **No build-system incremental cache.**
195. **Using package-manager commands that are non-deterministic or slow.**
196. **Using floating dependency versions.**
197. **No timeout on jobs that can hang.**
198. **No retry policy for known transient infrastructure failures.**
199. **Retrying product failures as if they were infrastructure failures.**
200. **No separation between smoke tests and full regression tests.**
201. **E2E tests blocked by shared staging environment contention.**
202. **Performance tests run on noisy shared runners and produce meaningless results.**
203. **No baseline, variance tracking, or trend analysis for performance tests.**
204. **Uploading huge artifacts on every run.**
205. **Excessive artifact retention.**
206. **No cleanup of old artifacts, caches, and container layers.**
207. **No observability for critical path.**
208. **No visibility into slow tests.**
209. **No visibility into runner utilization.**
210. **No cost allocation by repository, team, branch, or workflow.**
211. **Pipelines duplicate because push and pull-request triggers overlap.**
212. **Using one CI YAML file so large that teams cannot reason about it.**
213. **Deploying from untrusted branches.**
214. **Deploying from unprotected tags.**
215. **Deploying automatically from pull requests.**
216. **Running deployment jobs in the same trust context as PR validation.**
217. **No environment approvals for production.**
218. **No separation between test, staging, and production credentials.**
219. **No artifact promotion.**
220. **No release provenance, SBOM, signature, or attestation.**
221. **Publishing packages from test jobs.**
222. **Publishing on every branch.**
223. **Publishing with mutable version tags.**
224. **Overwriting previously published artifacts.**
225. **Using `latest` as the primary deploy mechanism.**
226. **Allowing CI to force-push release branches.**
227. **Allowing CI to move release tags.**
228. **No deploy concurrency lock.**
229. **No rollback mechanism.**
230. **No deployment audit trail.**
231. **No manual break-glass path with logging.**
232. **No post-deploy validation.**
233. **No change window or freeze enforcement where required.**
234. **No separation of duties for regulated environments.**
235. **Tests against production after deployment use privileged credentials and leak data into logs.**
236. **Infrastructure-as-code plans are generated in one job and applied in another without verifying the plan artifact.**
237. **Terraform or cloud credentials are available to jobs that only run tests.**
238. **No alert when CI config changes.**
239. **No required review for CI config changes.**
240. **No audit trail for who changed secrets, variables, runners, deploy keys, webhooks, environments, or branch protections.**
241. **No monitoring for newly registered self-hosted runners.**
242. **No monitoring for runner label changes.**
243. **No detection for unusual outbound traffic from CI.**
244. **No detection for crypto-mining behavior.**
245. **No detection for mass secret access.**
246. **No detection for new package-publishing jobs.**
247. **No detection for new `pull_request_target`, `workflow_run`, scheduled, or manual-dispatch workflows.**
248. **No detection for `curl | bash`, mutable action refs, `latest` images, or `write-all` permissions.**
249. **No security scan on CI YAML.**
250. **No periodic review of CI tokens and variables.**
251. **No inventory of third-party actions/components/pipes.**
252. **No review of workflow run logs after suspicious runs.**
253. **No incident-response playbook for CI compromise.**
254. **No ability to revoke all CI credentials quickly.**
255. **No cache purge procedure.**
256. **No artifact purge procedure.**
257. **No runner reimage/reprovision procedure.**
258. **No branch protection verification after CI changes.**
259. Running public or untrusted repository CI directly on the GitBucket server.
260. Running arbitrary commands on the Git hosting instance.
261. Storing deployment secrets on the GitBucket host.
262. Using the GitBucket CI plugin as a general-purpose scalable CI system.
263. Mixing Git hosting, CI execution, and deployment credentials on one machine.
264. No isolation between repositories.
265. No ephemeral execution environment.
266. No cleanup between builds.
267. No secret isolation.
268. No audit or policy enforcement around commands run by repository owners.

## Bitbucket

1. Use deployment environments and deployment variables for environment-specific credentials.
2. Use secured variables only for true secrets.
3. Do not pass secrets through child-pipeline input variables.
4. Use OIDC/federated identity for cloud access where available.
5. Keep repository/workspace variables scoped and periodically reviewed.

## Bitbucket Pipelines baseline

1. step:
2. node
3. npm ci
4. npm test
5. npm run build
6. dist/**
7. ./ci/deploy-with-oidc.sh
8. Reject plaintext secrets in CI YAML.
9. Reject `permissions: write-all`.
10. Require explicit GitHub `permissions`.
11. Reject unpinned GitHub Actions.
12. Reject Docker `latest` in sensitive jobs.
13. Reject unpinned GitLab remote/project includes.
14. Reject `curl | bash` unless allowlisted with hash/signature verification.
15. Reject `pull_request_target` workflows that checkout PR head.
16. Reject direct interpolation of PR title/body/comment/branch into shell.
17. Reject `eval` in CI scripts.
18. Reject `printenv`, `env`, `set -x`, and debug tracing in secret-bearing jobs.
19. Reject `allow_failure`/`continue-on-error` on security scans.
20. Reject deployment jobs from non-protected branches.
21. Reject production deploys without environment approval.
22. Reject privileged containers unless allowlisted.
23. Reject Docker socket mounts.
24. Reject caches containing `.ssh`, `.aws`, `.docker`, `.npmrc`, `.pypirc`, `.netrc`, `.kube`, `.env`.
25. Reject artifacts containing credentials or entire workspaces.
26. Require artifact expiration.
27. Require job timeouts.
28. Require cache keys based on lockfiles.
29. Require protected runners/variables for deploy jobs.
30. Require CODEOWNERS approval for CI changes.
31. Require OIDC for cloud deployments where supported.
32. Require artifact signing/provenance for releases.
33. Block AI agents from modifying CI permissions without human approval.
34. Block AI agents from changing workflow files unless explicitly authorized.
35. Block bot self-approval.
36. Alert on new runners, new secrets, new third-party apps, and workflow changes.
37. Alert on suspicious CI commands and unusual egress.

## Bitbucket Pipelines policy checks

1. Secrets in `bitbucket-pipelines.yml`.
2. Secured-variable assumptions in unsupported contexts.
3. Child-pipeline input variables used for secrets.
4. Deployment from broad branch patterns.
5. Branch-based environment rebuild instead of artifact promotion.
6. Duplicate default and PR pipelines.
7. `image: latest`.
8. Unpinned Pipes.
9. No dependency cache for expensive dependency installs.
10. Cache includes secret-prone directories.
11. Full clone where shallow clone would work.
12. No JUnit/test reports.
13. Artifacts include full workspace.
14. Production deployment without deployment environment controls.
15. Long-lived cloud keys where OIDC could be used.

## Bitbucket: safer pipeline baseline

1. step:
2. node
3. npm ci
4. npm test
5. npm run build
6. dist/**
7. ./scripts/deploy-staging.sh
8. CI YAML changes require code-owner review.
9. PR/fork jobs run without secrets and with read-only tokens.
10. `pull_request_target` is banned unless explicitly approved by security/platform owners.
11. Third-party Actions/Pipes/plugins/images must be pinned and reviewed.
12. Release/deploy/signing jobs run only from protected refs.
13. Production deployments require protected environments and human/policy approval.
14. Self-hosted runners are separated by trust level and are ephemeral for untrusted code.
15. Docker privileged mode and Docker socket access are banned for untrusted jobs.
16. Secrets are scoped, short-lived, masked/protected, and preferably brokered through OIDC or an external secret manager.
17. Caches and artifacts must not contain secrets.
18. Security scans cannot be permanently non-blocking.
19. CI logs, workflow edits, runner changes, secret changes, and deployment approvals are auditable.
20. AI agents cannot weaken CI, approve themselves, expose secrets, or modify release/deployment workflows without protected review.

## C. Bad caching behavior

1. No dependency cache.
2. Cache key changes every commit, producing constant misses.
3. Cache key never changes, producing stale or corrupted builds.
4. Cache key does not include lockfiles.
5. Cache includes secrets.
6. Cache includes `.ssh`, `.aws`, `.docker`, `.npmrc`, `.pypirc`, `.netrc`, or kubeconfig.
7. Cache includes build outputs that should be artifacts.
8. Cache includes the whole repository.
9. Cache includes the whole home directory.
10. Cache includes operating-system-specific paths but is shared across OSes.
11. Cache is shared between trusted and untrusted branches.
12. Cache is shared between protected and unprotected branches.
13. Untrusted PRs can poison cache used by privileged jobs.
14. Cache is too large to save/restore efficiently.
15. Cache upload time exceeds dependency install time.
16. Cache stores non-deterministic build state.
17. Cache masks missing dependencies.
18. Cache is not pruned.
19. Cache is used for Docker layers without considering secret leakage.
20. Cache paths are wrong, so the pipeline thinks it is caching but never hits.

## C. Bad command execution and injection behavior

1. **Interpolating untrusted expressions directly into `run:`.**
2. run: echo "${{ github.event.pull_request.title }}"
3. **Executing arbitrary package scripts from untrusted PRs with secrets present.**
4. **Using `eval`, `bash -c`, `sh -c`, `python -c`, or dynamic command construction from CI variables.**
5. **Running `curl | bash` or `wget | sh` in CI.**
6. **Executing downloaded artifacts without checksum/signature verification.**
7. **Using mutable tags for tools and scripts.**
8. **Executing untrusted artifacts in privileged follow-up jobs.**
9. **Allowing test code to alter CI control files.**
10. **Writing untrusted multiline content to CI environment files.**

## C. Bitbucket Pipelines: safer artifact promotion shape

1. step:
2. node
3. npm ci
4. npm test -- --ci
5. reports/junit.xml
6. npm run build
7. dist/**
8. ./deploy.sh dist

## C. GitLab CI-specific bad behavior

1. Storing secrets under `variables:` in `.gitlab-ci.yml`.
2. Relying on masked variables while allowing malicious jobs to transform or exfiltrate them.
3. Not using hidden/protected variables for sensitive values.
4. Production variables available to non-protected branches.
5. Pipeline variables allowed broadly.
6. Manual pipeline variables can override safer defaults.
7. `rules:` accidentally expose deploy jobs to merge-request pipelines.
8. Fork/MR pipelines can access variables or protected runners due misconfiguration.
9. Shared runners used for sensitive jobs.
10. Runners not scoped to the lowest practical group/project.
11. Missing runner tags for sensitive jobs.
12. Unprotected runners allowed to pick up protected jobs.
13. `privileged = true` runner configuration used casually.
14. Docker-in-Docker used for jobs that do not require it.
15. `pull_policy: if-not-present` on shared or public runners.
16. `include:project` uses a branch ref instead of a full SHA.
17. `include:remote` used without integrity or trust review.
18. Remote includes maintained by a different team without review.
19. `.gitlab-ci.yml` changes not controlled by CODEOWNERS.
20. Production deployment configuration stored in the same repo and editable by all maintainers.
21. `allow_failure: true` on SAST, dependency scanning, secret detection, IaC scanning, or container scanning.
22. `when: always` causes sensitive jobs to run in unexpected pipeline sources.
23. Missing `workflow: rules`, causing duplicate branch and MR pipelines.
24. Heavy global `before_script`.
25. Cache key not based on lockfiles.
26. Cache shared across trust zones.
27. Artifacts collected with broad paths.
28. `artifacts:untracked: true` used carelessly.
29. Missing `expire_in`.
30. Missing artifact access restriction.
31. Jobs download all previous artifacts by default.
32. No `needs` DAG; everything waits stage by stage.
33. No `needs:artifacts` control.
34. No `dependencies: []` where artifact download is unnecessary.
35. Deployment jobs are interruptible.
36. Deploy jobs run automatically from non-default branches.
37. Tags deploy without protected tags.
38. `CI_JOB_TOKEN` has broader access than needed.
39. Group-level variables used where project/environment variables would be safer.
40. AI agent allowed to edit `.gitlab-ci.yml`, variables, includes, and runner tags.

## C. Handle untrusted PRs/MRs safely

1. **Use unprivileged PR workflows for fork code.**
2. uses: actions/checkout@<full-sha>
3. run: npm ci
4. run: npm test
5. **Avoid `pull_request_target` unless you fully understand the trust boundary.**
6. **Never checkout and execute PR head code inside `pull_request_target`.**
7. **Do not expose secrets to fork PR jobs.**
8. **Do not run release, deploy, package publish, or signing jobs on PR events.**
9. **If a trusted maintainer must trigger extra tests, check the actor at runtime.**
10. **Treat labels/comments as requests, not authorization.**
11. **Validate branch names, labels, comments, PR titles, and workflow inputs.**
12. **Use allowlists for manually supplied inputs.**
13. **Avoid direct interpolation in shell.**
14. run: echo "${{ github.event.pull_request.title }}"
15. env:

## C. Split untrusted and privileged workflows

1. uses: actions/checkout@<full-commit-sha>
2. name: Install
3. name: Test
4. uses: actions/checkout@v4
5. run: npm install && npm test

## C. Use least privilege everywhere

1. Set the default CI token to read-only.
2. Grant write permissions only at the job that needs them.
3. Separate tokens for:
4. source checkout
5. package publishing
6. artifact upload
7. staging deploy
8. production deploy
9. Use environment-specific credentials.
10. Use repository/project-level credentials instead of organization/workspace-level credentials when possible.
11. Use short-lived credentials.
12. Prefer OIDC/federation over static cloud keys.
13. Bind OIDC trust to repository, branch/ref, workflow, environment, and audience.
14. Expire service accounts and tokens.
15. Rotate tokens automatically or on a schedule.
16. Remove stale deploy keys, app passwords, webhooks, OAuth apps, and GitHub/GitLab/Bitbucket integrations.
17. Do not use personal SSH keys or personal access tokens for CI.
18. Prevent bots from approving their own PRs.
19. Prevent bots from bypassing branch protection.
20. Ensure test jobs cannot deploy.
21. Ensure deployment jobs cannot rewrite source.
22. Ensure artifact-publishing jobs cannot modify workflow files.

## D. Bad artifact behavior

1. Uploading the entire workspace as an artifact.
2. Using `untracked` artifact capture carelessly.
3. Storing dependency caches as artifacts.
4. Storing artifacts indefinitely.
5. Storing huge logs, videos, screenshots, and coverage reports for every run.
6. Making artifacts available to users who do not need them.
7. Downloading artifacts into every later job by default.
8. Publishing test artifacts before removing secrets.
9. Publishing `.env`, credentials, SSH keys, kubeconfig, Terraform state, or debug dumps.
10. Storing production build artifacts from untrusted branches.
11. Deploying from artifacts that were not validated.
12. Not setting `expire_in` or equivalent retention.
13. Not limiting artifact access.
14. Creating one artifact per test shard when a merged report would suffice.
15. Using artifact paths that include large dependency directories.
16. Uploading artifacts even on success when they are only needed for debugging failures.
17. Using artifacts instead of a proper package/container registry.
18. Not promoting immutable artifacts by digest.
19. Not compressing or deduplicating large artifacts.
20. Keeping latest artifacts forever without realizing storage impact.

## D. Bad dependency/install behavior

1. **Re-resolving dependencies every run.**
2. **No lockfiles.**
3. **Using `npm install` instead of `npm ci` in deterministic CI contexts.**
4. **Updating dependencies during tests.**
5. **Downloading build tools from slow external URLs every run.**
6. **No internal mirrors for heavy dependencies.**
7. **No retry/backoff for transient package registry failures.**
8. **Retrying too aggressively and amplifying outages.**
9. **No offline mode for stable dependency sets.**
10. **Mixing build tool installation with test execution so failures are hard to classify.**

## D. Bitbucket Pipelines-specific bad behavior

1. Secured variables treated as impossible to exfiltrate.
2. Users with write access can modify `bitbucket-pipelines.yml` to print or artifact environment variables.
3. Repository variables used for production secrets.
4. Deployment variables available without deployment permissions.
5. Personal SSH keys stored as repository variables.
6. Bitbucket app passwords used broadly.
7. OIDC not used for cloud access.
8. OIDC trust policy not constrained by repository, branch, environment, or deployment.
9. Runners registered broadly at workspace level.
10. Runner labels allow unrelated jobs to land on sensitive runners.
11. Runner registration command/token not stored securely.
12. Deployment branches not restricted.
13. Production deployments can run from feature branches.
14. Merge checks do not require passing builds.
15. Branch permissions do not restrict changes to release branches.
16. `max-time` not set for long-running or runaway jobs.
17. No cache for dependency installation.
18. Caches contain secrets.
19. Artifacts retained inappropriately or pushed to Bitbucket when external artifact storage is needed.
20. Pipes or third-party tools used without review or pinning.
21. Child processes receive secrets unnecessarily.
22. Pull-request pipelines run the same secret-bearing steps as main-branch pipelines.
23. YAML changes do not require review.
24. AI agent allowed to rewrite `bitbucket-pipelines.yml` and push to branches that can access secrets.
25. Pipeline can deploy based on branch names without explicit deployment permission.
26. Agent changes CI YAML to make checks pass instead of fixing code.
27. Agent disables tests.
28. Agent marks failing tests as skipped.
29. Agent weakens assertions.
30. Agent adds `continue-on-error`, `allow_failure`, or `|| true`.
31. Agent removes security scans.
32. Agent increases token permissions.
33. Agent adds new secrets.
34. Agent prints secrets while debugging.
35. Agent uploads full logs to an external tool.
36. Agent sends source, logs, secrets, dependency lists, or vulnerability reports to an external LLM/tool without policy.
37. Agent follows malicious instructions embedded in PR text, comments, test names, code comments, README files, or failure logs.
38. Agent uses issue comments as commands without authentication.
39. Agent approves its own PR.
40. Agent merges its own CI-permission changes.
41. Agent creates a new workflow file that bypasses existing checks.
42. Agent creates a new branch/tag/release that triggers deployment.
43. Agent opens repeated tiny commits that trigger expensive full pipelines.
44. Agent creates infinite loops: CI fails, agent commits, CI runs, agent commits again.
45. Agent retries flaky tests until green.
46. Agent deletes evidence: logs, artifacts, failing screenshots, or test reports.
47. Agent introduces unpinned actions/packages/images.
48. Agent adds `curl | bash` installers.
49. Agent uses production credentials for testing.
50. Agent uses live production services as test fixtures.
51. Agent changes branch protection or required checks.
52. Agent bypasses codeowners by moving CI code into unprotected includes.
53. Agent copies credentials into generated files.
54. Agent stores generated secrets in the repo.
55. Agent caches too much state.
56. Agent causes pipeline-cost explosions through unnecessary matrices.
57. Agent treats attacker-controlled CI output as truth.
58. Agent cannot produce a clear diff rationale for security-sensitive CI changes.
59. Agent modifies release or deployment logic without human approval.
60. Agent ignores organization policy-as-code findings.
61. Agent adds third-party services without reviewing permissions.
62. Agent silently changes artifact retention.
63. Agent deploys from a dirty workspace or unreviewed commit.
64. Agent uses mutable refs for “latest” tools.
65. Agent lacks an allowlist of permitted files, commands, and workflow changes.

## D. Jenkinsfile and GitBucket/Jenkins-style integrations

1. **Jenkinsfile not stored in source control.**
2. **Untrusted jobs using trusted credentials.**
3. **Groovy string interpolation with secrets.**
4. **Global credentials available to too many jobs.**
5. **Unsafe Jenkins authorization settings.**
6. **Static cloud keys in Jenkins credentials instead of instance roles/federation.**
7. **Using experimental or weakly maintained CI plugins for production.**
8. **Ignoring plugin vulnerabilities.**
9. **Public repos running fork PR code with secrets because `pull_request_target` was misunderstood.**
10. **Attackers submitting PRs that alter CI to exfiltrate secrets.**
11. **Third-party actions treated as harmless utilities.**
12. **Using CI bots/PATs with excessive permissions.**

## D. Use ephemeral, isolated runners

1. Use hosted ephemeral runners for public PRs.
2. Use self-hosted runners only for trusted code, or use ephemeral just-in-time self-hosted runners that are destroyed after one job.
3. Separate runner pools by trust level:
4. public PR validation
5. internal PR validation
6. protected branch build
7. release/publish
8. production deploy
9. Never let untrusted jobs run on deployment runners.
10. Avoid shell executors for untrusted code.
11. Avoid privileged Docker-in-Docker for untrusted code.
12. Avoid Docker socket mounts.
13. Remove SSH keys and persistent cloud credentials from runner hosts.
14. Deny access to production/internal networks by default.
15. Restrict outbound egress for high-risk jobs.
16. Apply CPU, memory, disk, and runtime limits.
17. Clean workspaces, containers, volumes, and temp files after every job.
18. Patch runner hosts and runner binaries.
19. Keep runner inventory and job-to-runner audit logs.
20. Reimage runners after suspicious jobs.

## Dangerous `run:` injection patterns

1. Direct interpolation of untrusted contexts into shell:
2. Branch names used in shell without safe environment passing.
3. PR body, issue body, review body, comment body, commit message, author email, label name, milestone, or release notes inserted into `run`.
4. Shell conditionals built from user input.
5. Dynamic command construction from workflow inputs.
6. Unquoted variables in shell.
7. `eval`, backticks, `bash -c`, `sh -c`, PowerShell `Invoke-Expression`, or Python `exec` using event data.
8. Writing event data into a script and executing it.
9. Writing event data into YAML/JSON consumed by deployment tooling without schema validation.
10. Assuming branch names are safe strings.

## Dangerous action/component usage

1. `uses: owner/action@main`.
2. `uses: owner/action@master`.
3. `uses: owner/action@v1` for sensitive jobs rather than full SHA.
4. No review of action source.
5. Using abandoned actions.
6. Using typo-squatted actions.
7. Using third-party actions in jobs with secrets.
8. Using third-party actions in jobs with write tokens.
9. Using Docker actions with unpinned base images.
10. Using `docker://image:latest`.
11. Using actions that execute downloaded scripts.
12. Using actions that read all env vars.
13. No dependency update process for pinned actions.

## Dangerous images/dependencies

1. `image: latest`.
2. `services: docker:dind` with privileged mode by default.
3. Public image tags not pinned.
4. No image scanning.
5. Remote install scripts.
6. Package installs without lockfiles.
7. Dependency caches shared across trust boundaries.
8. Build scripts from untrusted MRs run with secrets.
9. No CI component pinning.
10. CI/CD components use global variables instead of validated inputs.

## Dangerous permissions

1. Missing top-level `permissions:`.
2. `permissions: write-all`.
3. `contents: write` in test jobs.
4. `pull-requests: write` in jobs that run PR code.
5. `actions: write` except in tightly controlled administrative workflows.
6. `id-token: write` on jobs that do not need cloud federation.
7. `packages: write` in test jobs.
8. `issues: write` or `pull-requests: write` just to report status when safer mechanisms exist.
9. Using `GITHUB_TOKEN` to push changes from a test job.
10. Allowing Actions to approve PRs or create self-approving change loops.

## Dangerous rules/triggers

1. `only`/`except` patterns that accidentally run deploy jobs on untrusted branches.
2. Broad `rules:` that run privileged jobs for merge requests.
3. Duplicate push and merge-request pipelines.
4. Release jobs triggered by unprotected tags.
5. Deploy jobs without protected environments.
6. Child pipelines generated from untrusted branch code.
7. Dynamic includes controlled by untrusted input.
8. `include: remote` without immutable pinning.
9. `include: project` from an untrusted or mutable ref.
10. No `workflow: rules` to control when pipelines should exist.

## Dangerous runner usage

1. `runs-on: self-hosted` for pull requests.
2. Generic self-hosted labels.
3. Letting workflow inputs choose runner labels.
4. Using the same self-hosted label for deploy and test jobs.
5. Running public PRs on internal network runners.
6. No `timeout-minutes`.
7. No resource cleanup.
8. No job isolation.
9. Docker socket mounted into self-hosted runner jobs.
10. Secrets or SSH keys stored on the runner host.

## Dangerous runners/executors

1. Shell executor for untrusted code.
2. Shared runner for trusted and untrusted jobs.
3. Protected variables available on unprotected runners.
4. Deployment jobs not restricted to protected runners.
5. Privileged Docker executor for untrusted jobs.
6. Docker socket mounted.
7. CI user has access to host secrets.
8. No runner cleanup.
9. Runner can reach internal/prod networks.
10. Runner has static cloud credentials.

## Dangerous triggers and privilege boundaries

1. `on: pull_request_target` combined with dependency install, build, test, lint, Docker build, package scripts, or arbitrary `run`.
2. `pull_request_target` combined with checkout of the PR head SHA or branch.
3. `workflow_run` that downloads an artifact from an untrusted workflow and executes files from it.
4. `issue_comment`, `pull_request_review`, or `repository_dispatch` workflows that execute commands from comments, labels, titles, or user input.
5. `workflow_dispatch` inputs used as shell commands, refs, image names, artifact names, environment names, or paths without allowlists.
6. `schedule` workflows that run privileged tasks and execute mutable repository scripts.
7. `push` triggers on all branches for jobs that publish or deploy.
8. `tags: ["*"]` release workflows without protected tag enforcement.
9. Missing `paths` or `paths-ignore` on expensive workflows.
10. Overlapping `push` and `pull_request` workflows that duplicate work.

## Deployment & Releases

1. Build once, deploy the same artifact.
2. Promote by digest, not by tag.
3. Sign release artifacts.
4. Generate provenance/attestations.
5. Generate SBOMs.
6. Verify signatures before deployment.
7. Verify checksums before deployment.
8. Store artifacts in a proper registry.
9. Restrict artifact upload permissions.
10. Restrict artifact download permissions.
11. Set artifact retention.
12. Do not execute artifacts from untrusted jobs in privileged jobs.
13. Do not use mutable names for release artifacts.
14. Scan artifacts before release.
15. Keep deployment metadata:
16. commit SHA
17. workflow run ID
18. builder identity
19. source repo
20. dependency lockfiles
21. image digest
22. signer identity
23. Preserve enough logs for incident response.
24. Do not rebuild in production deployment jobs.
25. Use separate staging and production promotion gates.
26. **Deploy only from protected refs.**
27. **Use environment approvals.**
28. **Use separate deploy credentials per environment.**
29. **Use deployment locks/concurrency controls.**
30. **Do not rebuild at deploy time. Promote a tested artifact.**
31. **Use signed artifacts/images.**
32. **Use provenance and SBOMs.**
33. **Never deploy from fork PRs.**
34. **Never publish packages from untrusted PRs.**
35. **Require manual approval or policy approval for production.**
36. **Keep rollback artifacts available and verified.**
37. **Audit release workflow changes.**
38. 'v*'
39. uses: actions/checkout@<full-length-commit-sha>
40. name: Build
41. name: Publish with OIDC
42. if: '$CI_COMMIT_BRANCH == "main"'
43. protected-deploy
44. ./scripts/deploy.sh
45. `default` pipeline runs expensive full validation on every branch.
46. Pull-request and branch pipelines both run for the same push.
47. Deployment steps run from broad branch patterns.
48. Branch deployments rebuild instead of promoting a tested artifact.
49. No deployment environment separation.
50. No deployment locks/stages for environments that need serial deployment.
51. No manual gate for production.
52. Publishing packages from PR validation.
53. Using `[skip ci]` without branch protection/required checks that prevent bypass.
54. No policy around forked PR behavior.

## E. Bad feedback and developer-experience behavior

1. **CI reports one giant failure instead of actionable job-level failures.**
2. **Logs are too noisy to find the failing test.**
3. **Logs are too sparse to reproduce the failure.**
4. **Failures do not include test artifacts where safe.**
5. **Artifacts are retained too briefly to debug.**
6. **Artifacts are retained too long when sensitive.**
7. **No ownership routing for failed jobs.**
8. **No flaky-test dashboard.**
9. **No pipeline duration SLO.**
10. **No cost visibility.**
11. **No per-team or per-job runner usage visibility.**
12. **No periodic cleanup of obsolete workflows.**
13. **No review of slowest jobs.**
14. **No enforcement against adding very slow PR gates.**
15. **Agent modifies CI YAML to make tests pass instead of fixing code.**
16. **Agent disables failing tests, scanners, linters, or type checks.**
17. **Agent adds `continue-on-error: true` or `allow_failure: true` to bypass gates.**
18. **Agent broadens permissions to fix an auth error.**
19. **Agent adds a PAT or secret to workflow files.**
20. **Agent prints environment variables while debugging.**
21. **Agent uploads full logs/artifacts containing secrets.**
22. **Agent summarizes secret-containing logs into PR comments.**
23. **Agent uses untrusted PR text, issue comments, or README instructions as operational instructions.**
24. **Agent runs arbitrary shell commands suggested by failing logs or external content.**
25. **Agent installs tools using `curl | bash` because it is expedient.**
26. **Agent pins nothing and uses `latest`.**
27. **Agent adds external GitHub Actions/Pipes/plugins without review.**
28. **Agent changes lockfiles opportunistically.**
29. **Agent updates dependencies to silence failures without assessing security/license impact.**
30. **Agent generates fake tests that assert implementation details but not behavior.**
31. **Agent deletes slow tests to improve CI time.**
32. **Agent writes non-deterministic tests.**
33. **Agent retries flaky tests until green and reports success.**
34. **Agent auto-merges its own PR.**
35. **Agent approves workflow changes it authored.**
36. **Agent comments secrets, internal URLs, or stack traces into public PRs.**
37. **Agent uses production credentials to reproduce test failures.**
38. **Agent opens network egress to arbitrary hosts.**
39. **Agent weakens branch protection or required checks.**
40. **Agent creates backdoor maintenance workflows.**
41. **Agent rewrites release scripts or signing steps without human review.**
42. **Agent fabricates benchmark improvements by changing measurement conditions.**
43. **Agent treats CI pass as proof of correctness/security.**
44. **Agent ignores failing security checks as “unrelated.”**
45. **Agent changes generated files or vendored code without provenance.**
46. **Agent does not produce an audit trail of what commands it ran and why.**

## E. Bad runner/agent isolation behavior

1. **Running untrusted jobs on persistent self-hosted runners.**
2. **Using shell executors for untrusted code.**
3. **Mounting the Docker socket into CI jobs.**
4. **Using Docker-in-Docker privileged mode for untrusted builds.**
5. **Adding CI users to the Docker group on shared machines.**
6. **Using the same runner for public forks, internal repos, and release jobs.**
7. **No network segmentation.**
8. **Allowing arbitrary outbound egress.**
9. **No cleanup between jobs.**
10. **Running as root unnecessarily.**
11. **Using broad runner labels/tags.**
12. **Allowing untagged jobs on sensitive runners.**
13. **No resource limits.**
14. **No timeout.**

## E. Harden runners and isolate workloads

1. Prefer ephemeral hosted runners for untrusted code.
2. Use one-job or just-in-time self-hosted runners where possible.
3. Do not run fork PRs on persistent self-hosted runners.
4. Separate runner pools by trust:
5. public/fork PR
6. internal test
7. package publish
8. staging deploy
9. production deploy
10. Separate runner pools by sensitivity:
11. no secrets
12. non-prod secrets
13. production secrets
14. Use protected runners for protected branches/tags only.
15. Use runner tags/labels narrowly.
16. Do not use privileged containers unless unavoidable.
17. Do not mount Docker socket into untrusted jobs.
18. Do not mount host directories into untrusted jobs.
19. Drop Linux capabilities.
20. Run as non-root where possible.
21. Patch runner OS and runner software.
22. Restrict outbound network access.
23. Block access to metadata endpoints unless needed.
24. Restrict internal-network access.
25. Clean workspaces and temp directories.
26. Prune Docker images/layers/volumes.
27. Rotate runner registration tokens.
28. Treat runner registration commands as secrets.
29. Log runner creation, deletion, and job assignment.
30. Apply CPU/memory/disk limits.
31. Use autoscaling to avoid queue buildup.

## E. Runner hardening best practices

1. **Use ephemeral runners for untrusted code.**
2. **Do not use shell executors for untrusted jobs.**
3. **Use separate runner pools by trust level.**
4. public fork PR pool,
5. internal branch test pool,
6. protected branch build pool,
7. release/signing pool,
8. deployment pool,
9. security scanning pool.
10. **Protect sensitive runners.**
11. **Disable untagged jobs on sensitive runners.**
12. **Avoid privileged Docker for untrusted jobs.**
13. **Avoid Docker socket mounts.**
14. **Pin runner images and base images.**
15. **Run jobs as non-root where possible.**
16. **Apply network egress controls.**
17. **Block access to cloud metadata endpoints from untrusted jobs.**
18. **Isolate caches by trust boundary.**
19. **Clean workspaces, containers, and credentials after every job.**
20. **Set CPU, memory, disk, and timeout limits.**
21. **Monitor runner registration and runner label changes.**
22. **Patch runners and CI plugins quickly.**

## Environments & Config

1. Running untrusted code on long-lived self-hosted runners.
2. Running public fork PRs on self-hosted runners.
3. Reusing the same runner workspace across jobs without guaranteed cleanup.
4. Leaving credentials, build artifacts, temp files, Docker layers, or caches on a runner between jobs.
5. Running CI jobs as `root` without need.
6. Giving jobs host Docker socket access.
7. Using Docker-in-Docker or privileged containers casually.
8. Mounting host directories into build containers.
9. Mounting `/`, `/var/run/docker.sock`, `/home`, `/root`, cloud credential directories, kubeconfig, or deployment config into jobs.
10. Using `--privileged`, host networking, host PID namespace, or host IPC namespace without isolation controls.
11. Running build containers with broad Linux capabilities.
12. Using outdated runner versions.
13. Not patching the runner OS.
14. Letting runners reach internal networks unnecessarily.
15. Allowing unrestricted outbound egress from CI.
16. Allowing CI jobs to call metadata endpoints and obtain cloud credentials.
17. Running production deployments and untrusted tests on the same runner fleet.
18. Using shared runners for regulated or sensitive workloads.
19. Letting runner labels/tags be too broad, such as `prod`, `linux`, or `deploy`, so unrelated jobs can land there.
20. Not pinning runner groups to specific repositories/projects.
21. Not rotating runner registration tokens.
22. Exposing runner registration commands or tokens in logs.
23. Allowing jobs to install persistent system services on self-hosted agents.
24. Allowing jobs to modify runner configuration.
25. Failing to autoscale or garbage-collect runners, causing resource exhaustion and cross-job residue.
26. Using `pull_policy: if-not-present` on shared runners where private images can remain on the host.
27. Running “trusted” and “untrusted” jobs in the same Kubernetes namespace.
28. Not constraining CPU, memory, disk, and network per job.
29. Leaving CI runners with access to production Kubernetes clusters by default.
30. Allowing nested virtualization or QEMU workloads from untrusted code.
31. Require CODEOWNERS or equivalent review for:
32. `.github/workflows/**`
33. `.gitlab-ci.yml`
34. `bitbucket-pipelines.yml`
35. CI includes/templates
36. build scripts
37. package manager scripts
38. deployment scripts
39. Dockerfiles
40. release scripts
41. Require at least one security/platform-owner approval for CI permission changes.
42. Block direct pushes to default/release branches.
43. Require signed commits where practical.
44. Protect release tags.
45. Prevent force-push on protected branches.
46. Require required checks before merge.
47. Require security checks before deployment.
48. Treat CI template repositories as production infrastructure.
49. Pin remote CI templates to immutable SHAs.
50. Add policy checks that reject:
51. unpinned actions/images/includes
52. broad token permissions
53. plaintext secrets
54. `pull_request_target` plus untrusted checkout
55. `privileged: true`
56. Docker socket mounts
57. `allow_failure` on security jobs
58. unrestricted production deploys
59. Lint CI YAML in PRs before merge.

## F. Dependency, action, pipe, plugin, and image best practices

1. **Pin third-party Actions to full commit SHA.**
2. **Pin Docker images by digest where practical.**
3. **Avoid `latest`.**
4. **Use verified/maintained actions and plugins.**
5. **Use Dependabot or equivalent for Actions and dependencies.**
6. **Use dependency review on PRs.**
7. **Use SBOMs and provenance for release artifacts.**
8. **Use lockfiles and deterministic install commands.**
9. **Avoid lifecycle script execution in untrusted contexts where possible.**
10. **Mirror critical tools internally or verify checksums/signatures.**
11. **Scan CI YAML.**
12. GitHub code scanning / CodeQL for workflow-relevant code,
13. OpenSSF Scorecard,
14. actionlint for GitHub Actions syntax,
15. zizmor or similar workflow security scanners,
16. GitLab CI linting and pipeline editor validation,
17. secret scanning,
18. IaC/container/dependency scanners.

## GitHub Actions

1. Set organization/repository default `GITHUB_TOKEN` to read-only.
2. Use job-level `permissions`.
3. Never use `write-all` for test jobs.
4. Do not allow Actions to approve pull requests.
5. Keep `id-token: write` only on jobs that actually exchange OIDC tokens.
6. Put production secrets behind protected environments with required reviewers.
7. Use reusable workflows carefully; avoid broad `secrets: inherit`.
8. Require CODEOWNERS review for workflow changes.

## GitHub Actions baseline

1. uses: actions/checkout@<full-length-commit-sha>
2. name: Install dependencies
3. name: Test

## GitHub Actions policy checks

1. `pull_request_target` plus any `run` step.
2. `pull_request_target` plus checkout of `github.event.pull_request.head.*`.
3. `workflow_run` that executes downloaded artifacts.
4. Missing top-level `permissions`.
5. `permissions: write-all`.
6. `contents: write` in PR/test jobs.
7. `id-token: write` without an allowlisted cloud-auth step.
8. `secrets: inherit` in reusable workflows without allowlisted caller/callee pairs.
9. `uses:` not pinned to full SHA for sensitive jobs.
10. `uses: */*@main`, `@master`, or `@latest`.
11. `docker://*:latest`.
12. `container.image` not pinned by digest or fixed version.
13. `curl | bash`, `wget | sh`, `iwr | iex`.
14. `${{ github.event.* }}` directly inside `run`.
15. `${{ github.head_ref }}` or branch names directly inside `run`.
16. `set -x` in jobs with secrets.
17. `env` or `printenv` in jobs with secrets.
18. `continue-on-error: true` in required jobs.
19. `|| true` after test/security commands.
20. Missing `timeout-minutes`.
21. Missing `concurrency` for PR workflows.
22. `runs-on: self-hosted` in PR workflows.
23. Generic self-hosted labels.
24. Artifact upload path is `.` or includes secret-prone directories.
25. Cache paths include secret-prone directories.
26. Cache restore in privileged jobs from untrusted branches.
27. Release/publish jobs triggered by unprotected branches/tags.
28. Workflow changes without CODEOWNER review.
29. Actions allowed to create or approve PRs.
30. Deployment job without environment protection.

## GitLab

1. Use **protected branches** for trusted code.
2. Use **protected tags** for releases.
3. Use **protected variables** for deployment credentials.
4. Use **protected runners** for deployment jobs.
5. Mask and hide sensitive variables.
6. Prefer external secrets managers over CI variables for high-value secrets.
7. Use protected environments and approvals for production.

## GitLab CI baseline

1. if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
2. if: '$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH'
3. when: never
4. test
5. build
6. deploy
7. package-lock.json
8. .npm/
9. npm ci --cache .npm --prefer-offline
10. dist/

## GitLab CI policy checks

1. Secrets in `variables`.
2. Unprotected variables used in deploy jobs.
3. Unprotected runners used for protected deployments.
4. Shell executor for untrusted projects.
5. `privileged = true` runner usage without exception.
6. Docker socket mounted into jobs.
7. `image: latest`.
8. Remote includes not pinned.
9. CI/CD components using `@latest`.
10. `allow_failure: true` on required jobs.
11. `when: always` on deploy jobs.
12. Deploy jobs without protected branch/tag restriction.
13. Deploy jobs without protected environment.
14. Artifacts include full workspace.
15. Artifacts include secrets or dotenv outputs from untrusted jobs.
16. Cache shared between protected and unprotected branches.
17. No cache key tied to lockfile/toolchain.
18. No job timeout.
19. Duplicate MR and branch pipelines.
20. No CI Lint validation.

## H. Use caching carefully

1. Cache dependencies, not secrets.
2. Cache package-manager directories, not entire home directories.
3. Include lockfile hash in cache keys.
4. Include OS and architecture where relevant.
5. Include language/runtime/toolchain version where relevant.
6. Separate caches by trust zone.
7. Avoid restoring untrusted caches in privileged jobs.
8. Keep caches small.
9. Track cache hit rate.
10. Invalidate caches after dependency/toolchain changes.
11. Do not use caches as artifact promotion.
12. Do not execute build outputs restored from untrusted caches.

## I. Bad observability and incident-response behavior

1. **No audit logs for workflow edits, runner registration, secret changes, token use, or deployment approvals.**
2. **No alerting on unusual CI behavior.**
3. **No provenance logs.**
4. **No retention for relevant logs.**
5. **Over-retention of sensitive logs.**
6. **No emergency kill switch for compromised workflows/runners.**
7. **No runbook for rotating CI secrets.**
8. **No way to quarantine compromised artifacts or packages.**

## I. Use artifacts as immutable promotion units

1. Build once.
2. Test that exact artifact.
3. Sign/attest that artifact.
4. Promote that artifact to staging.
5. Promote the same artifact to production.
6. Do not rebuild per environment.
7. Store checksums.
8. Verify checksums before deployment.
9. Keep artifact retention appropriate.
10. Do not include secrets in artifacts.
11. Do not execute untrusted artifacts in privileged jobs.
12. Use release versions that are immutable.

## J. Add CI governance and review controls

1. Require CODEOWNERS review for:
2. `.github/workflows/**`
3. `.gitlab-ci.yml`
4. `bitbucket-pipelines.yml`
5. Dockerfiles
6. build scripts
7. release scripts
8. dependency lockfiles
9. package-manager config
10. IaC/deployment manifests
11. Block direct pushes to protected branches.
12. Require signed commits/tags where appropriate.
13. Require status checks.
14. Prevent skipped CI from satisfying branch protection.
15. Require review for workflow changes from bots.
16. Run CI policy scanners on every CI-file change.
17. Run OpenSSF Scorecards or equivalent.
18. Use centralized reusable workflows/components.
19. Pin centralized components.
20. Version reusable workflow contracts.
21. Keep an exception registry.
22. Expire exceptions.
23. Review exceptions periodically.
24. Audit third-party CI dependencies.
25. Track CI incidents and near misses.
26. Train reviewers to treat CI config as privileged code.

## Performance & Caching

1. **Treating artifacts as trusted just because CI produced them.**
2. **Promoting artifacts without integrity checks.**
3. **Rebuilding separately for test and release.**
4. **No artifact retention policy.**
5. **Uploading entire workspaces as artifacts.**
6. **Using broad artifact names that collide across parallel jobs.**
7. **Using broad cache restore keys.**
8. **Sharing caches between protected and unprotected branches.**
9. **Caching build outputs instead of dependencies.**
10. **Cache dependencies, not secrets or build outputs.**
11. **Use lockfile-derived cache keys.**
12. **Separate caches by OS, architecture, language version, lockfile, and trust boundary.**
13. **Do not let fork PRs poison protected-branch caches.**
14. **Do not execute files restored from cache unless integrity is independently verified.**
15. **Use artifacts only for explicit outputs.**
16. **Name artifacts uniquely per job/shard.**
17. **Set artifact retention intentionally.**
18. **Exclude `.git`, credentials, `.env`, test traces with tokens, and package auth files from artifacts.**
19. **Sign release artifacts.**
20. **Generate and validate provenance.**
21. **Measure before optimizing.**
22. **Track pipeline duration, queue time, failure rate, retry rate, and slowest jobs.**
23. **Use a DAG/`needs` model instead of pure stage serialization.**
24. **Run fast checks first.**
25. **Separate PR, merge, nightly, release, and deployment pipelines.**
26. **Use path/change filters.**
27. **Parallelize independent tests.**
28. **Shard tests by historical runtime.**
29. **Use fail-fast for decisive parallel jobs.**
30. **Use timeouts on every job.**
31. **Cancel superseded runs.**
32. **Use resource groups/locks for shared environments.**
33. **Right-size runners.**
34. **Use lean base images.**
35. **Prebuild CI images for stable toolchains.**
36. **Cache package-manager directories carefully.**
37. **Avoid unnecessary artifact upload/download.**
38. **Retry only transient operations, with bounded backoff.**
39. **Quarantine flaky tests with ownership and deadlines.**
40. **Make failures reproducible locally.**
41. **Keep logs useful but not excessive.**
42. **Do not run full e2e/load/soak tests on every PR unless necessary.**
43. **Use nightly or scheduled jobs for expensive suites, but keep the release gate meaningful.**
44. Cache restored in privileged jobs from untrusted branches.
45. Cache keys based only on branch/ref.
46. Cache paths include home directory.
47. Cache paths include `.ssh`, `.aws`, `.docker`, `.kube`, `.npmrc`, `.pypirc`, `.gradle/gradle.properties`.
48. Uploading full workspace as artifact.
49. Downloading untrusted artifact and executing it.
50. Using artifact names from untrusted inputs.
51. Uploading test screenshots or traces from authenticated sessions.
52. Long artifact retention by default.
53. No artifact integrity checks.
54. No `concurrency` group to cancel obsolete PR runs.
55. No `timeout-minutes`.
56. `continue-on-error: true` on required tests.
57. Shell `|| true` after test commands.
58. Missing lockfile-aware dependency cache.
59. Rebuilding same Docker image in several jobs.
60. Large unbounded matrices.
61. No `fail-fast` tuning for matrix jobs.
62. No path filters for expensive jobs.
63. Full checkout history when not needed.
64. No test reports.
65. No slow-test tracking.
66. `artifacts: paths: ["."]`.
67. Artifacts include `.env`, `.ssh`, `.aws`, `.docker`, `.kube`, Terraform state, browser traces, or screenshots.
68. Artifacts never expire.
69. Caches include credentials.
70. Cache key too broad, such as only `$CI_COMMIT_REF_NAME`.
71. Cache key too narrow, causing constant misses.
72. Cache shared between protected and unprotected branches.
73. Artifact from MR pipeline consumed by deployment pipeline without validation.
74. `reports: dotenv` used to pass untrusted values into privileged jobs.
75. Build outputs from untrusted branches executed in protected jobs.
76. `allow_failure: true` on security gates or required tests.
77. `when: always` deploy jobs.
78. No job `timeout`.
79. No `interruptible: true` for superseded jobs.
80. Stage waterfall instead of `needs`.
81. No cache for dependencies.
82. Huge cache uploads.
83. No shallow clone tuning.
84. No CI Lint usage. GitLab’s docs state that `.gitlab-ci.yml` defines jobs and recommend validating syntax with CI Lint.
85. `image: latest`.
86. Unpinned public images.
87. Untrusted Pipes.
88. Remote installer scripts.
89. No dependency caches.
90. Caches too broad or too large.
91. Full clone for every step when not required.
92. Uploading full workspaces as artifacts.
93. Full expensive pipeline on every change.
94. No concurrency cancellation.
95. No dependency cache or bad cache key.
96. Full clone where shallow clone works.
97. Huge matrix with no cap.
98. Rebuilding same artifact repeatedly.
99. No test reports or slow-test tracking.
100. Unbounded artifact/cache retention.
101. No critical-path analysis.
102. Flaky tests normalized through reruns.
103. **Untrusted PRs run with no secrets and read-only permissions.**
104. **Privileged workflows never checkout or execute untrusted code.**
105. **All workflow tokens use least privilege.**
106. **Production secrets require protected environments and approvals.**
107. **Self-hosted runners are isolated by trust zone.**
108. **Public PRs never run on internal self-hosted runners.**
109. **Shell executors, Docker socket mounts, and privileged containers are avoided for untrusted jobs.**
110. **Third-party actions/components/pipes/images are pinned and reviewed.**
111. **Mutable refs like `latest`, `main`, and `master` are prohibited in sensitive jobs.**
112. **Remote installer scripts are verified or avoided.**
113. **Secrets are never stored in YAML, logs, caches, or artifacts.**
114. **OIDC/short-lived credentials replace static cloud keys where possible.**
115. **Caches are scoped by trust zone and keyed by lockfile/toolchain.**
116. **Artifacts are narrow, expire, and are never used to execute untrusted code in privileged contexts.**
117. **Build once, test once, promote the same artifact.**
118. **Fast checks run first.**
119. **Expensive checks use path filters, test selection, schedules, or release gates.**
120. **Pipelines use timeouts and cancellation of obsolete runs.**
121. **Flaky tests are tracked, owned, and fixed.**
122. **Security gates fail closed unless there is an explicit, expiring exception.**
123. **CI config changes require expert review.**
124. **CI policy is automatically scanned.**
125. **Runner hosts are patched, ephemeral where possible, and fully cleaned.**
126. **Network access from CI is denied by default and allowed by need.**
127. **Audit logs cover secrets, runners, variables, deploy keys, environments, webhooks, and workflow changes.**
128. **Incident response includes secret rotation, cache/artifact purge, runner reprovisioning, and package-token revocation.**
129. **Test reports, coverage, duration, queue time, flake rate, and cache hit rate are observable.**
130. **Release artifacts have provenance, SBOMs, signatures, and immutable versions.**
131. **Deployment jobs use protected branches/tags, manual gates where appropriate, and concurrency locks.**
132. **The CI system is treated as production infrastructure, not developer convenience glue.**

## Security & Secrets

1. Storing plaintext secrets in `.github/workflows/*.yml`, `.gitlab-ci.yml`, `bitbucket-pipelines.yml`, shell scripts, Dockerfiles, Makefiles, Helm values, Terraform files, or test fixtures.
2. Printing secrets with `echo`, `env`, `printenv`, `set -x`, `bash -x`, `docker inspect`, `kubectl describe`, verbose SDK logs, or failed command traces.
3. Uploading `.env`, `.npmrc`, `.pypirc`, `.netrc`, `.aws`, `.ssh`, kubeconfig, Docker config, Terraform state, cloud credential files, or entire home directories as artifacts.
4. Caching directories that contain credentials.
5. Passing secrets into third-party actions, pipes, containers, or plugins that do not need them.
6. Using long-lived cloud access keys instead of OIDC/federated short-lived tokens.
7. Using personal access tokens where app tokens, deploy tokens, job tokens, or OIDC would suffice.
8. Using a human’s personal SSH key for CI deployments.
9. Reusing the same credential across development, staging, production, package registries, cloud accounts, and repositories.
10. Giving every pipeline access to production credentials.
11. Giving every branch access to deployment variables.
12. Giving every job access to all secrets instead of scoping secrets by job/environment.
13. Leaving secrets available to documentation-only, lint-only, or unit-test-only jobs.
14. Exposing secrets to PRs from forks.
15. Exposing secrets to scheduled jobs whose YAML can be changed by less-trusted users.
16. Failing to rotate credentials after accidental log/artifact/cache exposure.
17. Relying only on masking while allowing malicious code to transform, encode, split, or exfiltrate secrets.
18. Storing structured JSON/XML/YAML secrets in ways that masking systems cannot reliably redact.
19. Letting CI variables override platform-defined variables.
20. Allowing users to supply pipeline variables at run time with high precedence.
21. Using environment variables for all secrets without understanding that child processes and build tools inherit them.
22. Storing deploy keys or SSH private keys as repository variables when the repo has many write-capable contributors.
23. Forgetting that deleted secrets remain in Git history if they were committed.
24. Failing to monitor secret reads, variable changes, and access-token creation.
25. Giving AI agents or external debugging tools raw CI logs containing secrets.
26. curl https://example.com/install.sh | bash
27. npm install
28. npm audit || true
29. Plaintext secrets in YAML.
30. Secrets embedded in shell commands.
31. Secrets passed as command-line arguments.
32. Secrets written to files without cleanup.
33. `printenv`, `env`, `set`, `export`, `set -x`, or debug traces in secret-bearing jobs.
34. Broad token permissions by default.
35. Job-level permissions not narrowed.
36. No explicit read-only default.
37. CI job can push commits.
38. CI job can modify workflow files.
39. CI job can publish packages without branch protection.
40. CI job can deploy from non-protected branches.
41. CI job can create tags/releases from untrusted branches.
42. Unpinned actions/includes/pipes/images.
43. Mutable tags such as `main`, `master`, `latest`, `stable`, `edge`, `v1`, or `v2`.
44. Remote includes without SHA/integrity.
45. `curl | bash`.
46. Downloading executable scripts without hash/signature verification.
47. Running package lifecycle scripts with secrets present.
48. Running untrusted tests with secrets present.
49. `allow_failure`, `continue-on-error`, `|| true`, or `exit 0` on security gates.
50. Security scans run but do not block merge/deploy.
51. No timeout.
52. No cancellation/concurrency control.
53. No path filters or workflow rules.
54. Duplicate push and PR/MR pipelines.
55. Overbroad matrix.
56. No cache, or cache is too broad.
57. Cache contains secrets.
58. Cache shared across trust boundaries.
59. Artifact captures entire workspace.
60. Artifact captures untracked files.
61. Artifact retention is infinite or excessive.
62. Artifact access is unrestricted.
63. Job downloads all previous artifacts by default.
64. Deployment job rebuilds instead of promoting tested artifact.
65. Deployment job uses mutable image tag.
66. No signature/digest verification.
67. Global `before_script` does heavy work in every job.
68. Global environment variables override secure platform defaults.
69. Dynamic shell with `eval`.
70. Unquoted shell variables.
71. PR title/body/branch/commit message used directly in shell.
72. Issue comments used directly in shell.
73. Branch names used as environment names without validation.
74. Branch names used as cloud resource names without validation.
75. Branch names used as file paths without validation.
76. Generated YAML accepted from untrusted code.
77. Child pipelines generated from untrusted artifacts.
78. External services invoked with broad secrets.
79. Deployment environments selected by untrusted input.
80. Manual jobs available to too many users.
81. Production jobs lack environment approvals.
82. Scheduled jobs have production secrets.
83. Tag pipelines deploy without protected tags.
84. Fork pipelines can reach secrets.
85. Self-hosted runners used for untrusted jobs.
86. Privileged Docker used unnecessarily.
87. Docker socket mounted.
88. Host directories mounted.
89. Job runs as root unnecessarily.
90. No egress restrictions for secret-bearing jobs.
91. No runner labels/tags separating trust levels.
92. No CODEOWNERS or equivalent review requirement for CI config.
93. CI config can be changed by the same users it is meant to constrain.
94. AI agent allowed to edit CI YAML and merge without human review.
95. CI agent allowed to disable tests or scans.
96. CI agent allowed to increase its own permissions.
97. CI agent allowed to add new third-party actions/includes/pipes.
98. CI agent allowed to add new secrets or expose existing ones.
99. Never put secrets in YAML.
100. Never put secrets in the repository.
101. Never put secrets in generated files committed by CI.
102. Never expose production secrets to PR/fork pipelines.
103. Use environment-scoped secrets.
104. Use protected secrets only on protected branches/tags.
105. Use hidden/masked/protected variables where supported.
106. Use unique secret values so masking can work reliably.
107. Avoid structured secrets when the platform warns masking may be unreliable.
108. Keep secrets out of command-line arguments.
109. Keep secrets out of artifacts.
110. Keep secrets out of caches.
111. Keep secrets out of test snapshots.
112. Keep secrets out of Docker image layers.
113. Keep secrets out of crash dumps and coverage reports.
114. Disable shell tracing in secret-bearing jobs.
115. Use secret scanners on code, history, logs, and artifacts.
116. Rotate secrets after any suspected exposure.
117. Prefer vault/secret-manager retrieval with short-lived identity.
118. Give each job only the secrets it needs.
119. Do not pass secrets to third-party actions/pipes/includes unless reviewed.
120. For AI agents, redact logs before sending them to external tools.
121. Treat build output and test failure messages as potentially sensitive.
122. Block `printenv`, `env`, and `set -x` in secret-bearing jobs via policy.
123. Use path filters so docs-only changes do not run full integration suites.
124. Use separate fast pre-merge and deep post-merge pipelines.
125. Run lint, formatting, type checks, and unit tests early.
126. Run expensive integration tests only when relevant paths change or before merge/release.
127. Use DAG dependencies instead of stage-only sequencing.
128. Use explicit `needs`.
129. Download only required artifacts.
130. Use test sharding.
131. Use fail-fast for large matrices.
132. Cache dependencies based on lockfiles.
133. Do not cache secrets.
134. Separate caches across trust boundaries.
135. Keep cache size smaller than the time it saves.
136. Prebuild CI images with stable tooling.
137. Avoid installing OS packages in every job.
138. Use immutable toolchain images.
139. Use timeouts for every job.
140. Auto-cancel superseded non-deployment jobs.
141. Never auto-cancel in-progress production deployments unless explicitly safe.
142. Mark only safe jobs interruptible.
143. Avoid retrying flaky tests without tracking.
144. Track queue time, runtime, cache hit rate, artifact size, and cost.
145. Keep artifacts small and short-lived.
146. Upload heavy debug artifacts only on failure.
147. Use external artifact storage for large/long-lived outputs.
148. Split monorepo pipelines by affected project.
149. Avoid broad matrices by default; run full compatibility on release branches or schedules.
150. Run security checks early enough to fail before expensive packaging.
151. Prefer deterministic tests over sleeps/retries.
152. Make AI agents batch fixes instead of pushing one commit per tiny change.
153. **Printing secrets to logs.**
154. **Logging transformed secrets.**
155. **Putting secrets directly in YAML.**
156. **Using CI/CD variables as a full secret manager.**
157. **Making secrets available to all jobs.**
158. **Using `secrets: inherit` casually in reusable workflows.**
159. **Passing secrets to child pipelines or shared pipeline variables that log plain text.**
160. **Assuming “secured variable” means safe from malicious code.**
161. **Caching secrets.**
162. **Publishing secrets in artifacts.**
163. **Leaking secrets through AI-agent logs.**
164. **Storing production secrets in test environments.**
165. **Using the same secret across dev, staging, CI, and production.**
166. **Not rotating secrets after CI compromise.**
167. **`pull_request_target` plus checkout of PR head.**
168. uses: actions/checkout@v4
169. run: npm test
170. **`workflow_run` executing untrusted artifacts.**
171. uses: actions/download-artifact@v4
172. run: ./artifact/test-output/script.sh
173. **No explicit permissions.**
174. **`permissions: write-all`.**
175. **Third-party actions pinned to mutable tags.**
176. uses: some-org/some-action@main
177. uses: some-org/some-action@v1
178. **Untrusted expression injection in `run:`.**
179. run: echo "PR title: ${{ github.event.pull_request.title }}"
180. **Using issue comments as commands without authorization.**
181. run: deploy ${{ github.event.comment.body }}
182. **Passing `workflow_dispatch` inputs to shell.**
183. run: ${{ github.event.inputs.command }}
184. **Using secrets in top-level `env:`.**
185. **`secrets: inherit` in reusable workflows without strict boundaries.**
186. **Caching secret-bearing directories.**
187. uses: actions/cache@v4
188. **Broad cache keys and restore keys.**
189. **No `timeout-minutes`.**
190. **No `concurrency` for expensive or deployment workflows.**
191. **No branch/path filtering.**
192. **Security checks marked non-blocking.**
193. **Auto-committing from CI to protected branches.**
194. **Auto-approving PRs from CI.**
195. **Release jobs triggered by tags that anyone can create.**
196. **Publishing packages from non-protected branches.**
197. **Using self-hosted runners with broad labels.**
198. **Mounting Docker socket or privileged containers in test jobs.**
199. **Using `docker://image:latest` or unpinned container images.**
200. **Uploading entire workspace as artifact.**
201. **Downloading artifacts from PR jobs into release jobs.**
202. **No CODEOWNERS protection for workflow changes.**
203. **Not using Dependabot/dependency review for Actions.**
204. **Not using workflow/code scanning.**
205. **Secrets hard-coded in YAML.**
206. **Unquoted variables causing YAML parsing surprises.**
207. **Pipeline variables overriding safer values.**
208. **Unprotected variables used in MR pipelines.**
209. **Protected runners accepting untrusted jobs.**
210. **Sensitive runners accepting untagged jobs.**
211. **Shell executor for untrusted builds.**
212. **Privileged Docker-in-Docker for untrusted branches.**
213. docker:dind
214. **Using `image: latest`.**
215. **Remote includes from mutable refs.**
216. remote: https://example.com/template.yml
217. **Security scans allowed to fail.**
218. **Deploy jobs missing `environment`, approvals, or protected branch rules.**
219. **No `resource_group` for shared deployment targets.**
220. **Using `CI_JOB_TOKEN` broadly across projects.**
221. **Allowing `CI_JOB_TOKEN` to push without strict controls.**
222. **Shared caches across trust boundaries.**
223. **Cache keys too broad.**
224. node_modules/
225. **Artifacts include secrets.**
226. **Artifacts have no expiry.**
227. **Dynamic child pipelines using unvalidated inputs.**
228. **Passing secrets through child pipeline variables.**
229. **`rules:` that accidentally run deploys on merge requests.**
230. **Old `only/except` rules with ambiguous behavior.**
231. **No `needs:` DAG; everything waits stage-by-stage.**
232. **No `interruptible: true` for obsolete branch pipelines.**
233. **No `timeout`.**
234. **No separation of PR/MR tests, protected-branch tests, release, and deploy workflows.**
235. **Secrets in plain pipeline variables.**
236. **Personal SSH keys as repository variables.**
237. **Assuming secured variables are safe from all writers.**
238. **Using repository variables where deployment variables should be used.**
239. **Passing secrets to child pipelines.**
240. **Using secured variables in YAML templating.**
241. **No `known_hosts` verification for SSH.**
242. **Broad repository access tokens.**
243. **Deployment steps without environment permissions.**
244. **Parallel artifacts with conflicting names.**
245. **No fail-fast on parallel groups.**
246. **No parallelization for independent test suites.**
247. **Improper service memory sizing.**
248. **Using `latest` images.**
249. **Installing all dependencies in every step.**
250. **No dependency cache or poor cache scoping.**
251. **Deployment reruns relying on expired artifacts.**
252. **Threat-model CI as privileged remote code execution.**
253. **Separate workflows by trust level.**
254. untrusted PR/MR tests,
255. trusted branch tests,
256. release builds,
257. deployments,
258. scheduled maintenance,
259. dependency updates,
260. security scans,
261. AI-agent automation.
262. **Never mix untrusted code execution and privileged credentials in the same job.**
263. **Build once, test once, promote the same artifact.**
264. **Use protected environments for deployment.**
265. **Keep the most sensitive secrets outside the CI platform.**
266. **Use OIDC or workload identity for cloud auth.**
267. **Use protected/masked/hidden variables where supported.**
268. **Avoid structured secrets when masking is weak.**
269. **Never print all environment variables.**
270. **Disable shell tracing around secrets.**
271. **Do not store secrets in caches or artifacts.**
272. **Do not pass secrets through Bitbucket shared pipeline variables or child pipeline inputs.**
273. **Use file-type variables for tools that expect files.**
274. **Rotate secrets after suspicious CI behavior.**
275. **Use secret scanning on commits, logs where possible, and artifacts where feasible.**
276. **Use separate low-privilege test credentials.**
277. **Never use personal SSH keys in CI.**
278. Top-level `env:` containing secrets.
279. Secrets passed to every job.
280. Secrets passed to PR jobs.
281. Secrets exposed to matrix jobs that do not need them.
282. `secrets: inherit` in reusable workflows without strict trust boundaries.
283. Reusable workflows that accept arbitrary commands plus inherited secrets.
284. Printing env variables.
285. Uploading logs/artifacts that include `.env`, `.npmrc`, `.pypirc`, `.docker`, `.kube`, or coverage traces with headers.
286. Using production credentials in test jobs.
287. Storing cloud keys instead of using OIDC with narrow claims.
288. Secrets in `variables:`.
289. Secrets exported in `before_script`.
290. Secrets committed into `.gitlab-ci.yml`.
291. Unmasked variables.
292. Unhidden variables.
293. Unprotected deployment variables.
294. Variables available to merge-request pipelines from untrusted branches.
295. Deployment credentials available outside protected branches/tags.
296. Using CI variables where an external secrets manager should be used.
297. Passing secrets through artifacts, dotenv reports, caches, or child-pipeline variables.
298. Secrets committed directly in YAML.
299. Using unsecured repository/workspace variables for secrets.
300. Passing secrets through child-pipeline input variables that are logged in plain text.
301. Assuming all variables are safe to use in YAML templating.
302. Printing env vars.
303. Using production secrets in PR validation.
304. Sharing one deployment variable across all environments.
305. Long-lived cloud keys instead of OIDC where available.
306. Storing package registry tokens in artifacts or caches.
307. Using secured variables in places Bitbucket does not support them.
308. Prefer OIDC/federated identity over static cloud keys.
309. Scope cloud trust policies to:
310. specific repository
311. specific branch/tag
312. specific workflow
313. specific environment
314. specific audience
315. specific subject claim
316. Use short-lived credentials.
317. Use different credentials for build, publish, staging, and production.
318. Put production secrets behind approvals.
319. Never expose secrets to PR/fork jobs.
320. Never put secrets in global `env`.
321. Never print environment variables.
322. Avoid structured multi-line secrets where possible.
323. Rotate secrets regularly.
324. Rotate immediately after suspicious CI activity.
325. Keep artifact/log retention minimal for secret-bearing jobs.
326. Scan for secrets in code, logs, and artifacts.
327. Do one-time historic secret scans, then incremental scanning for new commits.
328. Keep package-publishing tokens narrowly scoped.
329. Use read-only registry tokens for dependency installation.
330. Use separate signing infrastructure for high-value release signing.
331. Keep production data out of CI tests.
332. Mock or synthesize sensitive data.

## Testing & Coverage

1. Unit tests depend on live external services.
2. Unit tests require production-like infrastructure.
3. Integration tests run for every trivial change.
4. End-to-end tests run serially when they could be sharded.
5. Tests use fixed sleeps instead of readiness probes.
6. Tests poll without timeouts.
7. Tests share mutable global state.
8. Tests depend on wall-clock time, random order, or uncontrolled randomness.
9. Flaky tests are ignored instead of quarantined and fixed.
10. Flaky tests are retried until green, hiding real defects.
11. Security tests are marked optional.
12. Coverage thresholds are gamed by superficial tests.
13. Performance benchmarks run on noisy shared runners and are treated as precise.
14. Test containers are rebuilt for every job.
15. Test fixtures are huge and copied repeatedly.
16. The same test suite is executed in multiple jobs accidentally.
17. No smoke test exists for fast feedback.
18. No impact analysis or test selection exists for large monorepos.
19. No timing data is collected per test.
20. No ownership exists for slow or flaky tests.
21. AI agents add tests that assert implementation details but do not catch regressions.
22. AI agents delete or skip failing tests to make CI pass.
23. AI agents weaken assertions to make tests green.
24. AI agents add broad sleeps and retries instead of fixing synchronization.
25. AI agents rewrite CI to hide failures instead of fixing root cause.
26. **Skipping security tests on PRs.**
27. **Marking security scans as advisory forever.**
28. **Running security scanners after deployment.**
29. **Testing with production data.**
30. **Using live production services in tests.**
31. **Test fixtures containing real secrets.**
32. **Snapshot tests capturing secrets.**
33. **Browser/end-to-end tests saving auth tokens in screenshots, videos, traces, HAR files, or artifacts.**
34. **Not isolating test tenants/accounts.**
35. **Granting test credentials admin permissions.**
36. **Allowing tests to create public cloud resources without cleanup.**
37. **Using CI to run destructive tests against shared environments.**
38. **Letting tests mutate shared package registries, tags, images, or release channels.**
39. **Running fuzzers or load tests without resource quotas.**
40. **Flaky tests left in the main gate.**
41. **Retries hiding real failures.**
42. **No quarantine process for flaky tests.**
43. **No test timing data.**
44. **No test sharding by historical duration.**
45. **Naive alphabetical sharding that creates imbalanced workers.**
46. **Stateful tests that depend on order.**
47. **Tests depending on wall-clock time, time zones, random seeds, external APIs, or shared mutable environments.**
48. **No deterministic seed capture for randomized tests.**
49. **Running e2e tests against unstable shared environments.**
50. **No service readiness checks.**
51. **Fixed sleeps instead of health checks.**
52. **Integration tests that leak resources and slow later tests.**
53. **Using production-size datasets for ordinary PR tests.**
54. **No distinction between PR, merge, nightly, release, and soak test suites.**
55. **No local reproduction command.**
56. **Tests produce massive logs by default.**
57. **Coverage collection on every job even when only needed once.**
58. **Uploading coverage from every shard without merging discipline.**
59. **No performance budget or regression gate for critical paths.**
60. **Benchmark jobs running on noisy shared runners without baselines.**
61. **Benchmark results treated as deterministic when runner noise dominates.**
62. uses: actions/checkout@<full-length-commit-sha>
63. uses: actions/setup-node@<full-length-commit-sha>
64. run: npm ci
65. run: npm test
66. if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
67. untrusted-linux
68. package-lock.json
69. .npm/
70. npm ci --cache .npm --prefer-offline
71. **Preflight**
72. **Fast correctness**
73. **Build verification**
74. **Integration**
75. **End-to-end**
76. **Security**
77. **Release qualification**
78. **Deployment validation**
79. Keep fast PR feedback fast.
80. Fail early on cheap checks.
81. Use path-aware workflows.
82. Use dependency caches keyed by lockfile, OS, architecture, and toolchain.
83. Use shallow clones unless full history is required.
84. Use prebuilt CI images for stable toolchains.
85. Build once, test the same artifact, and promote that artifact.
86. Parallelize independent tests.
87. Do not parallelize tests that share mutable state.
88. Use service containers with unique per-job credentials/databases.
89. Track slow tests.
90. Track flaky tests.
91. Quarantine flaky tests only with owner, ticket, and expiry.
92. Rerun suspected flaky failures for classification, not silent success.
93. Collect JUnit or equivalent reports.
94. Fail when no tests are discovered.
95. Fail when reports are missing.
96. Enforce timeouts.
97. Use concurrency cancellation for obsolete PR runs.
98. Measure queue time, setup time, execution time, cache hit rate, and artifact upload/download time.
99. name: Checkout
100. name: Setup Node
101. name: Install
102. name: Test
103. name: Upload test report
104. if: $CI_MERGE_REQUEST_IID
105. if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
106. if: $CI_COMMIT_TAG
107. npm test -- --ci
108. npm run build
109. dist/

## References

[1] https://cheatsheetseries.owasp.org/cheatsheets/CI_CD_Security_Cheat_Sheet.html "CI CD Security - OWASP Cheat Sheet Series"
[2] https://csrc.nist.gov/pubs/sp/800/204/d/final?utm_source=chatgpt.com "SP 800-204D, Strategies for the Integration of Software Supply Chain Security in DevSecOps CI/CD Pipelines | CSRC"
[3] https://securitylab.github.com/resources/github-actions-preventing-pwn-requests/ "Keeping your GitHub Actions and workflows secure Part 1: Preventing pwn requests | GitHub Security Lab"
[4] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-04-Poisoned-Pipeline-Execution "CICD-SEC-4: Poisoned Pipeline Execution (PPE) | OWASP Foundation"
[5] https://docs.github.com/en/actions/reference/security/secure-use "Secure use reference - GitHub Docs"
[6] https://docs.gitlab.com/ci/variables/ "CI/CD variables | GitLab Docs"
[7] https://support.atlassian.com/bitbucket-cloud/docs/variables-and-secrets/ "Variables and secrets | Bitbucket Cloud | Atlassian Support"
[8] https://www.reddit.com/r/devops/comments/1aqlx7n/how_do_you_manage_secrets_in_your_cicd_pipeline/?utm_source=chatgpt.com "How do you manage secrets in your CI/CD pipeline?"
[9] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-02-Inadequate-Identity-And-Access-Management "CICD-SEC-2: Inadequate Identity and Access Management | OWASP Foundation"
[10] https://docs.gitlab.com/runner/security/ "Security for self-managed runners | GitLab Docs"
[11] https://support.atlassian.com/bitbucket-cloud/docs/adding-a-new-runner-in-bitbucket/ "Adding a new runner in Bitbucket | Bitbucket Cloud | Atlassian Support"
[12] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-03-Dependency-Chain-Abuse "CICD-SEC-3: Dependency Chain Abuse | OWASP Foundation"
[13] https://docs.gitlab.com/ci/yaml/ "CI/CD YAML syntax reference | GitLab Docs"
[14] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-08-Ungoverned-Usage-of-3rd-Party-Services "CICD-SEC-8: Ungoverned Usage of 3rd Party Services | OWASP Foundation"
[15] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-09-Improper-Artifact-Integrity-Validation "CICD-SEC-9: Improper Artifact Integrity Validation | OWASP Foundation"
[16] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-01-Insufficient-Flow-Control-Mechanisms "CICD-SEC-1: Insufficient Flow Control Mechanisms | OWASP Foundation"
[17] https://docs.gitlab.com/ci/environments/deployment_safety/ "Deployment safety | GitLab Docs"
[18] https://support.atlassian.com/bitbucket-cloud/docs/use-branch-permissions/?utm_source=chatgpt.com "Use branch permissions | Bitbucket Cloud"
[19] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-10-Insufficient-Logging-And-Visibility?utm_source=chatgpt.com "CICD-SEC-10: Insufficient Logging and Visibility"
[20] https://support.atlassian.com/bitbucket-cloud/docs/integrate-pipelines-with-resource-servers-using-oidc/ "Integrate Pipelines with resource servers using OIDC | Bitbucket Cloud | Atlassian Support"
[21] https://docs.gitlab.com/ci/jobs/job_artifacts/ "Job artifacts | GitLab Docs"
[22] https://support.atlassian.com/bitbucket-cloud/docs/cache-dependencies/ "Caches | Bitbucket Cloud | Atlassian Support"
[23] https://support.atlassian.com/bitbucket-cloud/docs/use-artifacts-in-steps/ "Pipeline artifacts | Bitbucket Cloud | Atlassian Support"
[24] https://cloud.google.com/blog/topics/threat-intelligence/bitbucket-pipeline-leaking-secrets "Holes in Your Bitbucket: Why Your CI/CD Pipeline Is Leaking Secrets | Google Cloud Blog"
[25] https://slsa.dev/?utm_source=chatgpt.com "SLSA • Supply-chain Levels for Software Artifacts"
[26] https://github.com/ossf/scorecard?utm_source=chatgpt.com "OpenSSF Scorecard - Security health metrics for Open ..."
[27] https://arxiv.org/abs/2401.17606?utm_source=chatgpt.com "Ambush from All Sides: Understanding Security Threats in Open-Source Software CI/CD Pipelines"
[28] https://owasp.org/www-project-top-10-ci-cd-security-risks/?utm_source=chatgpt.com "OWASP Top 10 CI/CD Security Risks"
[29] https://www.usenix.org/conference/usenixsecurity22/presentation/koishybayev "Characterizing the Security of Github CI Workflows | USENIX"
[30] https://docs.github.com/en/enterprise-cloud%40latest/actions/reference/security/secure-use "Secure use reference - GitHub Enterprise Cloud Docs"
[31] https://support.atlassian.com/bitbucket-cloud/kb/recommended-practices-for-managing-secret-data-in-bitbucket-pipelines/ "Recommended practices for managing secret data in Bitbucket Pipelines | Bitbucket Cloud | Atlassian Support"
[32] https://docs.gitlab.com/ci/runners/configure_runners/ "Configuring runners | GitLab Docs"
[33] https://docs.gitlab.com/ci/jobs/ci_job_token/ "CI/CD job token | GitLab Docs"
[34] https://www.jenkins.io/doc/book/using/using-credentials/ "
[35] https://github.blog/security/supply-chain-security/four-tips-to-keep-your-github-actions-workflows-secure/ "How to secure GitHub Actions workflows: 4 tips to handle untrusted input and tighten permissions - The GitHub Blog"
[36] https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-01-Insufficient-Flow-Control-Mechanisms?utm_source=chatgpt.com "CICD-SEC-1: Insufficient Flow Control Mechanisms"
[37] https://slsa.dev/spec/v1.2/build-requirements "SLSA • Build: Requirements for producing artifacts"
[38] https://www.jenkins.io/doc/book/pipeline/jenkinsfile/ "
[39] https://docs.gitlab.com/ci/pipeline_security/ "Pipeline security | GitLab Docs"
[40] https://arxiv.org/html/2601.14455v1 "Unpacking Security Scanners for GitHub Actions Workflows"
[41] https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching "Dependency caching reference - GitHub Docs"
[42] https://arxiv.org/abs/2604.03070?utm_source=chatgpt.com "Credential Leakage in LLM Agent Skills: A Large-Scale Empirical Study"
[43] https://docs.gitlab.com/ci/docker/using_docker_build/ "Use Docker to build Docker images | GitLab Docs"
[44] https://docs.github.com/actions/using-workflows/workflow-syntax-for-github-actions?utm_source=chatgpt.com "Workflow syntax for GitHub Actions"
[45] https://openssf.org/blog/2024/08/12/mitigating-attack-vectors-in-github-workflows/ "Mitigating Attack Vectors in GitHub Workflows – Open Source Security Foundation"
[46] https://www.usenix.org/conference/usenixsecurity23/presentation/muralee "ARGUS: A Framework for Staged Static Taint Analysis of GitHub Workflows and Actions | USENIX"
[47] https://support.atlassian.com/bitbucket-cloud/docs/parallel-step-options/ "Parallel step options | Bitbucket Cloud | Atlassian Support"
[48] https://docs.gitlab.com/ci/caching/ "Caching in GitLab CI/CD | GitLab Docs"
[49] https://docs.gitlab.com/ci/pipelines/pipeline_efficiency/ "Pipeline efficiency | GitLab Docs"
[50] https://docs.github.com/enterprise-cloud%40latest/actions/using-jobs/using-concurrency?utm_source=chatgpt.com "Control the concurrency of workflows and jobs"
[51] https://support.atlassian.com/bitbucket-cloud/docs/databases-and-service-containers/ "Databases and service containers | Bitbucket Cloud | Atlassian Support"
[52] https://docs.gitlab.com/ci/resource_groups/ "Resource group | GitLab Docs"
[53] https://support.atlassian.com/bitbucket-cloud/docs/set-up-runners-for-linux-shell/ "Set up runners for Linux Shell | Bitbucket Cloud | Atlassian Support"
[54] https://support.atlassian.com/bitbucket-cloud/docs/repository-access-tokens/ "Access tokens for a repository | Bitbucket Cloud | Atlassian Support"
[55] https://support.atlassian.com/bitbucket-cloud/docs/stage-options/ "Stage options | Bitbucket Cloud | Atlassian Support"
[56] https://github.com/takezoe/gitbucket-ci-plugin?utm_source=chatgpt.com "takezoe/gitbucket-ci-plugin"
[57] https://plugins.jenkins.io/gitbucket?utm_source=chatgpt.com "GitBucket | Jenkins plugin"
[58] https://www.reddit.com/r/github/comments/1alewms/it_is_safe_to_use_secret_in_action_in_a_public/?utm_source=chatgpt.com "It is safe to use secret in Action in a public repo? : r/github"
[59] https://www.reddit.com/r/github/comments/1rmh3n8/someone_automated_the_process_of_scanning_every/?utm_source=chatgpt.com "Someone automated the process of scanning every public ..."
[60] https://x.com/ellen_in_sf/status/2039198772656496867?utm_source=chatgpt.com "let's guess how many @ycombinator P26 will pivot to AI security"
[61] https://scorecard.dev/ "OpenSSF Scorecard"
[62] https://cheatsheetseries.owasp.org/cheatsheets/CI_CD_Security_Cheat_Sheet.html?utm_source=chatgpt.com "CI CD Security - OWASP Cheat Sheet Series"
[63] https://docs.github.com/en/actions/concepts/security/script-injections "Script injections - GitHub Docs"
[64] https://docs.gitlab.com/runner/executors/shell/ "The Shell executor | GitLab Docs"
[65] https://github.com/takezoe/gitbucket-ci-plugin "GitHub - takezoe/gitbucket-ci-plugin: GitBucket plug-in that adds simple CI ability to GitBucket. · GitHub"
[66] https://docs.gitlab.com/ci/components/ "CI/CD components | GitLab Docs"
[67] https://about.codecov.io/security-update/ "Bash Uploader Security Update - Codecov"
[68] https://github.com/advisories/ghsa-mrrh-fwg8-r2c3 "tj-actions changed-files through 45.0.7 allows remote attackers to discover secrets by reading actions logs. · CVE-2025-30066 · GitHub Advisory Database · GitHub"
[69] https://support.atlassian.com/bitbucket-cloud/docs/bitbucket-deployment-guidelines/ "Bitbucket deployment guidelines | Bitbucket Cloud | Atlassian Support"
[70] https://docs.gitlab.com/runner/configuration/advanced-configuration/ "Advanced configuration | GitLab Docs"
[71] https://support.atlassian.com/bitbucket-cloud/docs/test-reporting-in-pipelines/?utm_source=chatgpt.com "Get started with tests in Pipelines | Bitbucket Cloud"
[72] https://testing.googleblog.com/2016/05/flaky-tests-at-google-and-how-we.html?utm_source=chatgpt.com "Flaky Tests at Google and How We Mitigate Them"
[73] https://testing.googleblog.com/2017/04/where-do-our-flaky-tests-come-from.html?utm_source=chatgpt.com "Where do our flaky tests come from?"
[74] https://www.sciencedirect.com/science/article/pii/S0164121223002327?utm_source=chatgpt.com "Test flakiness' causes, detection, impact and responses"
[75] https://docs.gitlab.com/user/application_security/secret_detection/pipeline/ "Pipeline secret detection | GitLab Docs"
[76] https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching?utm_source=chatgpt.com "Dependency caching reference"
[77] https://support.atlassian.com/bitbucket-cloud/docs/git-clone-behavior/?utm_source=chatgpt.com "Git clone behavior | Bitbucket Cloud"
[78] https://support.atlassian.com/bitbucket-cloud/docs/pipeline-start-conditions/?utm_source=chatgpt.com "Pipeline start conditions | Bitbucket Cloud"
[79] https://www.reddit.com/r/devops/comments/1co3k0c/how_do_i_decide_between_gitlab_ci_bitbucket/ "How do I decide between Gitlab CI, Bitbucket Pipelines, Jenkins, and Azure Pipelines? : r/devops"
[80] https://slsa.dev/threats "SLSA • Threats & mitigations"
[81] https://docs.gitlab.com/ci/pipelines/ "CI/CD pipelines | GitLab Docs"
[82] https://support.atlassian.com/bitbucket-cloud/docs/cache-dependencies/?utm_source=chatgpt.com "Caches | Bitbucket Cloud"
[83] https://martinfowler.com/articles/continuousIntegration.html "Continuous Integration"
