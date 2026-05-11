# Bad RUST Behavior: Comprehensive Guide

This document organizes the worst RUST behaviors that are inexcusable in production.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core RUST best practices:

- **Embrace the borrow checker**: Prefer references and lifetimes over cloning or `Rc`/`Arc` when ownership is clear.
- **Use `Result` and `Option` strictly**: Avoid `.unwrap()` and `.expect()` in production code. Handle errors gracefully using `?`.
- **Leverage the type system**: Use newtypes to encapsulate and validate domain invariants.
- **Keep `unsafe` minimal and isolated**: Wrap unsafe blocks in safe abstractions with rigorous testing.
- **Practice modular architecture**: Split large crates into workspace members with clear boundaries.

## Borrow-checker bypasses that lie

1. `mem::transmute` to extend a lifetime.
2. Turning a borrowed reference into `'static` because an API “wanted static.”
3. Using `Box::leak` as a lazy lifetime fix in long-running code.
4. Using raw pointers to create aliasing that references would reject.
5. Using `Rc<RefCell<_>>` or `Arc<Mutex<_>>` merely to dodge ownership design.
6. Creating self-referential structs with raw pointers without a proven pinning/drop story.
7. Storing references into containers while pretending moves cannot happen.
8. Using `unsafe` because “the borrow checker is too strict” without proving the compiler-rejected case is actually valid.

## Crypto

1. Rolling your own crypto.
2. Using non-cryptographic RNGs for keys, nonces, salts, IVs, session IDs, reset tokens, CSRF tokens, or capability URLs.
3. Reusing nonces in AEAD modes.
4. Fixed IVs/nonces.
5. Hardcoded keys.
6. Unauthenticated encryption.
7. Disabling TLS certificate validation.
8. Accepting invalid certificates in production.
9. Inventing password hashing.
10. Using fast hashes for passwords.
11. Using deprecated algorithms because the code compiled.
12. Treating crypto examples as production architecture.
13. Calling `unwrap` on cryptographic operations involving attacker-controlled ciphertext, because that can turn malformed input into denial of service.

## False thread-safety

1. `unsafe impl Send` or `unsafe impl Sync` because “it seems fine.”
2. Sharing non-thread-safe FFI handles across threads.
3. Wrapping a raw pointer in a newtype and declaring it `Send`/`Sync` without proving the pointee, ownership, and foreign library are thread-safe.
4. Using atomics as decoration without a memory-ordering argument.
5. Mixing atomics and non-atomic access to the same memory.
6. Assuming “single-threaded today” makes a type safe to mark `Send`.
7. Ignoring reentrancy, callbacks, signal handlers, or interrupt contexts.
8. `unsafe` used as a compiler-silencer.
9. `unsafe` blocks with no safety comment.
10. Safety comments that merely restate the code: “SAFETY: this is safe.”
11. Safety comments that say “tested,” “works on my machine,” or “AI generated this.”
12. Huge unsafe blocks that mix unsafe operations with ordinary logic.
13. Unsafe code whose invariants are spread across unrelated modules with no documentation.
14. Unsafe code in public APIs without a `# Safety` section.
15. `unsafe fn` bodies that perform unsafe operations without explicit local unsafe blocks.
16. `unsafe trait` implementations without proving the trait contract.
17. Hiding unsafe inside macros, build scripts, proc macros, or generated files.
18. Adding `#![allow(unsafe_code)]` or broad lint suppressions instead of reviewing each unsafe site.
19. Copying unsafe snippets from StackOverflow, GitHub, Reddit, or an LLM without reading the docs for the exact API and version.
20. Treating Miri, tests, fuzzing, or “it ran once” as proof of soundness. They are evidence, not proof.
21. Accepting unsafe code from a dependency without checking whether the crate has active maintenance, advisories, tests, and a credible safety story.
22. Wrong `extern` ABI.
23. Calling C functions with the wrong signature.
24. Passing Rust references to C that can store them beyond their lifetime.
25. Passing `&mut T` to C while Rust aliases still exist.
26. Passing Rust structs across FFI without `#[repr(C)]` or an explicit layout guarantee.
27. Passing enums, trait objects, slices, `String`, `Vec`, `Option<T>`, `Result<T, E>`, or Rust-owned generics across FFI as if they were C ABI types.
28. Freeing memory on the wrong side of the FFI boundary.
29. Using `CString::from_raw` on memory not created by `CString::into_raw`.
30. Letting C mutate memory that Rust believes is immutable.
31. Accepting null pointers without checking them.
32. Ignoring alignment, length, ownership, encoding, and initialization of foreign buffers.
33. Treating foreign `char *` as UTF-8 without validation.
34. Returning borrowed pointers to temporary Rust data.
35. Allowing callbacks after the Rust object they reference has been dropped.
36. Assuming a foreign library is thread-safe, async-signal-safe, reentrant, or fork-safe without documentation.
37. Letting panics cross an FFI boundary with the wrong ABI.
38. Using `extern "C"` where unwinding is possible and should be modeled as `C-unwind`, or using `C-unwind` casually without understanding the other language’s exception/unwind settings.
39. Using `#[no_mangle]`, `#[export_name]`, or `#[link_section]` without understanding the global symbol/linker safety requirements. Rust 2024 requires marking these as unsafe attributes because the compiler cannot verify their soundness requirements.
40. `unwrap()` or `expect()` on user input, network input, filesystem input, environment variables, database rows, CLI args, IPC, untrusted JSON, decrypted data, or anything attacker-influenced.
41. `panic!` as normal error handling in libraries.
42. Panicking in public APIs without documenting the contract.
43. Panicking in `Drop`.
44. Panicking across FFI.
45. Panicking inside async tasks where the `JoinHandle` is ignored.
46. Treating malformed input as impossible.
47. Discarding `Result` with `let _ = ...`.
48. Calling `.ok()` or `.err()` just to throw information away.
49. `map_err(|_| Error::Failed)` that destroys critical context.
50. Logging an error and continuing as if the operation succeeded.
51. Retrying forever without backoff, limit, or cancellation.
52. Converting all errors to strings too early.
53. Using `anyhow::Error` as a public library API when callers need structured recovery.
54. Returning `Option` for a failure that needs an error explanation.
55. Returning `bool` for a failure that has multiple causes.
56. Swallowing `JoinError`, channel-close errors, task failures, flush failures, fsync failures, or serialization failures.
57. Calling `todo!()`, `unimplemented!()`, or `unreachable!()` in reachable production paths.
58. Depending on `debug_assert!` for security or memory safety.
59. Holding a `std::sync::MutexGuard`, `RwLockGuard`, `RefCell` borrow, or other blocking guard across `.await`.
60. Blocking an async runtime thread with `std::thread::sleep`, blocking filesystem/network calls, blocking crypto, CPU-heavy loops, or synchronous clients.
61. Spawning tasks and ignoring their `JoinHandle` when task failure matters.
62. Fire-and-forget background tasks with no shutdown path.
63. Unbounded channels fed by external input.
64. No backpressure on network, queue, WebSocket, streaming, logging, or telemetry paths.
65. `tokio::select!` code that is not cancellation-safe.
66. Assuming dropping a future rolls back partial side effects.
67. Creating a new runtime inside an existing runtime.
68. Mixing async runtimes accidentally.
69. Using sleeps as synchronization.
70. Relying on timing to make tests pass.
71. Holding locks while calling user code, callbacks, logging hooks, or async code.
72. Lock-order inversions.
73. `Arc<Mutex<_>>` as default architecture for shared mutable state.
74. Shared global mutable state without a concurrency model.
75. Using `static mut`.
76. Using atomics without documenting the invariant.
77. Using `Relaxed` because it is faster, not because it is correct.
78. Busy-spinning without yielding, parking, backoff, or power considerations.
79. Ignoring poisoning or pretending poisoning is recovery.
80. Assuming code is single-threaded because today’s executor happens to run it that way.
81. Blindly adding a crate because an LLM suggested it.
82. Adding a crate whose name, repository, docs, ownership, maintenance, license, and advisory status were not checked.
83. Trusting a hallucinated crate name.
84. Depending on random Git branches for production.
85. Wildcard dependency versions.
86. Unpinned git dependencies.
87. Ignoring RustSec advisories.
88. Ignoring yanked crates.
89. Ignoring unmaintained crates in security-sensitive paths.
90. Ignoring duplicate versions of critical crates, especially crypto, TLS, parsers, allocators, async runtimes, and serialization crates.
91. Allowing build scripts from untrusted crates without review.
92. Letting `build.rs` download code, run shell commands, depend on network state, or inspect host secrets unexpectedly.
93. Pulling in default features without checking what they enable.
94. Accidentally enabling OpenSSL/native TLS, vendored C, proc macros, networking, compression, or filesystem features.
95. Non-additive features: features that remove APIs, change semantics, or conflict with other features.
96. Changing default features casually.
97. Publishing a library without testing feature combinations.
98. Shipping an application without a `Cargo.lock`.
99. Treating `cargo update` as harmless in a release branch.
100. Failing to check licenses and source provenance.
101. Relying on abandoned crates for core safety/security behavior.
102. Vendoring C/C++ code without tracking its CVEs and compiler flags.
103. Letting proc macros from unknown crates execute in your build pipeline.
104. Accepting dependency bloat because “Cargo makes it easy.”

## Input and injection

1. SQL built with `format!`.
2. Shell commands built from untrusted strings.
3. Path joins that allow traversal.
4. Archive extraction without zip-slip protection.
5. SSRF-prone URL fetching.
6. Regexes vulnerable to catastrophic backtracking.
7. Deserializing untrusted data without size/depth limits.
8. Parsing untrusted input with indexing and `unwrap`.
9. Assuming JSON shape because the frontend “always sends it.”
10. Allocating based on untrusted lengths without caps.
11. Integer overflow in size calculations.
12. Ignoring Unicode normalization where identity/security depends on text.
13. Trusting client-provided authorization state.
14. Trusting headers without proxy/auth boundary design.
15. Using `HashMap` where hash-flooding matters and the hasher choice was changed casually.
16. Missing timeouts on network calls.
17. Missing body-size limits on HTTP endpoints.
18. Missing rate limits on expensive endpoints.
19. Logging attacker-controlled strings into systems where log injection matters.
20. Public safe APIs that allow invalid states.
21. Public fields that let callers violate invariants.
22. Constructors that do not validate.
23. Setters that can break invariants.
24. Boolean parameter soup for security or state transitions.
25. `String`ly typed domain concepts where newtypes are needed.
26. Confusing units: bytes vs chars, seconds vs milliseconds, UTC vs local, user ID vs account ID.
27. Exposing internal concrete types that prevent future fixes.
28. Returning borrowed data tied to surprising lifetimes.
29. Overly broad generic bounds that leak implementation.
30. Blanket impls that create coherence conflicts.
31. Trait implementations that violate expected laws:
32. `Eq` inconsistent with `PartialEq`.
33. `Hash` inconsistent with `Eq`.
34. `Ord` not total.
35. `Borrow`/`AsRef` returning surprising views.
36. `Deref` used to fake inheritance.
37. `Clone` duplicating ownership of unique resources.
38. `Copy` for types that represent ownership, permissions, handles, or state transitions.
39. `Default` producing invalid, insecure, or unusable values.
40. `Debug` leaking secrets.
41. `Index` on data where absence is expected and should be handled.
42. Hidden panics in innocent-looking helpers.
43. Missing `#[must_use]` on builders, guards, transactions, or values where ignoring them is likely a bug.
44. Encoding state in comments instead of types.
45. Encoding safety contracts in README prose only, not in API shape.
46. Releasing a breaking public API change as patch/minor.
47. Removing public items without a major version bump.
48. Changing trait behavior in a way downstream code can observe.
49. Adding trait impls that break downstream coherence without considering semver.
50. Changing default features casually.
51. Making features mutually exclusive without compile-time checks and clear docs.
52. Making features non-additive.
53. Changing MSRV silently when your project claims an MSRV policy.
54. Publishing a crate with undocumented safety requirements.
55. Publishing a crate with public unsafe APIs but no `# Safety` docs.
56. Publishing a crate with no license metadata.
57. Publishing a crate with examples that teach `unwrap` on real fallible paths.
58. Publishing a crate whose docs do not mention panics or errors.
59. Hiding a security fix in a vague changelog.
60. Failing to yank a dangerously broken release when appropriate.
61. Treating “0.x” as permission to break users carelessly.
62. Unbounded memory growth from external input.
63. Unbounded recursion on external input.
64. Unbounded queues.
65. Unbounded task spawning.
66. Unbounded log cardinality.
67. Recompiling regexes in hot loops.
68. Creating HTTP clients, DB pools, runtimes, or TLS configs per request.
69. `Vec::remove(0)` in hot paths.
70. Repeated `collect()` just to iterate.
71. Repeated `clone()` of large data because borrowing was inconvenient.
72. `to_string()`/`format!()` in hot paths without need.
73. Accidental O(n²) algorithms in parsers, routers, diffing, search, or deduplication.
74. Using `String` where `&str`, `Cow<'_, str>`, or bytes would be clearer.
75. Using `Vec<u8>` copies where slices would suffice.
76. Using async for CPU-bound work without a blocking pool or parallelism strategy.
77. Using blocking I/O in async hot paths.
78. Busy polling.
79. Sleeping instead of using readiness, notifications, timers, or backoff.
80. Using `unsafe` for speed without measurement and a safety proof.
81. Replacing clear safe code with clever unsafe code for unmeasured performance.
82. Ignoring `cargo clippy` performance lints because “it’s just a lint.”
83. Letting debug logging format huge structures on hot paths.
84. Cloning secrets or large buffers into logs/errors.
85. Shipping code that does not pass `cargo check`.
86. Shipping code that does not pass tests.
87. Shipping code that does not pass formatting.
88. Ignoring Clippy findings without specific justification.
89. Blanket `#![allow(warnings)]`.
90. Blanket `#[allow(clippy::all)]`.
91. Blanket `#[allow(dead_code)]` to hide unfinished code.
92. Tests that only prove the happy path.
93. Tests that assert implementation details but miss contracts.
94. Tests with `assert!(true)`, snapshot spam, or generated assertions nobody reviewed.
95. Flaky sleep-based tests.
96. Tests that depend on the developer’s machine, timezone, locale, network, clock, home directory, or installed services without isolation.
97. Not testing error paths.
98. Not testing malformed input.
99. Not testing large input.
100. Not testing feature combinations.
101. Not testing `--no-default-features` when supported.
102. Not testing docs for a public crate.
103. Not testing MSRV when an MSRV is promised.
104. Not running Miri on unsafe-heavy code where feasible.
105. Not fuzzing parsers, decoders, compressors, protocol handlers, or unsafe boundary code.
106. Not using Loom or equivalent techniques for subtle concurrent code.
107. Not testing cancellation paths in async code.
108. Not testing shutdown.
109. Not testing partial writes, interrupted I/O, retries, timeouts, and backpressure.
110. Not testing panic behavior if panics are part of the contract.
111. Removing tests because generated code broke them.
112. Accepting LLM-generated tests that simply mirror the implementation.
113. Disabling warnings globally.
114. Hiding generated files from review.
115. Checking in generated Rust without the generator, inputs, version, and review path.
116. Depending on nightly Rust in a stable-facing project without a concrete reason.
117. Using `RUSTFLAGS=-A warnings` in CI.
118. CI that only tests the default feature set.
119. CI that does not run on supported platforms.
120. CI that skips docs for a library crate.
121. Build scripts that depend on network access.
122. Build scripts that read secrets.
123. Build scripts that behave differently by developer machine.
124. Procedural macros with hidden filesystem/network side effects.
125. Not pinning toolchain where reproducibility matters.
126. Not documenting MSRV.
127. Relying on debug-only checks for release safety.
128. Assuming integer overflow behavior is the same in debug and release.
129. Leaving `dbg!`, `println!`, or temporary tracing in library code.
130. Leaving `todo!`, `unimplemented!`, feature stubs, or dead experimental paths in production builds.
131. Shipping benchmarks that do not measure the relevant workload.
132. “Fixed by cargo clean” as an explanation.
133. Representing validated data with raw primitive types after validation.
134. Losing the distinction between trusted and untrusted data.
135. Losing the distinction between encoded and decoded data.
136. Losing the distinction between escaped and unescaped strings.
137. Losing the distinction between authenticated and unauthenticated identity.
138. Losing the distinction between local time and UTC.
139. Serializing `usize`/`isize` in stable file/network formats.
140. Assuming endianness.
141. Assuming pointer width.
142. Assuming filesystem case sensitivity.
143. Assuming path separators.
144. Assuming UTF-8 where bytes are allowed.
145. Assuming one Unicode code point equals one visible character.
146. Assuming `SystemTime` is monotonic.
147. Assuming wall-clock time is safe for ordering/security.
148. Treating floating-point equality as domain truth where precision matters.
149. Using floats for money.
150. Ignoring overflow in financial, quota, gas, billing, or authorization calculations.
151. Using `HashMap` iteration order where deterministic consensus/output is required.
152. Depending on random seeds in deterministic protocols.
153. Treating serialization format changes as non-breaking.
154. Parsing with `.split(...).nth(...).unwrap()` in real input paths.
155. Silently accepting unknown fields where strictness matters.
156. Silently ignoring invalid states during deserialization.
157. Libraries printing to stdout/stderr instead of returning errors or using logging/tracing abstractions.
158. Services without structured logs around important failures.
159. No correlation/request IDs in distributed systems.
160. Logs that omit enough context to debug.
161. Logs that include secrets or PII.
162. Metrics with unbounded labels.
163. Metrics that hide failures by only counting successes.
164. Health checks that do not check dependencies they claim to check.
165. Readiness checks that return ready before the service can serve.
166. No timeout on outbound calls.
167. No graceful shutdown.
168. No flushing of telemetry/logs where required.
169. No migration/rollback plan for data changes.
170. No resource limits.
171. No crash-loop visibility.
172. Swallowing panic/task failures in background workers.
173. Treating restart as recovery when data consistency is not addressed.
174. Macros that hide unsafe code without making safety requirements obvious.
175. Macros that generate public APIs with undocumented panics/errors.
176. Proc macros that panic with useless diagnostics.
177. Proc macros that emit unhygienic names.
178. Proc macros that read unrelated files or environment variables unexpectedly.
179. Proc macros that perform network access.
180. Proc macros that make semver-sensitive assumptions about downstream code.
181. Macro-generated trait impls that violate trait laws.
182. Macro-generated `unsafe impl Send/Sync`.
183. Macro-generated FFI exports without symbol-safety review.
184. Macro-generated code that cannot be inspected, tested, or reproduced.
185. “The macro generated it” as an excuse for bad code.
186. `static mut` shared with interrupts without critical sections or atomics.
187. MMIO without volatile access where volatile is required.
188. Volatile used as a synchronization primitive when atomics/critical sections are required.
189. Incorrect interrupt masking.
190. Allocating in interrupt contexts unless the allocator is designed for it.
191. Panicking where no panic strategy exists.
192. Assuming `std`.
193. Assuming heap availability.
194. Assuming atomics exist on the target.
195. Wrong linker sections.
196. Wrong alignment for DMA buffers.
197. Not considering cache coherency for DMA.
198. Using normal references for memory that hardware mutates.
199. Ignoring memory barriers for device registers.
200. Busy loops that break power/latency constraints.
201. Using target features unavailable on deployed hardware.
202. No request body limits.
203. No timeout limits.
204. No concurrency limits.
205. No rate limits where abuse is plausible.
206. SQL injection via string concatenation.
207. Command injection via shell strings.
208. SSRF by blindly fetching user-supplied URLs.
209. Directory traversal in static file or upload handling.
210. Trusting `X-Forwarded-*` headers without trusted proxy configuration.
211. Wildcard CORS with credentials.
212. Disabling TLS verification.
213. Authentication middleware that can be skipped by route order.
214. Authorization checked only in the frontend.
215. Session cookies without secure flags where required.
216. CSRF ignored for cookie-authenticated state-changing routes.
217. Password reset tokens that are guessable, reusable, long-lived, or logged.
218. `unwrap` in request handlers on user-controlled data.
219. Spawning background work from requests with no durability, cancellation, or retry story.
220. Returning internal errors, paths, SQL, tokens, or stack traces to users.
221. Treating JSON deserialization success as validation.
222. Using `Debug` formatting in API responses.
223. Building SQL with `format!` and untrusted input.
224. Ignoring transaction boundaries.
225. Doing multi-step state changes without transactions where atomicity matters.
226. Ignoring isolation/concurrency behavior.
227. Assuming “read then write” is safe under concurrent requests.
228. Not handling unique constraint failures.
229. Not handling partial migrations.
230. Migrations with no rollback or forward-fix strategy.
231. Silent data truncation.
232. Timezone-naive timestamps.
233. Storing secrets or tokens unhashed.
234. Logging queries with secrets.
235. Treating `last_insert_id` or equivalent as safe without understanding connection/session semantics.
236. Unbounded query results loaded into memory.
237. N+1 queries in hot paths because it was easier.
238. Failing to fsync or flush where durability is promised.
239. Pretending a cache write is durable storage.
240. “It compiles, ship it.”
241. “The borrow checker accepted it, so the design is good.”
242. “The LLM said it is safe.”
243. “The LLM added a `SAFETY:` comment, so unsafe is reviewed.”
244. “The tests pass,” when the tests were generated from the same mistaken assumptions.
245. Asking an LLM to repeatedly patch compiler errors until they disappear without understanding the final design.
246. Adding `clone`, `Rc`, `RefCell`, `Arc`, `Mutex`, `Box::leak`, or `'static` until lifetime errors vanish.
247. Accepting generated unsafe code.
248. Accepting generated FFI code.
249. Accepting generated crypto code.
250. Accepting generated auth code.
251. Accepting generated parser code for untrusted input without fuzzing.
252. Accepting generated dependency names without verification.
253. Accepting generated API calls without checking current docs.
254. Accepting generated code that uses old crate versions or removed APIs.
255. Accepting generated code that suppresses warnings.
256. Accepting generated code that has broad `allow` attributes.
257. Accepting generated code that has no error model.
258. Accepting generated code that has no cancellation/shutdown model.
259. Accepting generated code whose invariants are not documented.
260. Accepting generated code whose author/reviewer cannot explain ownership, lifetime, error, concurrency, and security behavior.
261. Letting the LLM design the architecture by averaging internet examples from other languages.
262. Letting AI-generated Rust imitate Python/TypeScript/Go framework patterns without Rust-specific ownership, type, and error design.
263. Flooding open-source maintainers with unreviewed AI PRs.
264. Claiming AI-generated code is “reviewed” because the AI reviewed itself.
265. Dismissing soundness reports because “safe Rust users are fine.”
266. Hiding security fixes in vague changelog entries.
267. Not publishing an advisory for a real vulnerability.
268. Not yanking a release known to be dangerously broken.
269. Refusing to document unsafe invariants.
270. Reviewing unsafe code as if it were ordinary code.
271. Letting junior developers or AI agents own unsafe/FFI/crypto without expert review.
272. Merging generated code nobody understands.
273. Merging code that only the original author can maintain.
274. Using “performance” to shut down safety questions.
275. Using “Rust prevents that” to shut down security review.
276. Using “we can fix it later” for a public safety/security contract.
277. Treating CI as a substitute for design review.
278. Treating code coverage as a substitute for adversarial testing.
279. Treating dependency popularity as a security audit.
280. Treating crate downloads/stars as proof of maintenance.
281. Ignoring license obligations.
282. Copy-pasting code without license compatibility.
283. Depending on abandoned crates without a contingency plan.
284. Allowing code owners to approve their own risky changes without independent review.
285. No security contact for public crates/services.
286. No policy for vulnerability disclosure.
287. No audit trail for generated code, generated bindings, or generated unsafe.
288. `unwrap` / `expect`
289. `panic!`
290. `Arc<Mutex<_>>`
291. `Rc<RefCell<_>>`
292. `Box<dyn Trait>`
293. `async_trait`
294. `anyhow`
295. global state
296. `lazy_static` / `OnceLock`
297. `Cow`
298. `unsafe(no_mangle)`
299. `repr(C)`
300. git dependencies
301. large dependencies
302. custom allocators
303. `Relaxed`
304. `Pin`
305. `MaybeUninit`
306. `ManuallyDrop`
307. `transmute`
308. `get_unchecked`
309. The author can explain every unsafe block and unsafe trait impl.
310. Every unsafe block has a real `SAFETY:` comment.
311. Public unsafe APIs have `# Safety` docs.
312. Public panics and errors are documented.
313. No broad warning/lint suppression exists.
314. `cargo fmt --check` passes.
315. `cargo clippy --all-targets --all-features -- -D warnings` passes, or deviations are narrow and justified.
316. `cargo test --all-targets --all-features` passes.
317. Docs build.
318. Feature combinations relevant to the crate are tested.
319. Dependencies are reviewed with advisory/license/source checks.
320. LLM-suggested dependencies are manually verified.
321. Unsafe/concurrent/parser code has extra verification: Miri, fuzzing, Loom, sanitizers, property tests, or targeted review as appropriate.
322. Error handling covers expected failures.
323. Resource limits exist for untrusted input.
324. No generated code is accepted without provenance and review.
325. The code’s owner can describe its failure modes.

## Invalid initialization and destruction

1. `MaybeUninit::assume_init()` before every byte and invariant is initialized.
2. `mem::zeroed()` for types where all-zero is invalid.
3. `mem::uninitialized()` or equivalent patterns.
4. `Vec::set_len()` before elements are fully initialized.
5. Double-dropping values.
6. Forgetting values to paper over ownership bugs.
7. `ManuallyDrop` used without a precise drop protocol.
8. `ptr::read`, `ptr::copy`, or `ptr::copy_nonoverlapping` with wrong ownership, overlap, alignment, or initialization assumptions.
9. `Box::from_raw`, `Vec::from_raw_parts`, `CString::from_raw`, or allocator APIs with pointers not produced by the matching owner/allocator.
10. Creating references to packed fields that may be unaligned.
11. Creating `&T` or `&mut T` from raw pointers without proving non-null, aligned, initialized, dereferenceable, valid, and correctly aliased.

## Pointers, references, aliasing, and validity

1. **Dereferencing null, dangling, misaligned, or invalid raw pointers.**
2. **Creating a Rust reference from a raw pointer before proving validity, alignment, provenance, aliasing, and lifetime.**
3. **Creating `&mut T` when any other live reference or pointer can read or write the same memory in violation of aliasing rules.**
4. **Creating multiple mutable references to the same object through raw pointers.**
5. **Mutating through `&T` except through valid interior mutability such as `UnsafeCell`.**
6. **Producing invalid values for Rust types: invalid `bool`, invalid `char`, invalid enum discriminants, invalid `NonZero*`, invalid `NonNull`, invalid references, invalid `String`, invalid `Vec`, or invalid niche-optimized types.**
7. **Assuming all bit patterns are valid for a type.**
8. **Reading uninitialized memory as a typed value.** The Rustonomicon is blunt: interpreting uninitialized memory as a value causes UB.
9. **Calling `MaybeUninit::assume_init` before every byte that matters is initialized to a valid value.**
10. **Dropping uninitialized memory.**
11. **Double-dropping through `ptr::read`, `ManuallyDrop`, `MaybeUninit`, `mem::forget`, or custom drop glue.**
12. **Using `mem::zeroed` on types where the all-zero bit pattern is invalid.**
13. **Using `transmute` as a normal conversion tool.** The standard library says both source and destination must be valid at their types, violating that is UB, and `transmute` should be an absolute last resort.
14. **Transmuting lifetimes to `'static` unless the value truly lives for `'static`.**
15. **Transmuting between pointer and integer types without understanding the memory model and provenance.**
16. **Transmuting containers like `Vec<T>` to `Vec<U>` by assuming internal layout.**
17. **Relying on `repr(Rust)` layout for FFI, disk formats, network formats, or cross-version compatibility.**
18. **Using `slice::from_raw_parts` or `from_raw_parts_mut` without proving pointer validity, alignment, length, initialized memory, and aliasing.**
19. **Using `get_unchecked` when the index can be influenced by input, races, or stale validation.**
20. **Using `unreachable_unchecked` for a branch that is merely “unlikely.”**
21. **Using `assume`, unchecked math, unchecked indexing, or unchecked UTF-8 as performance tricks without proof.**

## Secrets

1. Hardcoded API keys, tokens, passwords, private keys, salts, or seed phrases.
2. Secrets in tests that are also valid in production.
3. Secrets in logs, panics, telemetry, metrics labels, traces, `Debug`, `Display`, or error messages.
4. Deriving `Debug`, `Clone`, `Serialize`, or `Deserialize` on secret-holding types without thinking.
5. Storing passwords as plaintext.
6. Comparing secrets with ordinary equality when constant-time comparison is needed.
7. Keeping sensitive data in memory longer than necessary.
8. Assuming `drop` zeroes memory.
9. Using `String`/`Vec<u8>` for secrets without a zeroization story in high-risk code.
10. Sending secrets to an LLM or external tool.

## The core rule

1. **Makes undefined behavior possible.**
2. **Lies to the type system, borrow checker, or API user.**
3. **Uses `unsafe` without a written, local, reviewable safety argument.**
4. **Turns expected failures into panics.**
5. **Silences tools instead of fixing the issue.**
6. **Ships AI/generated/copied code the author does not understand.**
7. **Creates public APIs that hide invariants, errors, panics, blocking, or unsafety.**
8. **Breaks dependency, feature, or semver trust.**
9. **Treats “it compiles” as evidence of correctness.**
10. Accepting AI-written Rust because it compiles, without understanding ownership, lifetimes, trait bounds, error behavior, concurrency behavior, and safety invariants.
11. Asking an AI to “fix the borrow checker” and accepting the first version that adds `clone()`, `Arc<Mutex<_>>`, `Box::leak`, `Rc<RefCell<_>>`, `static mut`, `unsafe`, or `transmute`.
12. Letting AI remove lifetime parameters by heap-allocating everything.
13. Letting AI replace clear ownership with shared mutable state.
14. Letting AI replace type errors with `as` casts.
15. Letting AI “fix” async code by adding `spawn`, `block_on`, `spawn_blocking`, unbounded channels, or global state without an execution model.
16. Letting AI weaken tests, remove assertions, broaden tolerances, or ignore flaky failures.
17. Merging generated code where nobody can explain why each `unsafe` block is sound.
18. Merging generated code where nobody can explain why cancellation, panics, drops, locks, and errors behave correctly.
19. Merging code with “TODO: handle error later,” `todo!()`, `unimplemented!()`, `panic!("should not happen")`, or placeholder branches in reachable production paths.
20. Hiding AI-generated code behind a macro or build script so reviewers cannot see what is actually compiled.
21. Using compile-time network calls, LLM calls, or nondeterministic code generation as part of normal builds.
22. Blindly accepting dependency additions suggested by an AI, especially proc macros, build dependencies, crypto crates, serialization crates, async runtimes, FFI bindings, or crates with little maintenance history.
23. Copying from Reddit, Stack Overflow, GitHub gists, GitLab snippets, or X posts without checking crate versions, API changes, safety contracts, and whether the example was a toy.
24. Treating social-media idioms as stronger evidence than the Rust Reference, Rust Book, standard-library docs, crate docs, or the crate’s issue tracker.
25. Creating a data race.
26. Dereferencing a dangling pointer.
27. Dereferencing a misaligned pointer.
28. Creating an invalid reference.
29. Creating a reference to uninitialized memory.
30. Creating `&T` or `&mut T` that violates aliasing rules.
31. Having two active mutable references to the same memory.
32. Creating a shared reference while mutation occurs through another path not permitted by Rust’s aliasing model.
33. Forging references from integers or arbitrary addresses.
34. Treating arbitrary memory as a valid Rust value.
35. Producing invalid values, such as invalid `bool`, invalid `char`, invalid enum discriminants, invalid `NonNull`, invalid `NonZero*`, invalid function pointers, invalid references, or invalid wide-pointer metadata.
36. Calling `assume_init` before full initialization is actually complete.
37. Using `MaybeUninit<T>` but accidentally reading uninitialized fields.
38. Using `mem::zeroed()` for types where all-zero is not a valid value.
39. Using `mem::uninitialized()` patterns through old code, wrappers, FFI, or transmute-like tricks.
40. Using `transmute` where layout, validity, provenance, alignment, lifetime, or drop behavior is not formally established.
41. Depending on `repr(Rust)` layout.
42. Assuming enum layout without `repr` guarantees.
43. Assuming niche optimization details as a stable contract.
44. Using `repr(packed)` and then creating references to possibly unaligned fields.
45. Using `get_unchecked`, `unwrap_unchecked`, or `unreachable_unchecked` because “I think this can’t happen,” without a mechanically reviewable proof.
46. Calling compiler intrinsics incorrectly.
47. Using unsupported target features.
48. Writing inline assembly with false clobbers, false memory effects, wrong stack assumptions, or wrong register constraints.
49. Violating ABI calling conventions.
50. Allowing unwinding through an ABI boundary where that is not permitted.
51. Mutating immutable bytes.
52. Violating Rust runtime assumptions.
53. Assuming undefined behavior is local. UB can invalidate the whole program’s meaning, not just the line where it appears.
54. Adding `unsafe` to make compiler errors disappear.
55. Adding `unsafe` before understanding why safe Rust rejected the program.
56. Using `unsafe` to bypass the borrow checker rather than fixing ownership.
57. Using `unsafe` because it is “faster” without measurement and proof.
58. Using `unsafe` in a public API without a `# Safety` section.
59. Calling an unsafe function without reading and satisfying its safety contract.
60. Writing an unsafe function whose caller obligations are vague.
61. Writing an unsafe trait without documenting what implementors must guarantee.
62. Implementing an unsafe trait without proving the required invariants.
63. Writing `unsafe impl Send` or `unsafe impl Sync` because the compiler complained.
64. Marking a type `Send` or `Sync` when it contains raw pointers, FFI handles, thread-affine resources, ref-counted non-threadsafe state, hidden mutation, or foreign-library state whose thread-safety is unknown.
65. Exposing a safe function that can cause UB when called with ordinary safe values.
66. Relying on users to “not do that” in safe APIs.
67. Making struct fields public when private fields are needed to preserve unsafe invariants.
68. Trusting arbitrary generic safe code to maintain invariants required by your unsafe code.
69. Using `unsafe` far away from the invariant check that makes it safe.
70. Writing large unsafe blocks where reviewers cannot tell which operation required unsafety.
71. Failing to put a `SAFETY:` comment next to every unsafe operation.
72. Having a `SAFETY:` comment that merely restates what the code does, rather than why the operation is valid.
73. Writing `SAFETY: should be fine`.
74. Writing `SAFETY: tested`.
75. Writing `SAFETY: copied from std` without preserving the same invariants.
76. Hiding unsafe code in macros.
77. Hiding unsafe code in dependencies.
78. Hiding unsafe code in proc macros or generated code.
79. Wrapping unsound unsafe code in a safe abstraction and calling it “ergonomic.”
80. Declaring FFI signatures by guesswork.
81. Using the wrong integer width, pointer type, struct layout, enum representation, alignment, ownership convention, allocator, or ABI.
82. Passing Rust references to C code that may store them, mutate behind shared references, outlive the call, or call back reentrantly.
83. Passing `String`, `Vec<T>`, `Box<T>`, trait objects, slices, or Rust enums over FFI without an explicit ABI-safe representation.
84. Assuming `repr(C)` solves ownership, lifetime, aliasing, threading, or allocator issues.
85. Freeing memory with the wrong allocator.
86. Calling `Box::from_raw`, `Vec::from_raw_parts`, `CString::from_raw`, or similar functions without exact ownership provenance.
87. Letting C hold a pointer into a Rust allocation that Rust may move, reallocate, or drop.
88. Letting Rust assume C initialized memory when C may fail partially.
89. Failing to check C return codes before reading out-parameters.
90. Treating nullable C pointers as non-null Rust references.
91. Constructing `&T` from a C pointer before checking nullness, alignment, initialization, aliasing, and lifetime.
92. Allowing panics to unwind into C unless the ABI and boundary explicitly support it.
93. Allowing C exceptions or longjmp-like behavior to cross Rust frames unsafely.
94. Marking an `extern` function as safe unless every safe Rust caller can call it without violating Rust’s guarantees.
95. Assuming a foreign library is thread-safe without documentation.
96. Assuming callbacks cannot be concurrent, reentrant, or called after teardown unless the foreign API guarantees that.
97. Using `unwrap()` on user input.
98. Using `unwrap()` on file, network, database, environment, CLI, config, clock, locale, OS, parser, deserializer, HTTP, channel, lock, task, or FFI results.
99. Using `expect("works")`, `expect("should not fail")`, or `expect("lol")`.
100. Using `panic!` for ordinary validation errors.
101. Using `panic!` for malformed requests.
102. Using `panic!` for missing config unless startup failure is the intended, documented behavior.
103. Using `panic!` for parse failures.
104. Using `panic!` for network failures.
105. Using `panic!` for database failures.
106. Using `panic!` for lock poisoning without an explicit recovery/crash policy.
107. Using `panic!` as control flow.
108. Catching panics as normal error handling.
109. Returning `None` when the caller needs to know why something failed.
110. Returning `String` errors from a library API when structured errors are needed.
111. Collapsing all errors into “failed” while losing source, context, retryability, path, operation, status code, or backtrace.
112. Calling `.ok()` merely to discard an error.
113. Calling `.unwrap_or_default()` to hide a failure.
114. Ignoring `Result` with `let _ = ...` unless the ignored failure is explicitly harmless and documented.
115. Using `debug_assert!` for safety-critical validation.
116. Using `assert!` for untrusted input validation when the right behavior is a recoverable error.
117. Panicking inside `Drop`.
118. Panicking while already unwinding.
119. Letting panic cross FFI boundaries.
120. Documenting a function as infallible when it can panic.
121. Hiding panics in trait implementations, conversions, indexing, formatting, serialization, or destructors.
122. Making examples teach `unwrap()` where `?` would be just as clear.
123. Representing validated domain values as raw `String`, `usize`, `i32`, or `bool` everywhere when invalid values are common and dangerous.
124. Using `bool` parameters whose meaning is unclear at call sites.
125. Using sentinel values instead of `Option`, `Result`, enums, or newtypes.
126. Using `usize` for external IDs, signed domain values, wire formats, or database identifiers without a reason.
127. Using `String` for states that should be enums.
128. Using `&str` or `String` for paths, URLs, email addresses, tokens, database IDs, units, or permissions when validation/invariants matter.
129. Making invalid state representable and then scattering checks everywhere.
130. Exposing public fields that allow users to violate invariants.
131. Deriving or implementing `Default` for a type when the default value is invalid, misleading, insecure, or partial.
132. Deriving `Serialize`/`Deserialize` in a way that bypasses validation.
133. Implementing `From` for fallible conversions instead of `TryFrom`.
134. Implementing `Deref` for non-pointer-like types to make method calls convenient.
135. Abusing `AsRef`, `Borrow`, `Into`, or `From` with surprising semantics.
136. Encoding state machines as comments instead of types.
137. Using `Option<Result<Option<T>>>`-style nesting because the design was not decomposed.
138. Returning huge tuples instead of named structs.
139. Using type aliases to hide semantically different concepts that should be distinct newtypes.
140. Making a trait public and implementable when external implementations could break invariants.
141. Using `unsafe` because the type model was too weak.
142. Cloning because you do not understand the borrow error.
143. Adding `.to_string()`, `.to_owned()`, `.clone()`, or `.collect()` until the compiler stops complaining.
144. Returning owned values everywhere because lifetimes feel scary.
145. Borrowing everything with complex lifetimes because allocation feels shameful.
146. Using `Box::leak` to get a `'static` lifetime.
147. Using `lazy_static`, `OnceCell`, `static`, or globals to avoid passing state explicitly.
148. Using `Rc<RefCell<T>>` as a default architecture.
149. Using `Arc<Mutex<T>>` as a default architecture.
150. Adding `RefCell` to bypass the borrow checker without a runtime borrowing design.
151. Adding `Mutex` to bypass mutability errors without a concurrency design.
152. Holding borrows longer than necessary.
153. Creating self-referential structs with unsafe pinning tricks when a simpler ownership model exists.
154. Using `Pin` without understanding what is pinned, who guarantees it, and what operations remain legal.
155. Moving pinned data.
156. Implementing projection over pinned fields unsafely.
157. Forgetting values to avoid borrow/drop issues.
158. Leaking memory to avoid lifetimes.
159. Relying on destructor order without making it explicit.
160. Creating reference cycles with `Rc` or `Arc` and no `Weak` break.
161. Making APIs require `'static` because lifetimes were hard rather than because the value must truly outlive the program/task/thread.
162. Holding a `std::sync::MutexGuard`, `RwLockGuard`, `RefCell` borrow, or other blocking guard across `.await`.
163. Holding any lock across `.await` without proving it is designed for that and cannot deadlock.
164. Blocking inside async code with file IO, DNS, database drivers, HTTP clients, sleeps, compression, crypto, CPU-heavy loops, or sync locks.
165. Calling `std::thread::sleep` inside async tasks.
166. Busy-looping in an async task without yielding.
167. Calling `block_on` inside an async runtime.
168. Creating nested runtimes casually.
169. Spawning tasks and dropping their `JoinHandle` when failure, cancellation, or shutdown matters.
170. Ignoring panics from spawned tasks.
171. Treating `tokio::spawn` as fire-and-forget without lifecycle ownership.
172. Using `spawn_blocking` for unbounded, long-lived, or never-ending work.
173. Using unbounded channels by default.
174. Ignoring backpressure.
175. Ignoring cancellation safety in `select!`.
176. Holding database transactions, file handles, sockets, locks, or permits across cancellable await points without a rollback/drop plan.
177. Assuming drop-based cleanup will always run at the logical time you expect.
178. Assuming spawned futures run on the same thread.
179. Capturing `Rc`, `RefCell`, non-`Send` guards, or thread-affine handles across `.await` in tasks that may move threads.
180. Using an async mutex when a plain mutex plus short critical section would be simpler.
181. Using a plain mutex when a lock must genuinely be held across `.await`.
182. Using global runtime handles and background tasks with no shutdown path.
183. Mixing async runtimes accidentally.
184. Calling async functions from `Drop`.
185. Designing APIs that hide whether they block, spawn, or hold locks.
186. Using `Arc<Mutex<T>>` as an architecture substitute.
187. Locking multiple mutexes without a documented lock order.
188. Holding locks while calling user code.
189. Holding locks while doing IO.
190. Holding locks while awaiting.
191. Holding locks while logging if logging can call back, allocate, block, or lock.
192. Swallowing poisoned-lock errors without a policy.
193. Using atomics because they feel lighter than locks.
194. Using `Relaxed` ordering without a proof.
195. Using lock-free code without a memory model argument.
196. Implementing double-checked locking incorrectly.
197. Building cancellation flags, state machines, or queues with atomics but no tests over interleavings.
198. Assuming tests on one machine prove concurrency correctness.
199. Assuming Rust’s ownership model prevents logical races.
200. Using `unsafe impl Send/Sync` to make cross-thread code compile.
201. Sending FFI handles across threads without upstream guarantees.
202. Using thread-local assumptions in code that can run on a work-stealing runtime.
203. Creating background threads with no shutdown, join, or error reporting.
204. Leaking threads intentionally to avoid lifecycle management.
205. A safe API that can cause UB.
206. An unsafe API without a `# Safety` contract.
207. A fallible API that hides failure.
208. A blocking API that does not say it blocks.
209. An async API that secretly blocks.
210. An API that panics on ordinary inputs.
211. An API that panics but does not document panics.
212. An API that returns `Option` when callers need diagnostics.
213. An API that returns unstructured errors when callers need to match causes.
214. An API that erases source errors prematurely.
215. An API that exposes internal representation unnecessarily.
216. An API that uses public fields when invariants matter.
217. An API that takes `&Vec<T>` instead of `&[T]` without a reason.
218. An API that takes `&String` instead of `&str` without a reason.
219. An API that takes owned `String`, `Vec`, `PathBuf`, or `Arc` when borrowing would work and ownership is not needed.
220. An API that returns references tied to hidden global state.
221. An API that forces `'static` without a real need.
222. An API that uses `bool` flags instead of meaningful types.
223. An API that accepts invalid values and says “caller must be careful” when the type system could enforce validity.
224. An API that implements operators with surprising behavior.
225. An API that implements `Deref` just for convenience.
226. An API that makes semver-compatible evolution impossible.
227. An API that exposes dependencies in public types without intending them to become part of your public contract.
228. An API that makes feature combinations ambiguous or untested.
229. An API that does not document examples, errors, panics, safety, or invariants.
230. Using a macro to hide code reviewers need to inspect.
231. Using a macro to hide `unsafe`.
232. Using a macro to hide panics.
233. Using a macro to hide global state.
234. Using a macro to hide network access, filesystem access, environment dependence, or nondeterminism.
235. Writing proc macros that emit inscrutable errors.
236. Writing proc macros that generate code with hidden trait bounds.
237. Writing proc macros that generate unsound unsafe code.
238. Pulling in proc-macro dependencies casually; proc macros execute during compilation.
239. Using `build.rs` to download code during builds.
240. Using `build.rs` to generate different code depending on undocumented local machine state.
241. Using `build.rs` to probe system state in ways that make builds unreproducible.
242. Using compile-time code generation to avoid writing understandable Rust.
243. Making generated code impossible to audit.
244. Failing to commit generated code when committing it is necessary for review/reproducibility.
245. Committing generated code without documenting the generator version and inputs.
246. Creating macros that make ordinary control flow, allocation, locking, or error handling invisible.
247. Adding a crate without reading its docs, license, maintenance status, unsafe usage, transitive dependencies, and security history.
248. Adding a crate for one trivial function without weighing the supply-chain cost.
249. Adding a proc macro casually.
250. Adding a build dependency casually.
251. Adding a crypto, auth, parser, deserializer, compression, TLS, or FFI crate casually.
252. Ignoring RustSec advisories.
253. Ignoring yanked versions.
254. Ignoring duplicate major versions when they matter.
255. Ignoring license incompatibilities.
256. Using wildcard dependency versions.
257. Using unpinned git dependencies in production.
258. Using path dependencies accidentally in published crates.
259. Depending on an abandoned crate for security-sensitive functionality.
260. Depending on a crate that has no CI, no tests, no docs, or unclear ownership for critical paths.
261. Failing to audit feature flags.
262. Enabling broad default features without knowing what they pull in.
263. Assuming disabling default features is non-breaking.
264. Changing features in ways that break downstream users.
265. Exposing dependency types in your public API accidentally.
266. Failing to decide whether `Cargo.lock` is committed for your binary, service, tool, or workspace.
267. Shipping binaries/services without reproducible dependency resolution.
268. Publishing libraries without understanding how your dependency bounds affect downstream users.
269. Trusting `cargo install` or `cargo test` on untrusted code without considering build scripts and proc macros.
270. Using `#![allow(warnings)]`.
271. Using `#[allow(clippy::all)]`.
272. Using `#[allow(clippy::correctness)]`.
273. Suppressing Clippy correctness or suspicious lints without a narrow, local reason.
274. Adding `allow` attributes because AI suggested them.
275. Adding `allow` attributes globally instead of fixing or locally documenting the issue.
276. Ignoring `unused_must_use`.
277. Ignoring unreachable code warnings.
278. Ignoring dead code that indicates unfinished behavior.
279. Ignoring unused variables by prefixing `_` when the variable should actually be used.
280. Ignoring `unsafe_op_in_unsafe_fn`.
281. Ignoring lints around integer casts, truncation, sign loss, precision loss, or lossy conversions in security/serialization/protocol code.
282. Ignoring lints around panics, unwraps, todos, indexing, or expect in production paths.
283. Refusing `cargo fmt`.
284. Fighting formatter output with weird layout.
285. Adding custom formatting to hide complexity.
286. Treating `cargo clippy` as optional in CI.
287. Treating warnings as harmless in production crates.
288. Shipping nontrivial Rust with only “it compiles.”
289. Shipping unsafe code without tests that exercise safety boundaries.
290. Shipping parsers without malformed-input tests.
291. Shipping deserializers without adversarial tests.
292. Shipping protocol code without boundary tests.
293. Shipping concurrency code without interleaving tests where practical.
294. Shipping FFI code without null, error-code, ownership, and teardown tests.
295. Shipping public APIs without doctests or examples.
296. Testing only happy paths.
297. Removing tests because the new implementation fails them.
298. Weakening assertions to make tests pass.
299. Ignoring flaky tests.
300. Relying on test order.
301. Relying on wall-clock sleeps instead of synchronization.
302. Relying on local machine timing.
303. Not running tests with different feature combinations.
304. Not running no-default-features when you claim to support it.
305. Not running all-features when you claim feature compatibility.
306. Not testing MSRV if you claim one.
307. Not testing panic paths when panics are part of the contract.
308. Not testing cancellation paths in async code.
309. Not testing drop behavior for resource-owning types.
310. Not fuzzing input parsers, format decoders, protocol handlers, or unsafe boundary code where inputs are adversarial.
311. Not running Miri on unsafe-heavy crates when feasible.
312. Treating a clean Miri run as proof of soundness.
313. Logging secrets.
314. Printing tokens, passwords, cookies, API keys, private keys, session IDs, or auth headers with `Debug`.
315. Deriving `Debug` on secret-containing types without redaction.
316. Using `String` for secrets that need zeroization.
317. Hardcoding credentials.
318. Checking secrets into tests, examples, fixtures, or docs.
319. Using `rand`-like APIs incorrectly for cryptographic randomness.
320. Writing custom crypto.
321. Rolling your own password hashing.
322. Using unauthenticated encryption where authentication is required.
323. Comparing secrets with ordinary equality when timing resistance matters.
324. Building shell commands by string concatenation.
325. Passing untrusted strings to shells.
326. Failing to validate paths against traversal.
327. Trusting filenames from archives.
328. Extracting archives without path sanitization.
329. Deserializing untrusted data into powerful types without limits.
330. Allowing unbounded allocation from untrusted length fields.
331. Allowing regex, parser, decompression, or JSON/XML/YAML inputs to create denial-of-service.
332. Using `serde(default)` to silently accept missing security-critical fields.
333. Treating “unknown enum variant” as safe without a compatibility policy.
334. Ignoring timeouts.
335. Ignoring request body limits.
336. Ignoring rate limits.
337. Trusting client-provided IDs, roles, tenant IDs, or permissions.
338. Using `unsafe` parsers for untrusted input without fuzzing and review.
339. Assuming Rust prevents command injection, SSRF, path traversal, auth bugs, logic bugs, replay bugs, downgrade bugs, or TOCTOU races.
340. Using `unwrap()` inside parsers for untrusted input.
341. Trusting length prefixes without bounds.
342. Trusting recursion depth.
343. Trusting counts, offsets, indexes, or capacities from input.
344. Casting parsed integers with `as` without range checks.
345. Ignoring integer overflow.
346. Using debug-mode overflow behavior as if it applied in release.
347. Using `from_utf8_unchecked` on untrusted bytes.
348. Using `String::from_utf8_unchecked` without proof.
349. Treating `PathBuf` from untrusted input as safe.
350. Treating URLs as strings after validation and then mutating them.
351. Deserializing directly into internal invariant-bearing structs.
352. Letting `serde` bypass constructors that validate invariants.
353. Accepting duplicate fields, unknown fields, or missing fields without an explicit compatibility policy.
354. Silently defaulting missing security-relevant fields.
355. Accepting future enum variants as harmless without policy.
356. Parsing floats and assuming no `NaN`, infinity, negative zero, or rounding issue.
357. Sorting floats with partial ordering hacks without a NaN policy.
358. Parsing timestamps without timezone and range policy.
359. Parsing money as float.
360. Parsing protocol values into platform-dependent integer sizes.
361. Using `as` casts to silence type errors in protocol, crypto, auth, serialization, indexing, or allocation code.
362. Casting signed to unsigned without checking negativity.
363. Casting larger integers to smaller integers without checking range.
364. Casting integers to floats where precision matters.
365. Casting floats to integers without handling NaN, infinity, negative values, and bounds.
366. Using `usize` for wire formats.
367. Using platform-dependent sizes in persistent formats.
368. Ignoring overflow in release builds.
369. Using wrapping arithmetic accidentally.
370. Using saturating arithmetic where overflow should be an error.
371. Using checked arithmetic and then unwrapping it in runtime paths.
372. Allocating based on unchecked multiplication.
373. Indexing based on unchecked arithmetic.
374. Assuming debug and release overflow behavior are equivalent.
375. Assuming endianness.
376. Assuming alignment from byte slices.
377. Reinterpreting bytes as structs instead of parsing fields unless layout and validity are fully controlled.
378. Panicking in `Drop`.
379. Blocking indefinitely in `Drop`.
380. Doing network IO in `Drop`.
381. Doing async work in `Drop`.
382. Relying on `Drop` for fallible cleanup without an explicit close/flush/commit API.
383. Ignoring errors from explicit cleanup APIs.
384. Using `mem::forget` to avoid destructor problems.
385. Using `ManuallyDrop` without a precise drop-state invariant.
386. Double-dropping.
387. Leaking resources intentionally to dodge ownership.
388. Assuming destructors run at process exit in all deployment conditions.
389. Assuming destructors run after abort.
390. Assuming cancellation runs destructors in the semantic order you wanted.
391. Creating reference cycles that prevent destructors from running.
392. Holding locks during drop of user-controlled values.
393. Calling user code from `Drop` while invariants are half-destroyed.
394. Implementing custom smart pointers without proving aliasing, drop, variance, and Send/Sync behavior.
395. Implementing `Eq` when equality is not reflexive.
396. Implementing `Ord` with an order that is not total.
397. Implementing `Hash` inconsistently with `Eq`.
398. Implementing `Borrow` inconsistently with owned equality/hash behavior.
399. Implementing `Clone` with surprising side effects.
400. Implementing `Copy` for resource-owning types.
401. Implementing `Default` as a fake invalid placeholder.
402. Implementing `Display` with debug-only or lossy output when users will parse it.
403. Implementing `From` for conversions that can fail.
404. Implementing `Into` manually when `From` is the intended path.
405. Implementing `Deref` just to inherit methods.
406. Implementing `AsRef`/`Borrow` with allocation or surprising transformation.
407. Implementing `Iterator` incorrectly around `size_hint`, fused behavior, or panic behavior when consumers rely on it.
408. Implementing `ExactSizeIterator` when the length can be wrong.
409. Implementing `TrustedLen` unsafely without absolute proof.
410. Implementing `Send`, `Sync`, or unsafe marker traits without proof.
411. Implementing traits in ways that break documented invariants of generic consumers.
412. Making breaking API changes in a patch release.
413. Removing public items without a major version bump.
414. Changing error variants incompatibly without policy.
415. Changing feature behavior incompatibly without policy.
416. Removing or renaming features casually.
417. Making optional dependencies part of feature behavior without documenting them.
418. Exposing dependency types publicly by accident.
419. Increasing MSRV without documenting it.
420. Publishing without license clarity.
421. Publishing without repository, docs, changelog, or release notes for nontrivial crates.
422. Publishing examples that do not compile.
423. Publishing docs that show `unwrap()` in normal usage where `?` would be correct.
424. Publishing unsafe APIs without `# Safety`.
425. Publishing APIs that panic without `# Panics`.
426. Publishing fallible APIs without `# Errors`.
427. Publishing security-sensitive crates without threat model, tests, fuzzing, and audit posture.
428. Yanking or replacing behavior to hide mistakes instead of documenting migration.
429. Treating feature flags as internal when downstream crates depend on them.
430. No docs for public unsafe functions.
431. No safety contract for unsafe functions or unsafe traits.
432. No panic documentation for functions that panic.
433. No error documentation for functions that return `Result`.
434. No examples for nontrivial public APIs.
435. Examples that encourage `unwrap()` in normal library usage.
436. Docs that say “safe” when they mean “memory-safe but can panic/deadlock/block/leak.”
437. Docs that hide blocking behavior.
438. Docs that hide allocation behavior when it matters.
439. Docs that hide global state.
440. Docs that hide environmental dependencies.
441. Docs that hide feature-flag requirements.
442. Docs that hide platform-specific behavior.
443. Docs that hide security assumptions.
444. Docs that hide cancellation behavior.
445. Docs that hide FFI ownership rules.
446. Docs that are generated but not reviewed.
447. README claims not backed by tests.
448. `from_raw`
449. `into_raw`
450. `zeroed`
451. `MaybeUninit`
452. `Pin::new_unchecked`
453. `impl Send`
454. `impl Sync`
455. `extern "C"`
456. `no_mangle`
457. `repr(C)` used as if it solved safety
458. `Cell`
459. `Mutex` in async code
460. `spawn` with dropped handle
461. `mpsc::unbounded`
462. `.unwrap()`
463. `.expect("should not happen")`
464. `panic!()`
465. `dbg!()`
466. `println!("{:?}", secret)`
467. `as usize`
468. `as u32`
469. `as i32`
470. `as *const _`
471. `as *mut _`
472. `String` everywhere
473. unnecessary `clone()`
474. unnecessary `collect()`
475. `Default` on invariant-bearing types
476. `serde(default)` on security-sensitive fields
477. global `allow` attributes
478. new dependencies added without explanation
479. new proc macros
480. new `build.rs`
481. disabled default features without feature testing
482. changed feature flags
483. generated code that reviewers cannot regenerate
484. The author cannot explain why the code is memory-safe.
485. The author cannot explain every `unsafe` operation.
486. The author cannot state each unsafe caller obligation.
487. The author cannot explain ownership and drop order.
488. The author cannot explain how errors propagate.
489. The author cannot explain why panics are acceptable.
490. The author cannot explain async cancellation behavior.
491. The author cannot explain lock ordering and lock duration.
492. The author cannot explain dependency additions.
493. The author cannot explain feature-flag behavior.
494. The author cannot explain public API invariants.
495. The author cannot explain why generated or copied code is correct.
496. Clippy correctness/suspicious lints were silenced.
497. Tests were weakened to pass.
498. Unsafe code was not isolated, documented, and tested.
499. Expected runtime failures use panic/unwrap.
500. Public docs omit safety/errors/panics.
501. Miri/fuzzing/concurrency testing was skipped where the risk clearly calls for it.
502. The justification is “the compiler accepted it.”
503. The justification is “AI wrote it.”
504. The justification is “Rust is safe.”
505. The justification is “we can fix it later.”

## The highest-priority “never justified” Rust behaviors

1. **Creating undefined behavior and calling it acceptable.** This includes data races, dangling or misaligned pointer access, invalid values, broken aliasing, mutating immutable bytes, invalid lifetimes, and wrong assumptions inside `unsafe`. The Rust Reference’s UB list is explicitly serious and also warns it is not exhaustive.
2. **Publishing a safe API that safe callers can use to trigger UB.** That is unsound Rust. This is worse than merely “using unsafe,” because it leaks unsafety into code that appears safe. Real ecosystem bugs include safe Rust triggering UB through incorrect `Send`/`Sync` bounds in `futures-rs`, and GitLab advisories for crates with invalid bit patterns, out-of-bounds iterator advances, and uninitialized `MaybeUninit::assume_init` usage.
3. **Writing `unsafe` without a real safety proof.** “Seems fine,” “the AI said so,” “copied from StackOverflow,” “Miri passed once,” or “the tests pass” is not a proof. The Rust Book recommends keeping unsafe blocks small, wrapping unsafe code in safe abstractions, and using Miri to check unsafe code.
4. **Suppressing lints, warnings, tests, or audits just to get green CI.** Rust and Clippy expose mechanisms to `allow`, `warn`, `deny`, and `forbid` lints; using `allow` to hide a real problem without a reason is a review smell. Clippy also warns that restriction lints should be enabled case-by-case, not blindly.
5. **Treating “it compiles” as “it is correct.”** Rust prevents many memory-safety bugs in safe code, but it does not prove business logic, cancellation safety, panic policy, input validation, resource limits, cryptography, SQL safety, authorization, protocol compatibility, or operational resilience.
6. **Accepting generated Rust without understanding every invariant it relies on.**
7. **Letting an AI introduce `unsafe`, FFI, `Send`, `Sync`, `Pin`, `transmute`, atomics, `static mut`, `no_mangle`, `export_name`, or lifetime tricks without human review.**
8. **Using the compiler as a random patch oracle: adding `clone`, `Arc`, `Mutex`, `'static`, `Box::leak`, `unsafe`, `RefCell`, `Rc`, or `unwrap` until errors disappear.**
9. **Satisfying the borrow checker by changing the program’s ownership model without checking semantics.**
10. **Adding `Arc<Mutex<T>>` everywhere to avoid thinking about ownership.**
11. **Adding `.clone()` everywhere to avoid thinking about borrowing, especially on large data, secret data, locked data, or hot paths.**
12. **Adding `'static` bounds until async code compiles, without proving the data really can live that long.**
13. **Using `Box::leak`, `mem::forget`, global singletons, or lazy statics to silence lifetime errors.**
14. **Replacing precise error types with `anyhow::Error` or strings everywhere in a library API because the generated code was easier.**
15. **Turning recoverable errors into panics because the AI could not thread `Result` through the call graph.**
16. **Turning panics into `Result` while silently losing the invariant violation that the panic was meant to expose.**
17. **Deleting failing tests because generated code changed behavior.**
18. **Weakening tests to match generated output.**
19. **Accepting generated benchmarks that do not measure release builds, realistic data, or optimized-away work.**
20. **Accepting generated documentation that overclaims safety, complexity, performance, or security.**
21. **Shipping generated code that uses crates the team has not reviewed.**
22. **Letting an AI choose dependencies by popularity/name alone.**
23. **Copying code from Reddit, X, StackOverflow, GitHub issues, or an LLM into `unsafe` without going back to primary docs.**
24. **Inventing a `SAFETY:` comment after the fact to appease Clippy.**
25. **Saying “the compiler would catch it” for code involving `unsafe`, FFI, logic, concurrency, IO, or external inputs.**
26. **Trusting AI-generated FFI signatures.**
27. **Trusting AI-generated `repr(C)` structs without checking C headers, padding, alignment, ABI, ownership, and platform differences.**
28. **Trusting AI-generated atomic orderings.**
29. **Trusting AI-generated `Pin` projections.**
30. **Trusting AI-generated parsers for untrusted input without fuzz/property tests.**

## The master rule

1. **Lies to the compiler** about lifetimes, aliasing, initialization, layout, thread-safety, or ownership.
2. **Lies to users** by exposing a safe API that secretly requires unsafe preconditions.
3. **Lies to reviewers** by hiding risk behind `unsafe`, broad `allow`, vague comments, generated code, or “it compiles.”
4. **Turns expected failure into process failure** without a documented contract.
5. **Treats AI output as authority** rather than as untrusted draft code.
6. **Ships code whose safety/security story the author cannot explain.**

## Undefined behavior

1. Data races.
2. Dereferencing null, dangling, or misaligned pointers.
3. Out-of-bounds pointer arithmetic or slice access.
4. Breaking Rust aliasing rules, especially creating multiple active mutable aliases.
5. Mutating memory through a shared reference except through `UnsafeCell`.
6. Producing invalid values such as invalid `bool`, invalid `char`, invalid enum discriminants, invalid references, uninitialized integers, or invalid wide-pointer metadata.
7. Calling a function with the wrong ABI.
8. Unwinding through a stack frame that does not permit it.
9. Incorrect inline assembly.
10. Running code compiled for unsupported CPU target features.
11. Violating Rust runtime assumptions, including stack-frame/destructor assumptions around foreign `longjmp`-style behavior.

## Unsafe blocks and functions

1. **Using `unsafe` as a way to bypass the borrow checker rather than encode the real invariant.**
2. **Writing an unsafe block without a nearby `SAFETY:` explanation.** Clippy’s `undocumented_unsafe_blocks` lint exists because undocumented unsafe blocks and impls are harder to maintain and can hide unsoundness.
3. **Writing a `SAFETY:` comment that restates the code instead of proving the preconditions.**
4. **Writing a false or unverifiable `SAFETY:` comment.**
5. **Putting many unrelated unsafe operations into one block so reviewers cannot audit each obligation separately.** Clippy has a `multiple_unsafe_ops_per_block` restriction lint for this exact auditability reason.
6. **Assuming `unsafe fn` means the whole body should be implicitly unsafe.** Rust 2024 warns by default for unsafe operations inside unsafe functions without explicit unsafe blocks, because conflating “caller has obligations” with “body may perform unsafe operations” was considered too risky.
7. **Making a function safe when the caller must uphold memory-safety preconditions.**
8. **Making a trait safe when implementors must uphold safety invariants that unsafe code relies on.**
9. **Hiding unsafe behavior inside a macro so the callsite looks safe.** Clippy explicitly calls out macros that hide unsafe blocks and allow unsafe operations in seemingly safe code.
10. **Adding `#![allow(unsafe_code)]` or `#[allow(...)]` around unsafe merely to suppress review.**
11. **Using `unsafe` in generated code without marking generated files as such and without a review process.**
12. **Relying on a private invariant from another crate unless that invariant is part of its public, documented contract.**
13. **Depending on debug-only checks to justify release-mode unsafe code.**

## Unsafe traits and marker traits

1. **Incorrectly implementing `Send` or `Sync`.** The Rustonomicon says these are unsafe traits, unsafe code can rely on them being correct, and incorrect implementations can cause UB.
2. **Marking a type `Send` because “it compiles except for this raw pointer.”**
3. **Marking a type `Sync` while it contains unsynchronized interior mutability.**
4. **Treating `Rc`, `RefCell`, `Cell`, raw pointers, or non-thread-safe C handles as thread-safe by wrapping them in newtypes with unsafe impls.**
5. **Implementing unsafe traits like `TrustedLen`, allocator traits, or FFI-related traits without satisfying every invariant.**
6. **Deriving or implementing `Copy`, `Clone`, `Default`, `Deref`, `Drop`, `Send`, or `Sync` in a way that violates ownership or safety invariants.**

## Unsound safe APIs

1. `pub fn from_raw(ptr: *mut T) -> Self` as a safe function when the pointer must be valid, aligned, non-null, uniquely owned, or allocator-compatible.
2. A safe wrapper around C that assumes the C pointer is valid.
3. A safe function that accepts any `usize` and internally calls `get_unchecked`.
4. A safe constructor that can produce invalid internal state.
5. A public field that lets users break invariants relied on by unsafe code.
6. A macro that expands to unsafe behavior while presenting a safe API.
7. A safe `Iterator`, `Read`, `Future`, or callback API whose implementation violates hidden invariants.

## `Pin` and self-referential behavior

1. **Moving a pinned value after promising it will not move.**
2. **Using `Pin::new_unchecked` without proving the pointee will remain pinned.**
3. **Writing manual pin projections without understanding structural pinning.**
4. **Creating self-referential structs with raw pointers and hoping normal moves will not happen.**
5. **Implementing `Unpin` for a type that relies on pinning for soundness.**
6. **Assuming async futures can be moved after pinning just because the code compiles.**
7. **Declaring an `extern` function with the wrong ABI.**
8. **Declaring wrong parameter types, return types, mutability, ownership, or nullability.**
9. **Marking an extern function `safe` when the caller must satisfy pointer, lifetime, thread, initialization, or ownership preconditions.**
10. **Assuming C `int`, `long`, `size_t`, enum layout, struct padding, or calling convention is the same on all targets.**
11. **Using `repr(C)` incorrectly or forgetting it for FFI structs.**
12. **Assuming `repr(C)` fixes Rust enum layouts that C cannot represent.**
13. **Passing Rust references to C that may store them beyond the call.**
14. **Passing pointers to stack values into callbacks that may outlive the stack frame.**
15. **Letting C mutate through a pointer while Rust has live immutable references.**
16. **Letting C read/write Rust-owned memory after Rust has freed or moved it.**
17. **Freeing C-allocated memory with Rust’s allocator or Rust-allocated memory with C’s allocator unless explicitly designed that way.**
18. **Ignoring null returns from C.**
19. **Ignoring error returns, `errno`, or library-specific error-state conventions.**
20. **Converting C strings to Rust strings without checking NUL termination and UTF-8 assumptions.**
21. **Creating `&str` from bytes that are not valid UTF-8.**
22. **Assuming C libraries are thread-safe without checking their docs.**
23. **Calling non-thread-safe C APIs from multiple Rust threads just because Rust types are `Send`.**
24. **Allowing unwinding across an FFI boundary that is not allowed to unwind.**
25. **Not defining ownership of callback userdata.**
26. **Generating bindings with bindgen and treating them as reviewed safe wrappers.**
27. **Putting `unsafe` wrappers around C APIs but failing to encode ownership, lifetime, and thread-safety rules in Rust types.**
28. **Using `no_mangle`, `export_name`, or `link_section` without checking symbol collisions and linker behavior.** Rust 2024 marks these as unsafe attributes because they have soundness requirements the compiler cannot verify.
29. **Using `.unwrap()` on user input, network input, files, config, database output, environment variables, CLI args, or service responses in production paths.**
30. **Using `.expect()` with a vague message like `"should work"` or `"impossible"` when the condition is not actually impossible.**
31. **Using `panic!`, `assert!`, indexing `[]`, `.unwrap()`, `.expect()`, `.next().unwrap()`, `.parse().unwrap()`, or `.lock().unwrap()` as ordinary error handling.**
32. **Panicking inside a function that returns `Result` for the same failure mode.** Clippy’s `panic_in_result_fn` lint exists because some codebases require `Result` functions to return errors rather than crash.
33. **Using `todo!()` or `unimplemented!()` in reachable production code.**
34. **Using `unreachable!()` for states that can be reached through malformed input, version skew, corrupted data, concurrency, or future enum variants.**
35. **Using `assert!` to validate attacker-controlled or user-controlled input.**
36. **Ignoring `Result` or `Option` values with `_ =` when failure matters.**
37. **Calling `.ok()` or `.err()` just to throw away useful error information.**
38. **Using `map_err(|_| ...)` to erase the context needed to debug or handle the failure.**
39. **Returning `Ok(())` after a partial failure.**
40. **Logging an error and continuing as if the operation succeeded.**
41. **Retrying forever without backoff, cap, cancellation, or observability.**
42. **Converting every error into a string in library APIs, preventing callers from matching error kinds.**
43. **Leaking secrets or PII in error messages, panic messages, `Debug`, `Display`, or logs.**
44. **Swallowing task panics in async code by dropping `JoinHandle`s whose results matter.**
45. **Using panics for expected parser failures, HTTP errors, permission errors, missing files, invalid config, rate limits, or unavailable services.** The Rust Book says expected failures are better represented with `Result`.
46. **Panicking in `Drop` for normal cleanup failure.**
47. **Doing blocking or fallible work in destructors without an explicit alternative API.** The Rust API Guidelines say dependable crates should have destructors that never fail and alternatives for destructors that may block.
48. **Documenting a function as infallible when it can panic.**
49. **Failing to document panic conditions in public APIs.** Rust API Guidelines call for a `Panics` section for panic conditions.
50. **Allowing invalid states when the type system could prevent them.**
51. **Using primitive types where a newtype is needed to encode units, ownership, trust level, identity, or validation state.**
52. **Accepting `String` for “validated email,” “SQL identifier,” “path inside base dir,” “secret token,” or “already escaped HTML.”**
53. **Using `bool` parameters that allow meaningless or dangerous combinations in public APIs.**
54. **Using `Option<T>` to mean multiple different things without documenting the distinction.**
55. **Making constructors that can create invalid objects.**
56. **Providing public fields that let callers violate invariants.**
57. **Making fields private but then exposing unsafe mutation through getters or `DerefMut`.**
58. **Relying on documentation alone for an invariant that could be encoded in a type.** Rust API Guidelines recommend leveraging types and validating arguments when practical.
59. **Public unsafe functions without a `# Safety` section.** Clippy’s `missing_safety_doc` warns on this, and the API Guidelines require safety sections explaining caller invariants.
60. **Public functions that can panic without a `# Panics` section.**
61. **Public functions returning `Result` without documenting meaningful error cases.**
62. **Public traits whose implementors must uphold hidden invariants but that are not sealed or unsafe.**
63. **Unsafe code relying on trait method behavior that implementors are not required to provide.**
64. **Using `Default` to create an invalid sentinel value.**
65. **Deriving `Deserialize` in a way that bypasses constructor validation.**
66. **Deriving `Clone` or `Copy` for types that represent unique ownership, locks, handles, or capabilities.**
67. **Implementing `PartialEq`, `Eq`, `Ord`, or `Hash` inconsistently.**
68. **Implementing `Ord` for a relation that is not total.**
69. **Implementing `Hash` inconsistently with `Eq`.**
70. **Implementing `Deref` to pretend a domain type is a collection or primitive when that exposes invalid operations.**
71. **Changing public API behavior without documenting migration.**
72. **Shipping breaking changes as patch/minor releases without a clear semver rationale.** Cargo’s SemVer chapter describes which public API changes are conventionally breaking and require major bumps.
73. **Silently changing feature flags so downstream crates compile different APIs or invariants.**
74. **Re-exporting public dependency types without treating that dependency as part of your semver surface.**
75. **Exposing `unsafe` details through type aliases, public fields, or trait bounds that downstream users cannot reason about.**
76. **Creating a data race through unsafe code.**
77. **Using `unsafe impl Send` or `unsafe impl Sync` to move non-thread-safe state across threads.**
78. **Sharing `Rc`, `RefCell`, raw pointers, or C handles across threads through an unsafe wrapper without synchronization.**
79. **Using `static mut` or global mutable state without synchronization.**
80. **Using atomics with `Relaxed` because it “seemed faster” without a memory-ordering proof.**
81. **Using atomics as a replacement for locks without proving the full protocol.**
82. **Checking a condition, then using unchecked access after another thread can change the condition.**
83. **Holding a blocking mutex guard across `.await` in code that can run concurrently.**
84. **Holding any lock while awaiting network IO, disk IO, user callbacks, or unbounded work unless the lock is explicitly designed for that and the deadlock/cancellation story is proven.**
85. **Blocking the async runtime with CPU-heavy loops, `std::thread::sleep`, synchronous IO, or blocking locks.** Tokio’s `spawn_blocking` docs say blocking or doing lots of compute in a future without yielding is problematic because it can prevent the executor from driving other futures.
86. **Using `spawn_blocking` as an unbounded CPU work queue.** Tokio notes that many CPU-bound computations should be limited with a semaphore or moved to a specialized executor.
87. **Using `spawn_blocking` for long-lived workers instead of dedicated threads.** Tokio recommends dedicated threads for long-lived blocking workloads.
88. **Calling blocking Tokio APIs inside async contexts that are documented to panic.** Tokio’s `Mutex::blocking_lock` panics inside async execution contexts and recommends `spawn_blocking` or `block_in_place`.
89. **Using `tokio::select!` in a loop with cancellation-unsafe operations while assuming progress is preserved.** Tokio documents methods that are not cancellation safe and explains that dropping and recreating a future must be a no-op for cancellation safety.
90. **Dropping futures that perform partial writes, partial reads, or state transitions without a cancellation-safety design.**
91. **Using unbounded channels for untrusted or high-volume input.**
92. **Spawning one task/thread per request without limits.**
93. **Ignoring backpressure.**
94. **Ignoring timeouts.**
95. **Ignoring shutdown and cancellation paths.**
96. **Creating lock-order cycles.**
97. **Using `Mutex` where message passing or ownership transfer would make the invariant clearer.**
98. **Using `Arc<Mutex<T>>` as a default architecture instead of a deliberate synchronization boundary.**
99. **Ignoring poisoned mutexes when the protected data may be inconsistent.**
100. **Detaching tasks that own important work and never observing whether they failed.**
101. **Trusting external input because Rust is memory-safe.**
102. **Treating config, database rows, environment variables, CLI args, HTTP headers, JSON, protobufs, file contents, or feature flags as infallible.**
103. **Using `unwrap`/`expect` on external input in request paths.**
104. **Deserializing untrusted bytes directly into trusted domain types without validation.**
105. **Using `unsafe` or `transmute` to parse network, disk, or IPC bytes.**
106. **Assuming `serde` validation is the same as domain validation.**
107. **Accepting unbounded input into `String`, `Vec`, `read_to_end`, `collect`, or recursive structures.**
108. **Building paths by string concatenation with user input.**
109. **Allowing `..`, symlinks, absolute paths, or platform-specific path tricks to escape a base directory.**
110. **Using `Command::new("sh").arg("-c").arg(user_input)` or equivalent shell injection patterns.**
111. **Building SQL, LDAP, HTML, shell, regex, or path expressions by concatenating unsanitized strings.**
112. **Disabling TLS certificate or hostname validation.**
113. **Rolling your own cryptography.**
114. **Using non-cryptographic RNG for secrets, tokens, session IDs, keys, or nonces.**
115. **Comparing secrets with normal equality when timing side channels matter.**
116. **Logging secrets, tokens, passwords, session cookies, Authorization headers, private keys, PII, or raw request bodies.**
117. **Deriving `Debug` for secret-bearing types without redaction.**
118. **Using `Debug` output as a stable or user-facing format.**
119. **Returning internal errors, paths, SQL, stack traces, or panic payloads to users.**
120. **Ignoring authentication or authorization errors by converting them to default values.**
121. **Treating “parse failed” as “use default allow.”**
122. **Failing open on security checks.**
123. **Caching authorization decisions without invalidation strategy.**
124. **Using world-writable temp paths unsafely.**
125. **Ignoring TOCTOU races around files, symlinks, and permissions.**
126. **Storing passwords without a password hashing scheme designed for passwords.**
127. **Not rotating or revoking secrets after exposure.**
128. **Using unsafe FFI crypto bindings without checking initialization, thread safety, return codes, and zeroization requirements.**
129. **Assuming dependency code is safe because it is Rust.**
130. **Ignoring RustSec/GitHub/GitLab advisories.** RustSec exists specifically as a vulnerability database for Rust crates and provides `cargo-audit`; GitLab’s advisory database also tracks Cargo advisories.
131. **Adding a crate without checking maintenance, license, repository, recent releases, unsafe usage, downloads only in context, issue history, and transitive dependencies.**
132. **Adding a crate because the name looks right without checking typosquatting risk.**
133. **Ignoring `build.rs` and proc-macro risk. Build scripts and proc macros execute code during build.**
134. **Using unreviewed Git dependencies in production.**
135. **Depending on a moving Git branch without a pinned revision.** Cargo’s own guide explains how relying on the latest default branch can produce different builds later, and how `Cargo.lock` records exact revisions.
136. **Not committing `Cargo.lock` for applications, services, CLIs, or deployable binaries.** Cargo says when in doubt, check `Cargo.lock` into version control.
137. **Manually editing `Cargo.lock` instead of using Cargo.**
138. **Using wildcard dependencies or overly broad ranges without a reason.**
139. **Ignoring yanked, unmaintained, deprecated, or vulnerable crates.**
140. **Letting Dependabot/Renovate/cargo-audit findings rot without triage.**
141. **Using `cargo update` blindly right before release.**
142. **Using old transitive versions because “it still builds.”**
143. **Enabling default features without knowing what they pull in.**
144. **Enabling optional features that change security, networking, TLS, serialization, or FFI behavior without review.**
145. **Vendoring C/C++ libraries through `-sys` crates and forgetting they need security updates too.**
146. **Depending on a crate’s undocumented behavior.**
147. **Depending on private modules, layout, error strings, or debug output from another crate.**
148. **Publishing a crate with path/git dependencies that cannot resolve for users.** Cargo documents that crates.io does not allow packages to be published with dependencies outside crates.io except dev-dependencies.
149. **Failing to document MSRV when your users need it.**
150. **Changing MSRV unexpectedly in a patch release for a library used by others.**
151. **Ignoring license incompatibilities.**
152. **Copying code into a crate without attribution or license compliance.**
153. **Publishing AI-generated code with invented provenance or license assumptions.**
154. **Merging code with failing tests.**
155. **Deleting tests instead of fixing the regression.**
156. **Weakening assertions until generated code passes.**
157. **Only testing happy paths for parsers, network protocols, storage formats, auth logic, or unsafe abstractions.**
158. **No regression test for a fixed bug.**
159. **No tests for documented panic/error behavior.**
160. **No tests for boundary values: empty, one, max, overflow, invalid UTF-8, invalid enum, large input, duplicate input, partial input, interrupted IO.**
161. **No tests for feature-flag combinations that users can enable.**
162. **No tests for no-default-features if you claim to support it.**
163. **No tests for MSRV if you claim an MSRV.**
164. **No tests for documented examples.**
165. **No property tests or fuzzing for untrusted parsers or complex state machines.**
166. **No Miri run for crates with nontrivial unsafe code.** Miri is an official UB detection tool for Rust that can run binaries and tests and catch unsafe code that violates safety requirements.
167. **No sanitizer strategy for FFI-heavy or unsafe-heavy code.** Rust’s sanitizer docs describe AddressSanitizer catching out-of-bounds access, use-after-free, double free, invalid free, and leaks.
168. **Treating Miri, fuzzing, or sanitizers as proof of correctness. They find bugs; they do not prove all invariants.**
169. **Ignoring flaky tests.**
170. **Hiding nondeterminism with sleeps instead of synchronization.**
171. **Using CI that does not run workspace tests.**
172. **Only running default features when non-default features are released.**
173. **Not running Clippy or rustfmt in CI for a maintained codebase.**
174. **Using `#[allow]` without a reason.**
175. **Using `#[cfg(test)]` to change production invariants.**
176. **Testing with mocks that remove the failure modes production code must handle.**
177. **Shipping generated code that was never formatted, linted, or reviewed.**
178. **Treating coverage percentage as a substitute for invariant testing.**
179. **Unbounded allocation from untrusted input.**
180. **Reading entire request bodies, files, or streams into memory without a size limit.**
181. **Recursive parsing without depth limits.**
182. **Using unbounded queues/channels for untrusted traffic.**
183. **Spawning unbounded tasks, threads, or blocking jobs.**
184. **Holding locks while doing slow IO, logging, formatting, allocation-heavy work, or user callbacks.**
185. **Using `clone` to quiet borrow errors in hot paths without measuring cost.**
186. **Using `String` concatenation in loops where it creates avoidable quadratic behavior.**
187. **Calling `.collect::<Vec<_>>()` just to iterate once.**
188. **Allocating in a hot path while claiming zero-copy or low latency.**
189. **Using `Debug` formatting in hot paths accidentally.**
190. **Using `println!`/`eprintln!` in libraries or hot production paths instead of structured logging or returning errors.**
191. **Leaking memory intentionally to solve ownership without documenting process lifetime and bounds.**
192. **Using `Box::leak` or global caches with unbounded cardinality.**
193. **Assuming async makes blocking work cheap.**
194. **Using `spawn_blocking` for unbounded CPU work.**
195. **Using a single global mutex around high-volume state.**
196. **Claiming performance wins without benchmarks.**
197. **Benchmarking debug builds.**
198. **Benchmarking toy inputs and extrapolating to production.**
199. **Benchmarking code that the optimizer removed.**
200. **Ignoring allocator behavior when performance depends on allocation.**
201. **Optimizing by adding unsafe before measuring safe alternatives.**
202. **No logs, metrics, or traces for important error paths.**
203. **Logging too much sensitive data instead of structured, redacted context.**
204. **Panicking in a service without supervision, restart policy, alerting, and rate limits.**
205. **Restart loops with no circuit breaker.**
206. **Assuming generated config is valid because the generator “should” be correct.**
207. **Failing to validate config before rollout.**
208. **Failing to stage config/schema/permission changes.**
209. **No kill switch for risky features.**
210. **No rollback path for migrations.**
211. **Deploying Rust services as if memory safety replaces operational safety.**
212. **Ignoring partial rollout signals.**
213. **Treating panics as invisible if the process restarts.**
214. **Dropping async task errors so production failures vanish.**
215. **Hiding error context from operators.**
216. **Letting debug-only logs be the only way to diagnose production failure.**
217. **Documenting unsafe code incompletely.**
218. **Documenting unsafe code incorrectly.**
219. **Saying “safe” when safety depends on undocumented caller behavior.**
220. **Omitting `# Safety` from public unsafe functions.**
221. **Omitting `# Panics` from public functions with known panic conditions.**
222. **Omitting error conditions from public fallible APIs.**
223. **Documenting examples that do not compile.**
224. **Documenting examples that use `unwrap` in ways users will copy into production without a warning.**
225. **Claiming constant time, lock-free, wait-free, zero-copy, allocation-free, thread-safe, or panic-free behavior without tests or proof.**
226. **Claiming semver compatibility while changing public behavior.**
227. **Leaving stale docs after changing invariants.**
228. **Using comments to contradict code.**
229. **Using comments to justify behavior the code does not actually enforce.**
230. **Hiding safety-critical assumptions in PR comments instead of source docs.**
231. **Failing to document feature flags that affect behavior, dependencies, or API.**
232. **Generating unsafe code without making that obvious to reviewers.**
233. **Generating unsafe code from untrusted input.**
234. **Hiding unsafe operations inside macros so callsites appear safe.**
235. **Generating public APIs that differ by feature flag without documentation.**
236. **Generating code that bypasses validation constructors.**
237. **Generating `Send`/`Sync` impls automatically without checking fields and invariants.**
238. **Generating FFI bindings and treating them as safe wrappers.**
239. **Generating `Deserialize` impls that allow invalid domain values.**
240. **Using proc macros from unreviewed crates in security-sensitive builds.**
241. **Letting `build.rs` download or execute unpinned external code during builds.**
242. **Committing generated code without recording generator version and inputs.**
243. **Not reviewing generated diffs.**
244. **Making generated code non-reproducible.**
245. **Hand-editing generated files without feeding changes back to the generator.**
246. **Using macros to create clever APIs that hide lifetimes, ownership, panics, or allocation.**
247. **Parsing untrusted data with `unwrap`.**
248. **Parsing untrusted data with unchecked indexing.**
249. **Parsing untrusted data with `unsafe` layout casts.**
250. **Assuming network byte order equals native byte order.**
251. **Assuming struct layout equals wire layout.**
252. **Assuming enum discriminants are valid.**
253. **Ignoring duplicate fields when duplicates are security-relevant.**
254. **Ignoring unknown fields when unknown fields should reject the message.**
255. **Accepting partial parses as full success.**
256. **Not bounding recursion depth.**
257. **Not bounding list/map/string sizes.**
258. **Not handling invalid UTF-8 explicitly.**
259. **Using lossy conversion where exactness matters.**
260. **Using floats for money, counters, authorization thresholds, or exact protocol fields.**
261. **Not canonicalizing before signing or verifying.**
262. **Signing one representation and verifying another.**
263. **Using `Debug` or `Display` as a serialization format.**
264. **Changing serialized format without versioning.**
265. **Not testing backward/forward compatibility.**
266. **Deserializing directly into privileged internal types instead of validating into them.**
267. **Releasing without knowing what code and dependencies are in the artifact.**
268. **Publishing a crate with known unsoundness and no warning.**
269. **Publishing a security fix without disclosing enough for users to act.**
270. **Silently breaking semver.**
271. **Changing public types, trait bounds, feature defaults, MSRV, panic behavior, or error behavior without release notes when users rely on them.**
272. **Yanking or replacing releases to hide mistakes without documenting impact.**
273. **Tagging a release that does not match the published crate.**
274. **Publishing from a dirty working tree.**
275. **Publishing generated artifacts that cannot be reproduced.**
276. **Using local path patches accidentally in release builds.**
277. **Relying on environment-specific build behavior.**
278. **Letting `build.rs` depend on undeclared system state.**
279. **Not checking license files and included sources before publishing.**
280. **Not running tests on the exact feature set being released.**
281. **Not running docs for public APIs.**
282. **Not checking package contents with `cargo package --list` or equivalent review.**
283. **Shipping debug assertions as the only enforcement of release invariants.**
284. **Shipping release builds that depend on debug-only behavior.**
285. **Hiding a soundness bug.**
286. **Hiding a vulnerability report.**
287. **Knowingly leaving users on a vulnerable version without advisory, yank, patch, or migration guidance.**
288. **Marking a vulnerability as “not security” because it is “only a panic” when it is remotely triggerable denial of service.**
289. **Marking UB as harmless because “it works on my machine.”**
290. **Dismissing Miri, sanitizer, or fuzz findings without root-cause analysis.**
291. **Closing soundness issues because they require a breaking API fix.**
292. **Refusing a semver break when the current API is unsound.**
293. **Adding a malicious or surprising `build.rs`.**
294. **Adding telemetry, network calls, or data collection in build scripts or proc macros without disclosure.**
295. **Namesquatting, typosquatting, or publishing lookalike crates.**
296. **Copying code without license compliance.**
297. **Claiming “memory safe” for a crate with unaudited unsafe code exposed through safe APIs.**
298. **Claiming “production ready” for generated or prototype code with no maintenance plan.**
299. **Leaving abandoned crates with known breakage and no deprecation notice if users are actively affected.**
300. **`unsafe`** — acceptable when necessary, minimal, documented, reviewed, and tested.
301. **`unwrap` / `expect`** — acceptable in tests, examples, prototypes, or proven invariants; bad when used on expected/external failures.
302. **`clone`** — acceptable when cheap or semantically needed; bad when used to avoid ownership reasoning.
303. **`Arc<Mutex<T>>`** — acceptable for shared mutable state; bad as a default architecture.
304. **`RefCell` / `Cell`** — acceptable for interior mutability; bad when used to dodge design.
305. **`Box<dyn Trait>`** — acceptable for dynamic dispatch; bad when used to hide type confusion or object-safety issues.
306. **Macros** — acceptable for reducing boilerplate; bad when they hide control flow, panics, allocations, or unsafe.
307. **Nightly Rust** — acceptable for deliberate reasons; bad when used because generated code wanted an unstable shortcut.
308. **Panics** — acceptable for broken invariants; bad for expected runtime failure.
309. **Broad error types** — acceptable at application boundaries; bad in library APIs where callers need structured handling.

## References

[1] https://doc.rust-lang.org/reference/behavior-considered-undefined.html "Behavior considered undefined - The Rust Reference"
[2] https://arxiv.org/html/2604.27001v1 "An Empirical Security Evaluation of LLM-Generated Cryptographic Rust Code"
[3] https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html "Unsafe Rust - The Rust Programming Language"
[4] https://std-dev-guide.rust-lang.org/policy/safety-comments.html "Safety comments policy - Standard library developers Guide"
[5] https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html "Unsafe attributes - The Rust Edition Guide"
[6] https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html "To panic! or Not to panic! - The Rust Programming Language"
[7] https://rust-lang.github.io/api-guidelines/checklist.html "Checklist - Rust API Guidelines"
[8] https://github.com/base/base/blob/main/.github/workflows/claude-review.yml "base/.github/workflows/claude-review.yml at main · base/base · GitHub"
[9] https://doc.rust-lang.org/cargo/reference/features.html "Features - The Cargo Book"
[10] https://docs.rs/cargo-deny "cargo_deny - Rust"
[11] https://www.usenix.org/publications/loginonline/we-have-package-you-comprehensive-analysis-package-hallucinations-code "Package Hallucinations: How LLMs Can Invent Vulnerabilities | USENIX"
[12] https://doc.rust-lang.org/stable/clippy "Introduction - Clippy Documentation"
[13] https://www.reddit.com/r/rust/comments/1mnafii/what_are_the_most_common_mistakes_and_codesmells/ "What are the most common mistakes and code-smells that newbies make? : r/rust"
[14] https://github.com/rust-lang/futures-rs/issues/2239 "MappedMutexGuard Send/Sync bound is unsound · Issue #2239 · rust-lang/futures-rs · GitHub"
[15] https://doc.rust-lang.org/reference/attributes/diagnostics.html "Diagnostics - The Rust Reference"
[16] https://rust-lang.github.io/rust-clippy/master/index.html "Clippy Lints"
[17] https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html "unsafe_op_in_unsafe_fn warning - The Rust Edition Guide"
[18] https://doc.rust-lang.org/nomicon/uninitialized.html "Uninitialized Memory - The Rustonomicon"
[19] https://doc.rust-lang.org/std/mem/fn.transmute.html "transmute in std::mem - Rust"
[20] https://doc.rust-lang.org/nomicon/send-and-sync.html "Send and Sync - The Rustonomicon"
[21] https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html "Unsafe extern blocks - The Rust Edition Guide"
[22] https://rust-lang.github.io/api-guidelines/documentation.html "Documentation - Rust API Guidelines"
[23] https://rust-lang.github.io/api-guidelines/dependability.html "Dependability - Rust API Guidelines"
[24] https://doc.rust-lang.org/cargo/reference/semver.html "SemVer Compatibility - The Cargo Book"
[25] https://doc.rust-lang.org/nomicon/races.html "Races - The Rustonomicon"
[26] https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html "spawn_blocking in tokio::task - Rust"
[27] https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html "Mutex in tokio::sync - Rust"
[28] https://docs.rs/tokio/latest/tokio/macro.select.html "select in tokio - Rust"
[29] https://rustsec.org/ "About RustSec › RustSec Advisory Database"
[30] https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html "Cargo.toml vs Cargo.lock - The Cargo Book"
[31] https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html "Specifying Dependencies - The Cargo Book"
[32] https://doc.rust-lang.org/cargo/commands/cargo-test.html "cargo test - The Cargo Book"
[33] https://github.com/rust-lang/miri/?utm_source=chatgpt.com "rust-lang/miri"
[34] https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html?utm_source=chatgpt.com "sanitizer - The Rust Unstable Book"
[35] https://blog.cloudflare.com/18-november-2025-outage/?utm_source=chatgpt.com "Cloudflare outage on November 18, 2025"
[36] https://doc.rust-lang.org/nomicon/working-with-unsafe.html "Working with Unsafe - The Rustonomicon"
[37] https://www.reddit.com/r/rust/comments/jkortm/what_are_the_bad_practices_in_rust_that_should_be/ "What are the bad practices in rust that should be avoided? : r/rust"
[38] https://tokio.rs/tokio/tutorial/shared-state "Shared state | Tokio - An asynchronous Rust runtime"
[39] https://github.com/tokio-rs/loom "GitHub - tokio-rs/loom: Concurrency permutation testing tool for Rust. · GitHub"
[40] https://www.reddit.com/r/rust/comments/1scqazw/slopc_a_proc_macro_that_replaces_todo_with/?utm_source=chatgpt.com "r/rust - slopc: a proc macro that replaces todo!() with LLM ..."
[41] https://rustsec.org/?utm_source=chatgpt.com "About RustSec › RustSec Advisory Database"
[42] https://doc.rust-lang.org/cargo/reference/features.html?utm_source=chatgpt.com "Features - The Cargo Book"
[43] https://github.com/rust-lang/miri/blob/master/README.md "miri/README.md at master · rust-lang/miri · GitHub"
[44] https://rust-fuzz.github.io/book/cargo-fuzz.html "Fuzzing with cargo-fuzz - Rust Fuzz Book"
