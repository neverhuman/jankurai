# Bad TYPESCRIPT Behavior: Comprehensive Guide

This document organizes the worst TYPESCRIPT behaviors that are inexcusable in production.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core TYPESCRIPT best practices:

- **Eliminate `any` types**: Use `unknown` for unvalidated data and enforce narrowing with type guards.
- **Validate data at the boundaries**: Use schema validation libraries (e.g., Zod) for API responses and inputs.
- **Prefer strict mode**: Enable `"strict"
- **Keep types aligned with runtime reality**: Avoid lying to the compiler with type assertions or non-null assertions.
- **Model illegal states out**: Use discriminated unions to represent mutually exclusive states.

## 1. Silencing TypeScript instead of fixing the problem

1. Committing `// @ts-nocheck` in source files.
2. Using `// @ts-ignore` to make a red squiggle disappear.
3. Using `// @ts-expect-error` without a precise explanation.
4. Leaving `@ts-expect-error` in place after the underlying error is gone.
5. Disabling a lint rule at file level because one line is hard.
6. Disabling type-aware linting because it exposes too many real problems.
7. Changing `.ts` / `.tsx` files back to `.js` just to escape type errors.
8. Excluding broken source folders from `tsconfig.json`.
9. Making CI run tests but not `tsc --noEmit`.
10. Allowing deployments when `tsc` reports errors.
11. Using `noEmitOnError: false` in a deploy/build path so broken code still emits JavaScript.
12. Turning off strict compiler options after the project has already adopted TypeScript.
13. Changing types until the compiler shuts up without understanding why.
14. Asking an AI agent to “fix TypeScript errors” and accepting a diff full of casts, `any`, or suppressions.
15. Hiding errors inside wrapper functions named things like `unsafeCast`, `forceType`, `fixType`, or `asWhatever`.
16. Treating TypeScript errors as “compiler drama” rather than design feedback.

## 1. Type-system escape hatches

1. Using `any` in handwritten production code.
2. Using `as any`.
3. Using `as unknown as T` to force a value into a type.
4. Using `unknown` as a cosmetic replacement for `any`, then immediately casting it away.
5. Returning `any` from public functions.
6. Accepting `any` in public APIs.
7. Storing `any` in state, context, caches, services, stores, Redux/Zustand/Pinia/etc.
8. Using `Record<string, any>`.
9. Using `Array<any>`, `any[]`, `Promise<any>`, `Map<string, any>`, `Set<any>`.
10. Passing `any` into generics, such as `UseQueryResult<any>` or `Response<any>`.
11. Using `Function` instead of a real function signature.
12. Using `{}` as “any object.”
13. Using `object` when the code needs a real object shape.
14. Using boxed primitive types: `String`, `Number`, `Boolean`, `Symbol`, `Object`.
15. Using `T = any` as a generic default.
16. Using `T extends any`.
17. Using unconstrained generics when the implementation assumes properties exist.
18. Using unused generic parameters.
19. Using a generic only to avoid writing a real type.
20. Using `never` to silence impossible-state errors instead of modeling the state correctly.
21. Typing a value as `unknown` but never narrowing it.
22. Typing parsed JSON as `SomeType` without validation.
23. Typing `fetch().json()` as a trusted domain object without schema validation.
24. Typing environment variables as present without checking them.
25. Typing feature flags, cookies, route params, search params, localStorage, sessionStorage, postMessage, WebSocket messages, queue messages, or database rows as trusted without validation.
26. Using type assertions to access properties on external input.
27. Using type assertions to bypass discriminated-union narrowing.
28. Using type assertions to silence excess property errors.
29. Using type assertions to bypass readonly/immutability.
30. Using type assertions to bypass `null`/`undefined`.
31. Using type assertions to make tests pass without proving behavior.
32. Using type assertions in reducers/state transitions instead of modeling the transition.
33. Casting an API response to a generated type while ignoring version drift.
34. Casting third-party library results instead of wrapping/narrowing them.
35. Casting `Error` values in `catch` instead of narrowing from `unknown`.
36. Casting DOM elements without checking they exist and have the expected type.
37. Casting `querySelector` results and immediately dereferencing them.
38. Casting dates, decimals, IDs, or branded values from strings without parsing/validating.
39. Casting string literals into union members instead of deriving/validating them.
40. Casting object keys to `keyof T` when the object was not proven to contain the key.

## 10. Module, import, and build misconduct

1. Using `require` in modern TypeScript modules without a clear interop reason.
2. Mixing CJS and ESM casually.
3. Using `esModuleInterop`/synthetic default imports to paper over incorrect runtime assumptions.
4. Importing types as values and causing runtime side effects.
5. Importing values only used as types.
6. Not using `import type` where required by the project’s module settings.
7. Relying on path aliases that only TypeScript understands.
8. Relying on `ts-node` behavior that production Node does not share.
9. Relying on bundler-only resolution in library code.
10. Publishing packages without checking the emitted JS actually resolves.
11. Publishing packages whose `.d.ts` files reference private/internal paths.
12. Publishing packages without declarations when consumers need types.
13. Publishing broken `exports`/`types` mappings.
14. Publishing ESM/CJS dual packages without testing both consumption modes.
15. Importing from package internals that are not public API.
16. Creating circular dependencies that produce partially initialized values.
17. Creating barrel files with hidden side effects.
18. Using barrels that destroy tree-shaking or create circular imports.
19. Depending on import order for side effects.
20. Using global augmentations when explicit imports would work.
21. Polluting global namespaces.
22. Using triple-slash references instead of normal imports except where specifically required.
23. Mixing test-only imports into production code.
24. Accidentally including test files in production builds.
25. Accidentally excluding source files from type checking.
26. Emitting JS into `src`.
27. Shipping source maps that reveal secrets or private source when that is not intended.
28. Using stale module-resolution modes in new projects.
29. Ignoring TypeScript’s host/runtime module-model guidance.

## 10. Unsafe async and promise handling

1. Floating promises.
2. Missing `await`.
3. `if (somePromise)` instead of `if (await somePromise)`.
4. Unhandled promise rejections.
5. `array.forEach(async item => ...)`.
6. Async callbacks passed where return value is ignored.
7. Async React event/effect code with unhandled errors.
8. Fire-and-forget promises without explicit `void`, logging, cancellation, and error handling.
9. `Promise.all` used when partial failure must be handled.
10. `Promise.all` over unbounded user-sized arrays.
11. Swallowed `.catch(() => {})`.
12. `try/catch` around code that starts promises but does not await them.
13. Tests that forget to `await expect(...).resolves/rejects`.
14. API handlers that start background work and return success before work is durably queued.
15. Mixing callbacks and promises in the same code path.
16. Async constructors or initialization hidden behind half-initialized objects.
17. Using sleeps/timeouts instead of awaiting deterministic signals.
18. Ignoring `AbortSignal`.
19. Not cleaning timers, subscriptions, sockets, or event listeners.

## 11. Bad error handling

1. `catch (e: any)`.
2. Empty `catch {}` blocks.
3. Logging and continuing after unrecoverable errors.
4. Throwing strings.
5. Throwing plain objects with no stack.
6. Replacing specific errors with `throw new Error("failed")` and losing context.
7. Ignoring `cause`.
8. Returning `null` for errors in one function, throwing in another, and returning `{ error }` elsewhere with no convention.
9. Treating validation errors as impossible.
10. Swallowing parse errors and using defaults silently.
11. Catching errors only to satisfy TypeScript.
12. Logging secrets/tokens/PII in error objects.
13. Sending raw internal errors to clients.
14. Hiding async errors in detached promises.
15. Converting all errors to `any`.
16. Assuming `catch` values are `Error`.

## 11. Declaration-file and library typing misconduct

1. Writing `.d.ts` files that lie about runtime behavior.
2. Declaring `module "*"` as `any`.
3. Declaring missing packages as `any` instead of minimal safe types.
4. Publishing declarations not generated from source.
5. Publishing declarations from code that failed type checking.
6. Publishing declarations that expose private implementation details.
7. Publishing declarations with broad `any`.
8. Publishing declarations with wrong nullability.
9. Publishing declarations with wrong optionality.
10. Publishing declarations with wrong overload order.
11. Publishing declarations with callback arity mistakes.
12. Publishing declarations with unused generics.
13. Publishing declarations that use boxed primitive types.
14. Publishing declarations that hide thrown/rejected error behavior.
15. Publishing declarations that omit side effects.
16. Publishing declarations that describe a different version of the runtime package.
17. Modifying global interfaces from a library without an extremely explicit reason.
18. Using declaration merging to hide design problems.
19. Using ambient declarations where explicit modules would be safer.
20. Letting generated declarations drift from implementation.
21. Not testing public type behavior with type-level tests.

## 12. Fake domain modeling

1. `type Status = string` when only specific statuses are allowed.
2. `type Role = string`.
3. `type UserId = string` interchangeable with `OrgId`, `PostId`, `SessionId`.
4. Boolean flag explosions like `{ isAdmin, isOwner, isEditor, isViewer }`.
5. Optional-field blobs representing multiple states.
6. `interface Thing { type: string }` instead of discriminated unions.
7. State machines represented as loose objects.
8. Magic strings copied across files.
9. Dates represented inconsistently as `Date`, ISO string, timestamp number, and nullable string.
10. Money represented as floating-point numbers without currency/minor-unit rules.
11. Permissions represented only in UI components.
12. Domain invariants only in comments.
13. `Partial<T>` used as a substitute for proper create/update DTOs.
14. `Record<string, T>` used when keys are a known union.
15. `object` / `{}` used instead of real shapes.
16. Optional properties used because the AI does not know which state owns which fields.
17. One “MegaType” shared across frontend, backend, DB, and API even though each layer has different invariants.

## 13. Non-exhaustive logic

1. `switch` over a union without exhaustiveness checking.
2. `default: return null` hiding unhandled cases.
3. Adding a new union member without updating all handlers.
4. `if/else` ladders over literal states with no final `never` check.
5. Reducers that ignore unknown actions.
6. State machines that silently fall back to initial state.
7. Permission checks with catch-all allow behavior.
8. Feature flag variants with no default-deny path.
9. Enum/string union conversions with unchecked casts.
10. `as never` to make exhaustive checks pass.

## 14. Bad public API types

1. Exported functions with inferred accidental return types when the return shape is part of the contract.
2. Public APIs returning `any`, `unknown`, `{}`, `object`, or `Record<string, unknown>` without a reason.
3. Package `.d.ts` files that lie about runtime behavior.
4. `declare module "foo";` to silence missing types.
5. `declare module "*";`.
6. Global augmentations that affect the whole app unnecessarily.
7. Re-exporting unstable internal types as public API.
8. Changing exported types without versioning.
9. Publishing generated types that were not produced by CI/codegen.
10. Mismatched runtime exports and type declarations.
11. Type-only breaking changes treated as harmless.
12. Exporting broad DTOs that let callers construct invalid states.
13. Public generics that accept impossible combinations.
14. `Function`, `Object`, `String`, `Number`, or `Boolean` in public types.

## 15. Dependency and package-management misconduct

1. Adding a dependency instead of writing five lines of clear code.
2. Adding a dependency suggested by AI without checking maintenance, license, size, and transitive dependencies.
3. Adding a dependency with no types and then declaring it as `any`.
4. Adding stale `@types/*` packages that do not match runtime package versions.
5. Ignoring peer dependency warnings.
6. Ignoring duplicate TypeScript versions in a monorepo.
7. Ignoring duplicate React/framework versions that break type identity.
8. Ignoring lockfile diffs.
9. Not committing lockfiles for apps.
10. Committing generated package-manager artifacts inconsistently.
11. Mixing npm/yarn/pnpm/bun lockfiles casually.
12. Using `latest` in production dependency specs.
13. Using unpinned codegen tools.
14. Using unpinned schema/codegen outputs.
15. Updating TypeScript, framework, or build tools without reading breaking changes.
16. Updating generated types without updating validators/tests.
17. Using abandoned packages for core paths.
18. Using packages with broad `any` types as if they were type-safe.
19. Using packages that require unsafe globals/polyfills without isolating them.
20. Using type packages that shadow real package-bundled types.
21. Patching `node_modules` manually.
22. Publishing internal packages without semantic version discipline.
23. Publishing breaking type changes as patch releases.

## 15. Generic type abuse

1. Generics that do not relate input to output.
2. `<T>(x: unknown) => x as T`.
3. `<T = any>`.
4. `<T extends object>` when primitives are valid or when the shape is known.
5. Fake type-safe fetch helpers: `api.get<T>(url): Promise<T>` with no decoder.
6. Generic repositories/services that erase domain constraints.
7. Deep conditional types no one can debug.
8. Recursive mapped types that destroy compiler performance.
9. Over-abstracted utility types instead of a simple interface.
10. Type-level programming to avoid writing runtime logic.
11. `keyof any` where `PropertyKey` or a narrower key type is intended.
12. `T as unknown as U` inside generic helpers.
13. Generic “form builders” that allow impossible field/value pairs.
14. Using `Partial<T>`, `Pick<T>`, and `Omit<T>` mechanically instead of designing boundary types.
15. Overusing `ReturnType<typeof fn>` across module boundaries where an explicit contract is clearer.
16. Hiding breaking changes behind utility types.

## 16. Maintainability and readability misconduct

1. Creating types no human can understand.
2. Creating type gymnastics to avoid simpler runtime code.
3. Encoding business logic only in types.
4. Encoding validation only in types.
5. Writing clever conditional types where explicit types would be clearer.
6. Writing 200-line inferred types that destroy editor performance.
7. Using recursive types without a real need.
8. Using deeply nested mapped/conditional/template-literal types without tests.
9. Duplicating domain types instead of importing the source of truth.
10. Creating “almost the same” types for the same concept.
11. Naming types `Data`, `Info`, `Props2`, `NewType`, `ResponseType`, `TData` everywhere.
12. Naming variables `data`, `obj`, `item`, `value`, `res`, `payload` when the domain meaning matters.
13. Leaving TODOs without owner/context.
14. Leaving “temporary” casts.
15. Leaving commented-out code.
16. Leaving obsolete comments.
17. Adding comments that merely narrate code.
18. Adding AI-style comments that overclaim safety.
19. Letting files grow into grab bags.
20. Letting utility folders become junk drawers.
21. Creating circular imports because code was copied into the nearest file.
22. Creating duplicated helpers because search was not done.
23. Creating abstractions before two or three real use cases exist.
24. Hiding side effects in utility functions.
25. Hiding network calls in innocent-looking helpers.
26. Hiding state mutation in selectors.
27. Mixing validation, transformation, fetching, rendering, and persistence in one function.
28. Using global mutable state without clear ownership.
29. Using singleton services that make tests order-dependent.
30. Creating untyped event buses.
31. Creating stringly typed event names and payloads.
32. Creating implicit conventions instead of typed interfaces.
33. Using magic strings for routes, permissions, feature flags, event names, cache keys, or storage keys.
34. Using magic numbers for timeouts, retries, pagination, limits, and money.
35. Failing to centralize domain constants where the domain requires consistency.

## 16. Unsafe indexed access

1. `arr[0].id` without checking length.
2. `record[key].value` when `key` may be absent.
3. `map.get(key).value`.
4. `object[someString]` without an index signature or key guard.
5. Treating `process.env.X` as definitely present.
6. Treating URL params as definitely present.
7. Using `!` after every indexed access.
8. Turning off `noUncheckedIndexedAccess` because it is “annoying.”
9. Using `Record<string, T>` for maps where absence is possible but not modeled.
10. Assuming non-empty arrays without a `NonEmptyArray` type or runtime check.
11. Indexing tuple-like arrays as if every position exists.
12. Relying on array order from external APIs.

## 17. Truthiness bugs

1. `if (count)` when `0` is valid.
2. `if (name)` when empty string is valid.
3. `if (value)` for `number | undefined`.
4. `if (obj.prop)` when `false`, `0`, or `""` are legitimate.
5. `foo || default` when `foo` may validly be `0`, `false`, or `""`.
6. `Boolean(value)` as validation.
7. Truthy checks on promises.
8. Truthy checks on arrays to mean “non-empty.”
9. Truthy checks on objects to mean “has keys.”
10. Optional chaining that masks missing required data.
11. Nullish and falsy checks mixed casually.

## 18. Unsafe object/dictionary patterns

1. Using plain objects as maps with untrusted keys.
2. Prototype pollution risks via `Object.assign(target, userInput)`.
3. Spreading unvalidated input into domain objects.
4. Accepting unexpected extra keys in security-sensitive objects.
5. Using `{ [key: string]: any }`.
6. Mutating shared config objects.
7. Returning internal mutable objects from APIs.
8. Casting `readonly` objects to mutable.
9. Using `delete` to change object shapes instead of modeling variants.
10. Relying on property order.
11. Deep cloning with `JSON.parse(JSON.stringify(...))` when Dates, Maps, Sets, `undefined`, BigInt, or class instances matter.
12. Comparing objects by `JSON.stringify`.
13. Confusing absent property with property set to `undefined`.
14. Ignoring `exactOptionalPropertyTypes`.

## 19. Unsafe module/import/build behavior

1. Runtime path aliases that only TypeScript understands.
2. Build succeeds but runtime cannot resolve imports.
3. Type-only imports emitted as runtime imports by mistake.
4. Value imports used only as types.
5. Side-effect imports nobody checks.
6. Circular dependencies that produce partially initialized modules.
7. Mixing ESM/CJS randomly.
8. Relying on `esModuleInterop` to hide incorrect import semantics without understanding runtime output.
9. Import casing that works on macOS but fails on Linux.
10. `require` inside typed ESM code without a boundary.
11. Dynamic imports from user input.
12. Barrel files that accidentally create cycles or huge import graphs.
13. Multiple versions of the same type package producing incompatible nominal-ish types.
14. Committing generated JavaScript from TypeScript source when it is not meant to be source-controlled.
15. Source maps exposed with secrets or private source paths in production.

## 2. Compiler-error suppression

1. `// @ts-ignore` in production code.
2. `// @ts-nocheck` in handwritten files.
3. File-wide `// @ts-expect-error` style suppression.
4. `@ts-expect-error` without a specific reason.
5. `@ts-expect-error` where the expected error is not part of a type test.
6. Leaving stale `@ts-expect-error` after the underlying issue is fixed.
7. Suppressing one TypeScript error while accidentally suppressing unrelated errors on the same line.
8. Suppressing errors instead of fixing wrong declarations.
9. Suppressing errors from third-party libraries without an issue link, wrapper, or migration plan.
10. Suppressing generated-code errors in source directories without making the generated boundary explicit.
11. Adding `// eslint-disable` to bypass TypeScript safety rules.
12. Adding `/* eslint-disable */` to a whole file.
13. Disabling `@typescript-eslint/no-explicit-any`.
14. Disabling `@typescript-eslint/no-unsafe-*`.
15. Disabling `@typescript-eslint/no-floating-promises`.
16. Disabling `@typescript-eslint/ban-ts-comment`.
17. Disabling `strict-boolean-expressions` because truthiness bugs are inconvenient.
18. Disabling lint rules in CI but keeping them locally.
19. Downgrading type-safety rules from `error` to `warn` to merge a PR.
20. Deleting or weakening lint config to appease AI-generated code.
21. Adding blanket ignore patterns for `src/**`.
22. Ignoring all errors in a directory where humans still write code.
23. Creating suppressions that allow new code to add more errors.
24. Treating “legacy” as a permanent excuse for new violations.

## 2. `any` pollution

1. `let x: any`.
2. Function parameters typed as `any`.
3. Function return values typed as `any`.
4. `Promise<any>`.
5. `Array<any>` / `any[]`.
6. `Record<string, any>`.
7. `Map<string, any>`.
8. `Set<any>`.
9. `unknown as any`.
10. `as any`.
11. `as any as T`.
12. `JSON.parse(...) as any`.
13. API responses typed as `any`.
14. Event payloads typed as `any`.
15. Redux/Zustand/React state typed as `any`.
16. Express/Fastify/Koa request bodies typed as `any`.
17. Database rows typed as `any`.
18. Config/env objects typed as `any`.
19. Test mocks typed as `any` when `Partial<T>`, a builder, or a narrow mock type would work.
20. Public package APIs exposing `any`.
21. Generic defaults like `<T = any>`.
22. Utility types that leak `any` through the whole codebase.
23. “Temporary” `any` with no issue, owner, or removal path.
24. `any` introduced by AI because it could not infer the real type.

## 20. Bad React/JSX TypeScript

1. Props typed as `any`.
2. `children: any`.
3. Event handlers typed as `any`.
4. `useState<any>`.
5. `useRef<any>`.
6. Context values typed as `any`.
7. Reducer actions typed as `{ type: string; payload?: any }`.
8. Component props that mirror backend DTOs directly when UI state differs.
9. Non-null assertions on refs everywhere.
10. Async `useEffect` bodies without cleanup/error handling.
11. Ignoring race conditions in effects.
12. Storing derived state with inconsistent types.
13. `dangerouslySetInnerHTML` with untrusted input.
14. Trusting client-side TypeScript for authorization.
15. Exposing server-only env vars/types/data to the client.
16. Next.js server/client boundary confusion.
17. Treating route params/search params as typed without parsing.

## 21. Bad Node/API TypeScript

1. `req.body as SomeType` without validation.
2. `req.params.id as string` without checking.
3. `process.env as Env`.
4. Hardcoded secrets.
5. Secrets in client bundles.
6. Logging authorization headers, cookies, tokens, or PII.
7. Raw SQL string interpolation.
8. NoSQL query objects built directly from user input.
9. Shell commands built from user input.
10. File paths built from user input without normalization/allow-listing.
11. `eval`, `new Function`, or dynamic code execution on untrusted input.
12. JWT decoding without verification.
13. Authorization checks only in frontend code.
14. CORS `*` with credentials.
15. Rate limits omitted from sensitive endpoints.
16. Server-side validation omitted because “the form already validates.”
17. Accepting webhook payloads without signature verification.
18. Treating TypeScript interfaces as API security.

## 22. Database/ORM type mistakes

1. Assuming ORM-generated types validate user input.
2. Passing request bodies directly into `create`/`update`.
3. Trusting client-supplied IDs for ownership.
4. Selecting too many columns and returning secrets by accident.
5. Returning DB entities directly as API responses.
6. Using nullable DB columns as non-null app fields.
7. Ignoring transaction boundaries.
8. Not modeling unique constraint errors.
9. Not modeling not-found separately from permission-denied.
10. Raw queries with interpolated values.
11. Migrations that change schema without updating types/codegen.
12. Generated DB types not reproducible in CI.
13. Treating JSON columns as typed without validation.
14. Using `as` to force old code to fit new schema.

## 26. Generated code and codegen sloppiness

1. Manually editing generated types.
2. Generated files with no source schema.
3. Generated files that cannot be reproduced in CI.
4. Codegen output committed without the generator version.
5. API client types generated from stale schemas.
6. Backend/frontend schemas drifting.
7. Generated `any` everywhere.
8. Generated clients that do not validate runtime responses.
9. AI-generated schemas copied into multiple files.
10. Duplicated Zod/OpenAPI/Prisma/GraphQL schemas with no single source of truth.
11. “Just update the type” without updating runtime behavior.

## 27. Dependency and supply-chain recklessness

1. Installing random packages because AI suggested them.
2. Adding a dependency to avoid writing five lines of typed code.
3. Adding abandoned libraries with no types or bad types.
4. Using packages with `@types` that do not match runtime versions.
5. Ignoring critical security advisories.
6. Committing without a lockfile in app repos.
7. Allowing multiple incompatible versions of important type packages.
8. Using `declare module` instead of choosing a maintained package or writing a narrow adapter.
9. Importing deep private package paths with unstable types.
10. Using transitive dependencies directly.
11. Trusting package types more than runtime docs.
12. Copy/pasting code from README examples without adapting error handling and validation.

## 29. Giant, unmaintainable files

1. 700-line components.
2. 1,000-line service files.
3. Files with unrelated responsibilities.
4. One file containing types, validation, database, UI, and business logic.
5. Deeply nested conditionals generated by AI.
6. Copy-pasted functions with tiny variations.
7. Multiple sources of truth for the same type.
8. Repeated inline object types instead of named domain concepts.
9. Repeated validation logic instead of shared schemas.
10. “Utils” files that become junk drawers.
11. Circular imports caused by poor decomposition.
12. No clear ownership boundary between modules.
13. No meaningful naming.
14. Code that only the original prompt can explain.

## 3. Letting `any` leak through operations

1. Assigning an `any` value to a typed variable.
2. Calling an `any` value as a function.
3. Accessing properties on an `any` value.
4. Returning `any` from a function.
5. Passing `any` as an argument to a typed function.
6. Spreading `any[]` into function calls.
7. Putting `any` inside generic positions like `Set<any>` where `Set<string>` is expected.
8. Optional chaining on `any` and pretending it is now safe.
9. Using `any` from external libraries without quarantining it at the boundary.
10. Allowing `any` in reducers, middleware, RPC routers, API clients, or shared helpers.
11. Letting `any` cross module boundaries.

## 3. `tsconfig` sabotage

1. Turning off `"strict": true` in a new TypeScript project.
2. Turning off `"noImplicitAny"`.
3. Turning off `"strictNullChecks"`.
4. Turning off `"strictFunctionTypes"`.
5. Turning off `"strictPropertyInitialization"`.
6. Turning off `"useUnknownInCatchVariables"`.
7. Turning off `"strictBindCallApply"`.
8. Turning off `"noImplicitThis"`.
9. Turning off `"noImplicitReturns"` in app logic.
10. Turning off `"noFallthroughCasesInSwitch"`.
11. Turning off `"noImplicitOverride"` in class-heavy code.
12. Turning on `"noCheck"` for anything except explicitly generated/transitional tooling.
13. Using `"allowUnreachableCode": true` to avoid cleaning dead logic.
14. Using `"allowUnusedLabels": true` to hide likely syntax mistakes.
15. Omitting `"noUncheckedIndexedAccess"` in code that indexes arbitrary objects/arrays and then acting as if indexed values are always present.
16. Omitting `"exactOptionalPropertyTypes"` while relying on the difference between “missing” and `undefined`.
17. Using one `tsconfig` for incompatible runtime environments: browser, Node, worker, tests, build scripts.
18. Setting `lib` so broad that invalid runtime globals appear valid.
19. Setting `types` so broad that test globals leak into production source.
20. Setting `paths` aliases without making the runtime/bundler understand the same aliases.
21. Using stale `moduleResolution` such as `classic` or old Node modes in modern projects.
22. Mixing `module` and `moduleResolution` in ways that do not reflect the runtime.
23. Using bundler settings for a library that will be consumed by non-bundler users.
24. Publishing declarations generated from a different config than the code was checked under.
25. Making `tsc` optional in CI.
26. Relying on Babel/SWC/esbuild transpilation without a separate `tsc --noEmit` check.
27. Treating “the app runs” as a replacement for type checking.
28. Changing compiler options in a feature PR to avoid fixing the feature.
29. Hiding config changes inside unrelated commits.
30. Maintaining separate local and CI TypeScript configs that disagree on strictness.
31. Failing to pin/coordinate TypeScript versions across a monorepo.

## 30. Optional chaining as duct tape

1. `user?.profile?.email` where email is required after login.
2. `order?.items?.[0]?.price` in payment logic.
3. `permissions?.includes("admin")` in auth logic without default-deny clarity.
4. `config?.apiKey` where startup should fail if missing.
5. `data?.foo?.bar` repeated everywhere instead of validating `data`.
6. Optional chaining in reducers to hide invalid state.
7. Optional chaining in tests to avoid crashes.
8. Optional chaining added by AI to eliminate null errors.

## 31. Bad naming and misleading types

1. `data`, `payload`, `obj`, `thing`, `stuff` for domain values.
2. `IUser` meaning API user in one place and DB user in another.
3. `User` reused for public profile, auth session, database row, and admin view.
4. `Config` for both raw env and validated config.
5. `FooType`, `FooInterface`, `FooModel` with no semantic distinction.
6. Types named after implementation rather than domain.
7. `Response` shadowing DOM/Fetch response concepts.
8. `Error` types that are not errors.
9. `Nullable<T>` used everywhere instead of modeling why null occurs.
10. Comments contradicting types.
11. Types that describe what the code wishes were true.

## 32. Mutability hazards

1. Mutating function inputs without naming/documenting it.
2. Mutating objects from React state.
3. Mutating cached/shared objects.
4. Exporting mutable singletons.
5. Casting `readonly` away.
6. Returning mutable internal arrays/objects.
7. Sorting arrays in place when callers expect original order.
8. Reusing `Date` objects and mutating them.
9. Global mutable test state.
10. Mutable default config shared across requests.
11. Shared request-scoped state in module globals.
12. Hidden mutation inside validators/parsers.
13. Mutating objects after validation so their type no longer reflects reality.

## 33. Bad date/time/number handling

1. Money as float without rounding/minor-unit policy.
2. `Date.parse` on ambiguous strings.
3. Timezone assumptions hidden in code.
4. Comparing date strings with mixed formats.
5. Storing local time when UTC is required.
6. Treating timestamps as seconds in one place and milliseconds in another.
7. Using `number` for branded quantities that must not mix.
8. Ignoring `NaN`.
9. Ignoring `Infinity`.
10. Parsing numbers with `Number(value)` and accepting `"" -> 0`.
11. Type aliasing `type Dollars = number` but mixing cents/dollars.
12. AI-generated date logic without tests around DST/timezones.

## 34. Bad forms and user input

1. Form values typed as final domain objects before parsing.
2. Treating all form inputs as strings and casting later.
3. Client-only validation.
4. Missing server validation.
5. Not trimming/canonicalizing when domain requires it.
6. Trimming when domain must preserve input exactly.
7. Treating empty string, null, and undefined interchangeably.
8. Ignoring multi-select/file edge cases.
9. Type assertions on `FormData.get`.
10. No validation for file type, size, and content.
11. Trusting browser-provided MIME types.
12. Generated forms that submit fields not allowed by the backend.

## 35. Bad configuration/env handling

1. Reading `process.env.X!` throughout the app.
2. No startup validation.
3. No distinction between raw env and validated config.
4. Config values parsed at every use site.
5. Boolean env parsed by truthiness.
6. Number env parsed without range checks.
7. Defaults that hide missing production secrets.
8. Client-exposed env vars with secrets.
9. Config drift between local, CI, and production.
10. `as Config` on env/config files.
11. Feature flags typed as plain strings.
12. Unused config silently ignored.
13. Required config optional in types.

## 36. Bad comments and TODOs

1. `TODO fix types`.
2. `// @ts-ignore TODO`.
3. `// temporary` with no date/owner/ticket.
4. Comments explaining what the type system should express.
5. Comments claiming validation happened elsewhere when it did not.
6. Comments that rationalize unsafe casts without proof.
7. Dead comments from AI output.
8. Large blocks of generated explanation instead of clear code.
9. Misleading JSDoc on public APIs.
10. TODOs in security-sensitive paths.
11. TODOs that survive review.

## 37. Bad CI/repo policy

1. No `tsc --noEmit` in CI.
2. No lint in CI.
3. No type-aware linting for TypeScript.
4. No tests for changed code.
5. CI that runs on different tsconfig than local.
6. CI that ignores generated type drift.
7. CI that ignores schema/codegen drift.
8. Warnings allowed forever.
9. TypeScript version differs across packages without reason.
10. Multiple tsconfigs with inconsistent strictness.
11. Monorepo packages importing source across boundaries without declared dependencies.
12. Package builds that pass independently but fail as consumers.
13. No code owner for shared types/schemas.
14. No dependency update strategy.
15. No lockfile integrity.

## 38. Bad monorepo TypeScript

1. One root tsconfig that accidentally includes everything.
2. Packages importing each other via relative `../../` paths.
3. Runtime dependency not listed in `package.json`.
4. Type dependency only working because of hoisting.
5. Package boundary violations.
6. Circular workspace dependencies.
7. Different packages using incompatible `strict` settings.
8. Generated types committed in one package but regenerated from another.
9. Path aliases that do not match package exports.
10. Package exports missing `types`.
11. Source imports from packages that should consume built declarations.
12. No project references or equivalent build graph in large repos.
13. Typecheck too slow, so people stop running it.

## 39. Bad “fixes” to TypeScript errors

1. Replaces a real type with `any`.
2. Replaces a real type with `unknown` and never narrows it.
3. Adds `!`.
4. Adds `as SomeType`.
5. Adds `as never`.
6. Adds optional properties everywhere.
7. Makes required fields optional.
8. Makes union discriminants plain `string`.
9. Deletes a branch.
10. Deletes an assertion.
11. Deletes a test.
12. Changes tsconfig.
13. Changes ESLint config.
14. Adds `@ts-ignore`.
15. Wraps code in `try/catch` and swallows the error.
16. Adds `|| ""`, `|| []`, or `|| {}` without domain justification.
17. Converts compile-time failure into runtime fallback.
18. Moves the error somewhere else.
19. Creates a duplicate type instead of importing the canonical one.

## 4. Misusing `unknown`

1. `const x = value as unknown as RealType` without proof.
2. `unknown` immediately cast to the desired type.
3. `unknown` stored in app state and used everywhere.
4. `unknown` returned from public APIs when the function could validate and return a real type.
5. `unknown` in error handlers with no narrowing.
6. `unknown` used to hide incomplete modeling.
7. `unknown` used as “safe any” while still skipping validation.
8. `Record<string, unknown>` used for real domain objects that deserve a schema.
9. Treating `unknown` as validated because it came from “our backend.”
10. Treating generated OpenAPI/GraphQL types as a runtime guarantee.

## 4. Runtime-boundary dishonesty

1. Believing interfaces validate runtime data.
2. Believing `as SomeType` changes runtime behavior.
3. Believing `!` checks for non-null at runtime.
4. Accepting untrusted JSON as a domain object.
5. Accepting request bodies as DTOs without validation.
6. Accepting response bodies from third-party APIs without validation.
7. Accepting database rows as trusted when the query shape can drift.
8. Accepting `process.env.X!` without startup validation.
9. Accepting URL params as typed values without parsing.
10. Accepting query params as booleans/numbers without parsing.
11. Accepting localStorage/sessionStorage values as typed objects without parsing.
12. Accepting feature flags as union literals without validating unknown flag values.
13. Accepting cookies/JWT claims/session data as trusted because they have a TypeScript type.
14. Accepting WebSocket/event/queue messages without schema validation.
15. Accepting `postMessage` payloads without origin and payload validation.
16. Accepting form data as typed objects without validation.
17. Accepting file uploads as typed metadata without validation.
18. Accepting CLI args as typed config without parsing.
19. Accepting generated client types as proof that the server still obeys the contract.
20. Using `JSON.parse(...) as T` directly.
21. Using `response.json() as Promise<T>` directly.
22. Using `z.infer<T>` or similar inferred types without actually running the schema at runtime.
23. Validating only in tests but not at the boundary.
24. Validating on the client but not on the server.
25. Validating only “happy path” fields and ignoring unknown/malicious fields.
26. Converting invalid data to defaults silently.
27. Swallowing validation errors and continuing with partial objects.
28. Logging validation errors without failing the operation.
29. Treating timestamps as `Date` when JSON gave you strings.
30. Treating money/decimal values as safe `number` values without precision rules.
31. Treating IDs from external systems as branded/internal IDs without conversion.
32. Treating string enums from APIs as exhaustive forever.
33. Treating nullable API fields as non-null because “the backend always sends it.”
34. Trusting generated OpenAPI/GraphQL types without checking the actual transport layer.
35. Trusting mock data more than production data.

## 40. Bad review behavior

1. “Looks fine” review on a large AI-generated diff.
2. Reviewing only the UI behavior.
3. Not checking type changes.
4. Not checking generated files.
5. Not checking dependency additions.
6. Not checking security boundaries.
7. Not checking runtime validation.
8. Not checking error paths.
9. Not checking whether removed code was intentional.
10. Not checking whether types match runtime behavior.
11. Approving suppressions without explanation.
12. Approving `any` because “we can fix it later.”
13. Approving code the author cannot explain.
14. Approving code that only passes because strictness was weakened.

## 5. Lying with type assertions

1. `(value as User)` for unvalidated API data.
2. `(req.body as CreateUserInput)` without schema validation.
3. `(JSON.parse(raw) as Config)` without validation.
4. `as Foo` on object literals to dodge excess-property checks.
5. `as Foo` on return objects instead of annotating the return type.
6. `as unknown as Foo` without a documented invariant.
7. `as never` to bypass exhaustiveness errors.
8. `as const` used to freeze a wrong or over-specific structure instead of modeling it.
9. Casting away `readonly`.
10. Casting nullable values to non-nullable values.
11. Casting partial mocks to full services in production code.
12. Casting one domain ID to another, such as `userId as OrgId`.
13. Casting external library results instead of writing a narrow adapter.
14. Casting because “the tests pass.”
15. Letting AI add casts in bulk.

## 5. Null and undefined abuse

1. Using `!` as a reflex.
2. Using `foo!.bar!.baz!`.
3. Using `array[0]!` without proving length.
4. Using `map.get(key)!` without handling missing keys.
5. Using `document.getElementById(id)!` without handling missing DOM nodes.
6. Using `process.env.SECRET!` without config validation.
7. Using `find(...)!` without a fallback or explicit failure.
8. Using `filter(Boolean)` and assuming TypeScript understands the intended narrowing.
9. Using truthiness checks for values where `0`, `""`, or `false` are valid.
10. Using `||` defaults when `0`, `""`, or `false` should be preserved.
11. Using `&&` chains where optional chaining would preserve valid falsy values.
12. Using optional properties for values that are actually required after construction.
13. Making every field optional to silence constructor/initialization errors.
14. Using `Partial<T>` as long-lived state instead of modeling loading/ready/error states.
15. Using `Required<T>` to pretend optional runtime data is present.
16. Using `NonNullable<T>` to pretend a value was checked.
17. Returning `undefined` as an implicit error channel.
18. Mixing `null` and `undefined` without a convention.
19. Storing both “missing” and `undefined` in optional properties when presence matters.
20. Using `exactOptionalPropertyTypes: false` while relying on `in` checks.
21. Treating absent values, null values, empty values, and default values as the same thing.
22. Failing to distinguish “not loaded,” “not found,” and “loaded but empty.”
23. Using empty objects as placeholders for not-yet-loaded domain values.
24. Using empty arrays as placeholders when “not fetched” and “fetched empty” are different.
25. Swallowing impossible nulls instead of throwing loudly at invariant boundaries.

## 6. Abusing non-null assertions

1. `user!.id`.
2. `ref.current!` without a lifecycle guarantee.
3. `array.find(...)!.value`.
4. `map.get(key)!`.
5. `process.env.SECRET!`.
6. `document.querySelector(...)!`.
7. `params.id!`.
8. `props.foo!`.
9. `config.whatever!`.
10. `promiseResult.data!`.
11. Class fields declared with `!` to avoid initialization.
12. Non-null assertions repeated across a file instead of one validated boundary.
13. `!` added by AI to fix “possibly undefined” errors.
14. `!` in code paths reachable from user input.
15. `!` in async code where timing can change the invariant.

## 6. Async and Promise misconduct

1. Floating promises.
2. Calling async functions without `await`, `return`, `.catch`, or an intentional `void`.
3. Using `void somePromise()` to silence lint without a real error path.
4. Using async callbacks in places that ignore promises.
5. Using `Array.prototype.forEach(async ...)`.
6. Not awaiting `Promise.all`.
7. Starting concurrent work and never joining/cancelling it.
8. Swallowing promise rejections.
9. Catching and logging errors but continuing as if the operation succeeded.
10. Returning a promise from a function typed as `void` where callers cannot observe failure.
11. Forgetting `return` inside `.then`.
12. Mixing `.then` chains and `async/await` until control flow is unclear.
13. Using `async` functions with no `await` and no reason.
14. Creating race conditions by reading stale state after `await`.
15. Ignoring `AbortSignal` in fetches or long-running operations.
16. Not cleaning up timers, subscriptions, sockets, observers, or streams.
17. Forgetting `finally` for cleanup paths.
18. Using unbounded `Promise.all` over large inputs.
19. Using `Promise.all` when partial failure semantics are required.
20. Using `Promise.allSettled` and ignoring rejected results.
21. Retrying without backoff, jitter, idempotency, or cancellation.
22. Retrying non-idempotent operations blindly.
23. Throwing strings or plain objects.
24. Rejecting promises with non-`Error` values.
25. Catching `unknown` and assuming it is `Error`.
26. Losing stack traces by wrapping errors poorly.
27. Hiding original causes instead of using `cause`.
28. Returning `null` on errors without preserving error information.
29. Treating timeouts as impossible.
30. Treating network calls as infallible because the type says `Promise<T>`.

## 7. Bad data modeling

1. Modeling domain states as a bag of optional booleans.
2. Modeling mutually exclusive states without a discriminated union.
3. Using `type: string` instead of literal discriminants.
4. Using `status: string` where only known statuses are valid.
5. Using boolean flag soup: `isLoading`, `isLoaded`, `hasError`, `isError`, `isEmpty`, etc., where impossible combinations can exist.
6. Not modeling error states.
7. Not modeling loading states.
8. Not modeling empty states.
9. Not modeling permission/auth states.
10. Not modeling partial data explicitly.
11. Using `Partial<DomainModel>` as an all-purpose loading/update/input type.
12. Reusing database types directly as API types.
13. Reusing API response types directly as UI state.
14. Reusing form types directly as domain types.
15. Reusing domain types directly as persistence types.
16. Reusing generated types everywhere instead of creating boundary adapters.
17. Making every property optional because object construction is inconvenient.
18. Making properties nullable everywhere because one layer is nullable.
19. Using `string` for IDs from different domains.
20. Mixing user IDs, org IDs, order IDs, and external IDs as plain strings.
21. Using unbranded strings for opaque tokens that should not be mixed.
22. Using `number` for money without a precision policy.
23. Using `number` for timestamps when semantics require `Date`, ISO string, or epoch type.
24. Using mutable domain objects across layers.
25. Using DTOs with methods.
26. Using classes as data bags without invariants.
27. Using inheritance for data variants where unions are clearer.
28. Using enums with duplicate values.
29. Mixing string and numeric enum members.
30. Comparing enum values to raw strings/numbers.
31. Using enums where literal unions would avoid runtime baggage.
32. Using literal unions where runtime iteration/metadata is required but not supplied.
33. Failing to make switches exhaustive.
34. Adding a default branch that hides missing union cases.
35. Throwing generic “should never happen” instead of enforcing `never` exhaustiveness.
36. Not updating all consumers when adding a union member.
37. Encoding business rules only in comments.

## 7. Disabling strictness

1. `strict`.
2. `noImplicitAny`.
3. `strictNullChecks`.
4. `strictFunctionTypes`.
5. `strictBindCallApply`.
6. `strictPropertyInitialization`.
7. `noImplicitThis`.
8. `useUnknownInCatchVariables`.
9. `noUncheckedIndexedAccess`.
10. `exactOptionalPropertyTypes`.
11. `noImplicitReturns`.
12. `noFallthroughCasesInSwitch`.
13. `noImplicitOverride`.
14. `noPropertyAccessFromIndexSignature`.
15. `forceConsistentCasingInFileNames`.
16. `noUncheckedSideEffectImports`, where supported and applicable.
17. `allowUnreachableCode: true`.
18. `allowUnusedLabels: true`.
19. `suppressImplicitAnyIndexErrors`.
20. `suppressExcessPropertyErrors`.

## 8. Function and API typing mistakes

1. Public exported functions without clear parameter and return types when inference is not obvious.
2. Public APIs that expose internal implementation types.
3. Callbacks typed as `(...args: any[]) => any`.
4. Event handlers typed as `any`.
5. Reducers/actions typed as `any`.
6. Express/Koa/Fastify handlers typed as generic `Request`/`Response` without typed params/body/query where needed.
7. React component props typed as `any`.
8. React state initialized with `null as any`.
9. Custom hooks returning poorly typed arrays instead of named objects or const tuples.
10. Returning unions that force consumers to guess instead of using discriminants.
11. Returning `T | null | undefined | false | ""` as an informal protocol.
12. Throwing for expected domain outcomes instead of returning typed results.
13. Returning `Promise<T | undefined>` without documenting/typing why undefined happens.
14. Overloading when a union would be clearer.
15. Putting general overloads before specific overloads.
16. Writing overloads that differ only by callback arity.
17. Writing overloads that differ only in one argument type where a union works.
18. Writing overload signatures that do not match implementation reality.
19. Using optional callback parameters when the callback is always called with the argument.
20. Using `any` return type for ignored callbacks instead of `void`.
21. Using `void` in places where the return value is actually observed.
22. Accepting mutable arrays when the function only reads.
23. Mutating input parameters without making mutation explicit.
24. Mutating objects received from callers.
25. Mutating function arguments typed as readonly via casts.
26. Using rest parameters without typing them precisely.
27. Using tuple positions without naming or documenting semantics.
28. Using positional booleans instead of options objects.
29. Using options objects where required fields are not enforced.
30. Allowing invalid combinations of options.
31. Not separating “create input,” “update input,” and “stored entity.”
32. Not typing thrown/rejected domain errors in a discoverable way.

## 8. Misusing `skipLibCheck`

1. Using `skipLibCheck` to ignore broken hand-written `.d.ts` files in your own repo.
2. Using it to avoid resolving duplicate incompatible versions of the same types.
3. Using it to mask dependency tree corruption.
4. Using it as the only fix after a TypeScript upgrade breaks declarations.
5. Using it without knowing whether bad declarations affect your app.
6. Using it to ignore generated types you publish to others.
7. Treating `skipLibCheck` as a substitute for dependency hygiene.

## 9. Object, index, and collection hazards

1. Indexing objects with arbitrary strings and assuming presence.
2. Indexing arrays and assuming the element exists.
3. Using `{ [key: string]: T }` when only known keys are valid.
4. Using `Record<string, T>` where keys are actually a finite union.
5. Using `Record<K, T>` but not ensuring every `K` is initialized.
6. Ignoring `undefined` from `Map.get`.
7. Ignoring `undefined` from `Array.find`.
8. Ignoring empty arrays before destructuring.
9. Destructuring optional nested fields without defaults/checks.
10. Using object spread on class instances and expecting methods/prototypes to survive.
11. Using JSON stringify/parse as cloning.
12. Using JSON stringify/parse on `Date`, `Map`, `Set`, `BigInt`, `undefined`, functions, cyclic objects, or custom classes.
13. Using `delete` on arrays.
14. Using dynamic `delete` on objects without considering shape/perf/type impact.
15. Mutating arrays while iterating.
16. Using `for...in` over arrays.
17. Sorting arrays without a comparator when numeric/domain ordering matters.
18. Mutating arrays with `.sort()` where immutable behavior is expected.
19. Relying on object key order for semantic behavior.
20. Using objects as dictionaries when `Map` is required.
21. Using `Map` when JSON serialization is required and forgetting conversion.
22. Using plain objects with user-controlled keys without guarding against prototype pollution.
23. Using `Object.keys(obj)` and casting to `(keyof T)[]` without proving keys.
24. Using `Object.entries` and losing key/value relationships.
25. Reconstructing typed objects from entries with a blind cast.
26. Merging configs without preserving exact optional semantics.

## 9. Skipping runtime validation at trust boundaries

1. HTTP request bodies.
2. Query strings.
3. Route params.
4. Headers.
5. Cookies.
6. JWT/session claims.
7. Webhooks.
8. Third-party APIs.
9. Your own API if independently deployed/versioned.
10. `JSON.parse`.
11. `localStorage` / `sessionStorage`.
12. `postMessage`.
13. WebSocket messages.
14. Message queues.
15. IPC.
16. CLI args.
17. Environment variables.
18. Config files.
19. Feature flags.
20. Database rows after schema drift.
21. Cache values.
22. URL params.
23. Form data.
24. Uploaded files.
25. LLM/tool outputs.
26. Generated code or generated JSON.
27. `fetch(...).then(r => r.json() as User)`.
28. `const env = process.env as Env`.
29. `const body: CreateUserInput = req.body`.
30. `JSON.parse(raw) as Settings`.
31. Trusting client-side validation only.
32. Assuming TypeScript protects API boundaries.
33. Assuming OpenAPI/GraphQL generated types validate runtime data.
34. Assuming Prisma/ORM types validate external input.
35. Trusting `z.infer<T>` without actually calling `parse`/`safeParse`.
36. Validating only happy-path fields.
37. Ignoring unknown extra fields when they matter for security.
38. Converting invalid data into partial objects and continuing.

## AI & Vibe-Coding

1. Merging code you cannot explain.
2. Merging code the AI cannot explain coherently.
3. Merging code without reading the diff.
4. Asking AI to “continue” repeatedly until the app works.
5. Letting AI rewrite unrelated files.
6. Letting AI change architecture to fix a local bug.
7. Letting AI add `any`, casts, suppressions, or optional chaining as a “fix.”
8. Letting AI duplicate existing helpers/types instead of importing them.
9. Letting AI invent APIs, config flags, package functions, or framework behavior.
10. Letting AI add dependencies without review.
11. Letting AI generate security-sensitive code without human threat modeling.
12. Letting AI write auth/payment/crypto code unsupervised.
13. Letting AI write migrations without review.
14. Letting AI update schemas without runtime compatibility checks.
15. Letting AI create giant files because “it works.”
16. Letting AI leave TODOs as design.
17. Letting AI produce over-general abstractions.
18. Letting AI produce under-modeled blobs.
19. Letting AI generate tests that only pass the generated code.
20. Letting AI “fix” failing tests by weakening assertions.
21. Letting AI remove tests.
22. Letting AI change tsconfig/eslint to pass.
23. Letting AI ignore TypeScript errors.
24. Letting AI flatten all errors into `try/catch`.
25. Letting AI create fake fallback behavior instead of handling the real case.
26. Letting AI use client-side state for server-side truth.
27. Letting AI hide data-shape uncertainty under `as`.
28. Letting AI turn every nullable thing into optional chaining.
29. Letting AI turn every hard edge into `any`.
30. Treating “compiled” as “correct.”
31. Treating “the UI rendered” as “the system is safe.”
32. `any` appears in normal app code.
33. `@ts-ignore` appears.
34. `as unknown as` appears.
35. `!` appears repeatedly.
36. `TODO fix types` appears.
37. `JSON.parse(...) as X`.
38. `fetchJson<T>()` with no schema.
39. `process.env.X!`.
40. `Record<string, any>`.
41. Giant components/services.
42. Duplicate interfaces with similar names.
43. Types defined far away from the runtime parser.
44. Optional properties everywhere.
45. Discriminants typed as `string`.
46. Catch blocks that swallow.
47. Tests removed or weakened.
48. New dependencies for trivial tasks.
49. Server/client boundary confusion.
50. Auth logic in UI only.
51. Generated tests that only test happy path.
52. Lint/tsconfig loosened in the same PR.
53. Code compiles but nobody can explain the invariants.
54. Accepting AI-generated TypeScript without reading the diff.
55. Accepting AI-generated TypeScript because “it compiles.”
56. Accepting AI-generated TypeScript because “the page loads.”
57. Asking an AI to “fix the TypeScript error” and accepting a type escape.
58. Letting an AI add `any`.
59. Letting an AI add `as any`.
60. Letting an AI add `@ts-ignore`.
61. Letting an AI add `@ts-nocheck`.
62. Letting an AI weaken `tsconfig`.
63. Letting an AI weaken ESLint config.
64. Letting an AI delete tests.
65. Letting an AI update snapshots without review.
66. Letting an AI change package versions casually.
67. Letting an AI introduce new dependencies without review.
68. Letting an AI alter generated files manually.
69. Letting an AI edit migrations without human review.
70. Letting an AI touch auth, payments, permissions, data deletion, secrets, or infrastructure without tight review.
71. Letting an AI invent API fields.
72. Letting an AI invent types that do not match runtime data.
73. Letting an AI create duplicate domain types instead of finding the existing source of truth.
74. Letting an AI create parallel utility functions instead of reusing existing ones.
75. Letting an AI introduce dead comments explaining obvious code.
76. Letting an AI create giant files or giant functions because it was not given boundaries.
77. Letting an AI “simplify” discriminated unions into loose objects.
78. Letting an AI turn precise types into `string`, `number`, or `object`.
79. Letting an AI replace a failing validator with a cast.
80. Letting an AI replace a failing test with a weaker assertion.
81. Letting an AI remove error handling to quiet tests.
82. Letting an AI swallow errors to make flows pass.
83. Letting an AI add broad try/catch blocks that hide real failures.
84. Letting an AI add comments like “production-ready,” “robust,” or “type-safe” without evidence.
85. Letting an AI create “temporary” code without an issue, owner, or removal plan.
86. Letting an AI run broad repo edits without constrained scope.
87. Letting an AI auto-commit without human review.
88. Letting an AI claim tests passed when the tool was not run.
89. Letting an AI claim types passed when `tsc` was not run.
90. Letting an AI work without a fast validation harness.
91. Letting an AI work in a repo where safety rules are optional.
92. Letting an AI choose architecture from local context alone.
93. Letting vibe-coded code bypass the same review bar as human code.

## Frontend/browser

1. Trusting `window` globals without declaration and runtime checks.
2. Adding `declare global` for a script that may not load.
3. Trusting `dataset` values as typed values.
4. Trusting DOM input values as numbers/booleans.
5. Trusting browser storage as fresh schema.
6. Trusting feature detection via casts instead of runtime checks.
7. Casting browser APIs across incompatible environments.
8. Using Node types in browser bundles accidentally.
9. Using DOM types in server-only code accidentally.
10. `any`, `as any`, or `unknown as T`.
11. `@ts-ignore` or `@ts-nocheck`.
12. TypeScript/lint config weakened in a feature PR.
13. `strict` disabled for new code.
14. `strictNullChecks` disabled.
15. Runtime input cast instead of validated.
16. Floating promise.
17. Non-null assertion on external or optional data.
18. Public API returning/accepting `any`.
19. Tests removed or weakened.
20. AI-generated code accepted without typecheck/lint/test evidence.
21. Security-sensitive code with casts, unchecked input, or swallowed errors.
22. `unknown` at real boundaries.
23. `@ts-expect-error` in type tests.
24. Narrow, documented suppressions during a controlled migration.
25. Generated code in a clearly marked generated directory.
26. `skipLibCheck` for dependency declaration performance, when your own source is still checked.
27. Very small, reviewed interop wrappers around bad third-party types.
28. Type assertions after a runtime check that TypeScript cannot express.
29. A temporary migration bridge with owner, issue, and removal deadline.

## Minimum enforcement baseline

1. `strict: true`.
2. `noImplicitAny: true`.
3. `strictNullChecks: true`.
4. `noUncheckedIndexedAccess: true`.
5. `exactOptionalPropertyTypes: true`.
6. `noImplicitReturns: true`.
7. `noFallthroughCasesInSwitch: true`.
8. `noImplicitOverride: true`.
9. `noPropertyAccessFromIndexSignature: true`.
10. `forceConsistentCasingInFileNames: true`.
11. `noEmitOnError: true` for build/deploy paths.
12. `tsc --noEmit` in CI.
13. `@typescript-eslint/recommended-type-checked` or stricter.
14. `no-explicit-any`.
15. `no-unsafe-assignment`.
16. `no-unsafe-member-access`.
17. `no-unsafe-call`.
18. `no-unsafe-argument`.
19. `no-unsafe-return`.
20. `ban-ts-comment`.
21. `no-floating-promises`.
22. `no-misused-promises`.
23. Runtime schemas at every trust boundary.
24. Human review of all AI-generated diffs.
25. Tests for success, failure, null/undefined, invalid input, and authz.

## Node/API servers

1. `req.body as SomeDto` without validation.
2. `req.query as SomeQuery` without parsing.
3. `req.params as SomeParams` without parsing.
4. Treating middleware-populated properties as present without typed augmentation and runtime checks.
5. Adding global request augmentation for one route-specific property.
6. Returning raw database objects as API responses.
7. Trusting ORM model types as API contract types.
8. Not typing service boundaries.
9. Not separating internal errors from public errors.
10. Catch-all error middleware that hides typed error semantics.
11. Not validating env at startup.
12. Not making config immutable after startup.
13. Using `process.env` all over the codebase instead of a validated config object.

## React / TSX

1. Props typed as `any`.
2. `children: any`.
3. Event handlers typed as `any`.
4. `useState<any>`.
5. `useRef<any>`.
6. `useRef(null) as any`.
7. Context default value cast to a full context type without provider checks.
8. Creating context with fake no-op defaults that hide missing providers.
9. Non-null asserting context instead of throwing a clear provider error.
10. Using array indexes as keys when order can change.
11. Typing component props as `React.FC<any>`.
12. Spreading unknown props into DOM elements.
13. Passing invalid DOM attributes via broad prop types.
14. Using `dangerouslySetInnerHTML` with untrusted strings.
15. Async `useEffect` directly instead of defining/calling an inner async function with cleanup.
16. Ignoring cleanup in effects.
17. Ignoring stale closures.
18. Ignoring dependency arrays by disabling lint.
19. Casting route params to required strings.
20. Casting loader/action data without validation.
21. Treating server component/client component boundaries casually.
22. Serializing non-serializable props across server/client boundaries.

## Security

1. Types that imply a user is authenticated before auth is checked.
2. `user as AuthenticatedUser`.
3. Client-provided `role`, `isAdmin`, `ownerId`, `price`, `credits`, or `plan`.
4. Trusting hidden form fields.
5. Trusting localStorage for permissions.
6. Trusting decoded JWT payload without verifying signature and claims.
7. Client-side feature gates as security.
8. Stringly typed permissions.
9. Permission checks duplicated inconsistently.
10. Missing default-deny behavior.
11. Authorization logic hidden in UI components.
12. Audit logs typed as optional and silently skipped.
13. Payment/subscription state controlled client-side.
14. “Admin mode” flags in frontend-only state.
15. Hardcoded credentials.
16. Secrets committed to repo.
17. Secrets logged.
18. Secrets sent to client.
19. `eval`.
20. `new Function`.
21. Unsafe `innerHTML`.
22. Unsafe `dangerouslySetInnerHTML`.
23. Raw SQL interpolation.
24. Shell command interpolation.
25. Path traversal risks.
26. SSRF-prone URL fetching.
27. Prototype pollution via untrusted objects.
28. Insecure random for tokens.
29. Weak crypto because AI suggested it.
30. Password handling without a vetted library.
31. JWTs decoded but not verified.
32. Missing CSRF protection where relevant.
33. Missing authz on server routes.
34. Trusting TypeScript types as validation/security.
35. Hardcoding secrets.
36. Committing `.env` files with real secrets.
37. Logging tokens, cookies, passwords, API keys, authorization headers, or PII.
38. Putting secrets in frontend TypeScript.
39. Trusting client-side checks for authorization.
40. Trusting TypeScript types for access control.
41. Building SQL strings with interpolation.
42. Building NoSQL queries from untrusted objects without validation.
43. Passing user input to dynamic `import`.
44. Passing user input to `child_process.exec`.
45. Constructing shell commands with string concatenation.
46. Constructing filesystem paths from user input without normalization and allowlists.
47. Using `innerHTML`/dangerously-set HTML with untrusted content.
48. Bypassing sanitizer types with casts.
49. Trusting HTML because it is typed as `SafeHtml`.
50. Building regexes from user input without escaping/limits.
51. Allowing prototype pollution via untrusted object merge.
52. Deserializing untrusted data into privileged objects.
53. Using weak crypto or homegrown crypto.
54. Ignoring dependency vulnerabilities.
55. Installing random packages to solve tiny problems.
56. Installing packages suggested by AI without checking reputation/source.
57. Ignoring lockfile changes.
58. Running package scripts from untrusted dependencies carelessly.
59. Ignoring SSRF risks in server-side fetches.
60. Using broad CORS because it fixed local dev.
61. Returning verbose internal errors to users.
62. Swallowing security-relevant errors instead of alerting/logging safely.
63. Not rate-limiting sensitive operations.
64. Not checking authentication and authorization separately.
65. Trusting JWT contents without verifying signature/issuer/audience/expiry.
66. Trusting webhook payloads without verifying signatures.
67. Trusting file MIME type from the client.
68. Trusting frontend-generated IDs for ownership.
69. Exposing source maps or stack traces with sensitive paths/data unintentionally.

## Testing

1. No tests for validation/parsing boundaries.
2. No tests for error paths.
3. No tests for missing/null/empty values.
4. No tests for authorization failures.
5. No tests for async failures.
6. No tests for schema evolution.
7. Snapshot-only tests for logic.
8. Tests that assert implementation details but not behavior.
9. Tests that pass because everything is mocked as `any`.
10. Tests that use `as any` to bypass the same invariant production code depends on.
11. Type errors hidden in test files.
12. Test tsconfig looser than production tsconfig without a reason.
13. AI-generated tests that merely confirm AI-generated behavior.
14. Updating snapshots without reviewing semantic changes.
15. Not running `tsc`, lint, unit tests, and relevant integration tests in CI.
16. Treating “the page loads” as enough.
17. Deep utility types with no type tests.
18. Public library types with no compile-time assertions.
19. Generic API helpers with no negative tests.
20. Assuming inferred types are what you intended.
21. Not testing that invalid types fail.
22. No tests for declaration output.
23. No tests for package consumers.
24. Breaking library consumers with “only type changes.”
25. Type tests that rely on `any`, making them meaningless.
26. No `tsc --noEmit` or equivalent type-check step in CI.
27. CI transpiles but does not type-check.
28. CI runs tests but not lint.
29. CI runs lint but treats type-safety violations as warnings.
30. CI ignores generated type errors.
31. CI uses a weaker `tsconfig` than local development.
32. CI skips packages in a monorepo.
33. CI only checks changed files when dependency changes can break downstream files.
34. No test proving runtime validators reject bad data.
35. No tests for null/undefined paths.
36. No tests for loading/empty/permission states.
37. No tests for serialization/deserialization.
38. No tests for schema migrations.
39. No tests for public API types.
40. No type tests for exported libraries.
41. No regression test for a fixed type bug.
42. Deleting tests to make AI-generated code pass.
43. Updating snapshots without reading them.
44. Mocking types so aggressively that production behavior is not tested.
45. Mocking network responses with impossible shapes.
46. Mocking environment variables as always present.
47. Mocking time/randomness/global state without cleanup.
48. Letting tests depend on order.
49. Letting tests pass because errors are swallowed.
50. Letting `console.error` noise hide real failures.
51. Not failing tests on unhandled promise rejections.
52. Not checking bundle/build output for runtime import failures.
53. Not checking package declarations before publishing.
54. Not testing against the minimum supported TypeScript version for a library.

## The master rule

1. **Using `any` to avoid understanding the data.**
2. **Using explicit `any` in application code.**
3. **Letting `any` leak from libraries or parsing.**
4. **Using `any[]` for collections.**
5. **Using `as any` to silence errors.**
6. **Using `as unknown as T` as a laundering pipeline.**
7. **Using `as T` to pretend runtime data has been validated.**
8. **Using assertions instead of fixing the source type.**
9. **Using unnecessary assertions that add noise.**
10. **Using non-null assertions, `!`, to make errors disappear.**
11. **Using optional chaining to hide missing invariants.**
12. **Using `!` after database lookups, route params, config, DOM queries, refs, cache reads, or API responses.**
13. **Using `// @ts-ignore`.**
14. **Using `// @ts-nocheck`.**
15. **Using file-wide or directory-wide type suppression.**
16. **Using `@ts-expect-error` without a specific explanation and issue link.**
17. **Leaving stale suppressions in code.**
18. **Using ESLint disable comments as a blanket escape hatch.**
19. **Disabling `@typescript-eslint/no-explicit-any`.**
20. **Disabling `@typescript-eslint/no-unsafe-*` rules because “the code works.”**
21. **Turning off strict mode in a real TypeScript codebase.**
22. **Turning off `noImplicitAny` to pass CI.**
23. **Turning off `strictNullChecks` to avoid handling missing values.**
24. **Turning off `noUncheckedIndexedAccess` while indexing untrusted objects, arrays, maps, environment values, headers, params, or JSON.**
25. **Turning off `exactOptionalPropertyTypes` while relying on the semantic difference between “missing” and `undefined`.**
26. **Using `suppressImplicitAnyIndexErrors`.**
27. **Using `skipLibCheck` to hide your own broken declaration files.**
28. **Publishing broken `.d.ts` files.**
29. **Writing declaration files that lie about runtime behavior.**
30. **Module augmentation to paper over bad types instead of fixing or wrapping the dependency.**
31. **Declaring third-party modules as `any` instead of writing minimal safe types.**
32. **Using `Function` as a type.**
33. **Using `{}` as “empty object.”**
34. **Using `Object`, `object`, `String`, `Number`, `Boolean`, or other misleading broad/wrapper types when a precise type is possible.**
35. **Using boxed primitive types in annotations: `String`, `Number`, `Boolean`, `Symbol`, `BigInt`.**
36. **Using `unknown` but immediately casting it away.**
37. **Using generics as decoration.**
38. **Unconstrained generic soup.**
39. **Type-level cleverness that nobody can debug.**
40. **Using type gymnastics to avoid runtime checks.**
41. **Using `Partial<T>` as a lazy model for every update, draft, patch, form, and test fixture.**
42. **Using `DeepPartial<T>` in production domain code.**
43. **Using `Record<string, T>` when the key space is actually finite.**
44. **Using stringly typed IDs.**
45. **Using booleans to represent multi-state domain logic.**
46. **Using optional-property soup instead of discriminated unions.**
47. **Using `enum` or `const enum` casually in libraries without understanding emitted JS and package-boundary behavior.**
48. **Using numeric enums for protocol/domain values without explicit mapping.**
49. **Using `never` incorrectly so impossible states become reachable.**
50. **Using `satisfies` as if it validates runtime data.**
51. **Treating the compiler as the enemy.**
52. **Treating type errors as “red squiggles” instead of design feedback.**
53. **Adding `as any`, `@ts-ignore`, or `!` because the AI told you to.**
54. **Adding suppressions without understanding the original error.**
55. **Stacking suppressions.**
56. **Suppressing a rule globally because one file is hard.**
57. **Disabling type-aware linting because it is slower.**
58. **Disabling `tsc --noEmit` in CI.**
59. **Allowing production builds to emit with type errors.**
60. **Using `transpileOnly`, Babel, SWC, esbuild, or ts-loader modes as your only “type check.”**
61. **Using “works in dev” as a substitute for type checking.**
62. **Counting suppressions as normal debt rather than release-blocking debt.**
63. **Reviewing AI-generated TypeScript without searching for `any`, `as unknown as`, `@ts-ignore`, `@ts-nocheck`, and `!`.**
64. **Letting “temporary” suppressions survive past the PR.**
65. **Letting suppressions cross package/API boundaries.**
66. **Casting API responses directly to domain types.**
67. **Casting `fetch().json()` to `T` without validation.**
68. **Casting `JSON.parse` to `T` without validation.**
69. **Casting request bodies, query params, route params, cookies, headers, localStorage, sessionStorage, IndexedDB, env vars, CLI args, files, queues, webhooks, or database rows without validation.**
70. **Trusting frontend validation on the backend.**
71. **Letting TypeScript interfaces stand in for schemas.**
72. **Writing a Zod/Yup/Valibot/io-ts schema and then not using it at the boundary.**
73. **Letting schemas and TypeScript types drift.**
74. **Accepting arbitrary object input and spreading it into domain objects.**
75. **Mass assignment.**
76. **Treating unknown JSON as trusted because it came from your own service.**
77. **Not validating webhook signatures and payloads.**
78. **Not validating environment variables at process startup.**
79. **Parsing numbers with `Number(x)` and accepting `NaN`, `Infinity`, empty string, or whitespace.**
80. **Using `Date` parsing on untrusted strings without format validation.**
81. **Accepting unbounded arrays, strings, nested objects, or file sizes.**
82. **Assuming object keys are safe.**
83. **Merging untrusted objects into config objects.**
84. **Allowing `__proto__`, `prototype`, or `constructor` keys into object merge paths.**
85. **Using type assertions to bypass branded/opaque domain types.**
86. **Accepting impossible states from external systems and “fixing” them with casts.**
87. **Returning internal domain objects directly as API responses without explicit serialization.**
88. **Using the same type for database rows, domain entities, API DTOs, form state, and UI props.**
89. **Failing to distinguish “absent,” “null,” “undefined,” “empty string,” and “zero.”**
90. **Using `JSON.stringify`/`JSON.parse` cloning as a type or data-cleaning strategy.**
91. **Floating promises.**
92. **Calling an async function without `await`, `return`, or `.catch`.**
93. **Using `void someAsync()` without a documented fire-and-forget policy and internal error handling.**
94. **Using `Array.prototype.forEach(async () => ...)` and assuming it waits.**
95. **Using `map(async ...)` without `await Promise.all(...)`, `Promise.allSettled(...)`, or an intentional concurrency strategy.**
96. **Putting a Promise in an `if` condition.**
97. **Passing async callbacks to places expecting `void` without handling rejection.**
98. **Ignoring rejected promises in event handlers.**
99. **Swallowing errors in `.catch(() => {})`.**
100. **Using `try/catch` around async code but forgetting `await`.**
101. **Creating a `new Promise` around an already-Promise-returning API.**
102. **Using `Promise<any>`.**
103. **Using `Promise<unknown>` but never narrowing.**
104. **Serial `await` in loops where operations should be concurrent.**
105. **Unbounded concurrency against databases, queues, APIs, or file systems.**
106. **No timeout, cancellation, or `AbortSignal` for network calls.**
107. **No cleanup for timers, intervals, subscriptions, sockets, watchers, streams, or event listeners.**
108. **Ignoring backpressure in Node streams.**
109. **Using `setTimeout` as a synchronization primitive.**
110. **Race conditions hidden behind optimistic UI state.**
111. **No idempotency on retried async operations.**
112. **No transactional boundary around multi-step writes.**
113. **Not handling partial failure in `Promise.all`.**
114. **Treating background jobs as successful because they were enqueued.**
115. **Not logging async failures with correlation/request context.**
116. **Turning off `strictNullChecks`.**
117. **Using `!` instead of handling `null`/`undefined`.**
118. **Using truthiness checks where `0`, `""`, `false`, or `NaN` are valid values.**
119. **Using `||` defaults when `0`, `false`, or `""` should be preserved.**
120. **Non-exhaustive `switch` over discriminated unions.**
121. **Using a `default` branch to hide missing union cases.**
122. **Ignoring `never` exhaustiveness checks.**
123. **Allowing switch fallthrough.**
124. **Functions with missing returns.**
125. **Returning `undefined` from functions typed as returning real data.**
126. **Throwing inside a function that callers expect to return `undefined` or `null`.**
127. **Using exceptions for normal control flow without documenting it in the type.**
128. **Using `try/catch` to hide impossible states.**
129. **Leaving unreachable code.**
130. **Leaving unused variables/params as evidence of AI-generated dead branches.**
131. **Empty catch blocks.**
132. **`catch (e) {}` with no logging, recovery, rethrow, or metric.**
133. **Assuming caught values are `Error`.**
134. **Casting `catch (e)` to `Error` without checking.**
135. **Throwing strings, numbers, plain objects, or `null`.**
136. **Rejecting promises with non-`Error` values.**
137. **Returning `{ error: string }` from some functions and throwing from others with no convention.**
138. **Logging sensitive data in errors.**
139. **Returning stack traces to clients.**
140. **Converting all errors into generic 500s with no observability.**
141. **Suppressing errors because tests pass.**
142. **Failing to preserve `cause` when wrapping errors.**
143. **Using `console.log` as production observability.**
144. **Catching and retrying forever.**
145. **Retrying non-idempotent operations without safeguards.**
146. **Retrying authentication/authorization failures.**
147. **Not distinguishing user error, validation error, dependency error, and programmer error.**
148. **No correlation IDs/request IDs in logs.**
149. **No alerting on critical background-job failures.**
150. **No crash policy for truly unrecoverable invariant violations.**
151. **Using `eval`.**
152. **Using `new Function`.**
153. **Passing strings to `setTimeout`, `setInterval`, or similar implied-eval APIs.**
154. **Building SQL with string concatenation.**
155. **Building shell commands with string concatenation.**
156. **Passing user input to `child_process.exec`.**
157. **Using user-controlled paths without normalization and allowlisting.**
158. **Path traversal vulnerabilities in download/upload routes.**
159. **Trusting MIME type or file extension alone.**
160. **Storing uploaded files in publicly executable locations.**
161. **Rendering untrusted HTML.**
162. **Using `dangerouslySetInnerHTML` with unsanitized content.**
163. **Writing to `innerHTML` with untrusted content.**
164. **Putting untrusted data into script, style, URL, or HTML contexts without context-specific encoding.**
165. **Assuming React/Angular/Vue eliminate XSS completely.**
166. **Using untrusted attribute names or event-handler attributes.**
167. **Allowing `javascript:` URLs.**
168. **Allowing open redirects.**
169. **No server-side authorization checks.**
170. **Checking only authentication, not authorization.**
171. **Relying on hidden frontend fields for access control.**
172. **Trusting user IDs, org IDs, roles, prices, or permissions sent by the client.**
173. **No object-level authorization.**
174. **No tenant isolation checks.**
175. **Using wildcard CORS with credentials.**
176. **Hardcoding secrets, tokens, private keys, credentials, or API keys.**
177. **Committing `.env` files.**
178. **Logging secrets, tokens, cookies, auth headers, PII, or payment data.**
179. **Sending secrets to AI tools or pasting them into prompts.**
180. **Using weak crypto, homegrown crypto, MD5/SHA1 for passwords, or reversible password storage.**
181. **Not hashing passwords with a password-specific KDF.**
182. **Accepting unsigned or unverified JWTs.**
183. **Ignoring JWT audience, issuer, expiry, algorithm, or key rotation.**
184. **Using long-lived tokens where short-lived/session-scoped tokens are required.**
185. **No CSRF protection where cookies authenticate state-changing requests.**
186. **No rate limiting on login, signup, password reset, API mutation, or expensive endpoints.**
187. **No audit trail for sensitive actions.**
188. **No security headers for production web apps.**
189. **No dependency vulnerability scanning.**
190. **No secret scanning.**
191. **No threat model for security-critical features.**
192. **Treating TypeScript types as an authorization layer.**
193. **Installing packages suggested by AI without verifying they exist and are legitimate.**
194. **Running `npm install <ai-suggested-package>` blindly.**
195. **Adding a dependency for trivial code.**
196. **Adding abandoned, unmaintained, low-download, or recently-created packages without review.**
197. **Ignoring typosquatting risk.**
198. **Ignoring dependency confusion risk.**
199. **Committing `package.json` changes without the lockfile.**
200. **Not enforcing lockfile-based installs in CI.**
201. **Using `latest` or broad version ranges for critical tooling.**
202. **Not pinning the package manager version.**
203. **Ignoring install scripts/postinstall risk.**
204. **Ignoring licenses until release.**
205. **Copy-pasting code from README examples without security review.**
206. **Vendoring code without preserving license and provenance.**
207. **Deep-importing package internals.**
208. **Depending on private/internal APIs of libraries.**
209. **Using packages with no types by suppressing TypeScript instead of writing a typed adapter.**
210. **Adding duplicate libraries that solve the same problem.**
211. **Letting AI replace a standard library/API with a random package.**
212. **Not auditing bundle size after dependency changes.**
213. **Not removing unused dependencies.**
214. **Not separating runtime dependencies from dev/type-only dependencies.**
215. **Publishing packages that include secrets, tests, fixtures, local configs, or internal files.**
216. **Publishing with long-lived npm tokens when trusted publishing/OIDC is available.**
217. **No 2FA on npm/publishing accounts.**
218. **Merging AI-generated code you have not read.**
219. **Submitting AI-generated PRs you cannot explain.**
220. **Accepting code because it compiles once.**
221. **Accepting code because the demo path works.**
222. **Letting the AI make broad architectural changes without a spec.**
223. **Letting the AI add packages without explicit dependency review.**
224. **Letting the AI invent APIs, methods, config flags, package names, or file paths.**
225. **Not checking that imported symbols actually exist.**
226. **Not checking that documentation examples are current and safe.**
227. **Not checking runtime behavior after type fixes.**
228. **Generating huge PRs that mix feature work, refactors, formatting, dependency changes, and style churn.**
229. **Submitting code that does not follow project conventions.**
230. **Changing public APIs without migration notes.**
231. **Changing tests to fit broken code.**
232. **Deleting tests because they fail.**
233. **Generating fake tests that assert mocks, not behavior.**
234. **Generating tests that only prove the implementation repeats itself.**
235. **Generating comments that describe what code should do, while the code does something else.**
236. **Leaving AI “scaffolding” in production.**
237. **Leaving TODOs, placeholders, fake data, or “implement later” branches.**
238. **Letting the agent auto-approve, auto-merge, or auto-deploy its own work.**
239. **Giving coding agents unrestricted shell, filesystem, cloud, database, or GitHub permissions.**
240. **Letting agents read `.env`, private keys, tokens, customer data, or production dumps.**
241. **Letting agents mutate production data.**
242. **Prompting the AI with secrets.**
243. **Trusting AI output as a security review.**
244. **Trusting AI output as a legal/license review.**
245. **Trusting AI output as a substitute for domain understanding.**
246. **Not writing project rules for the AI assistant.**
247. **Using AI to bypass learning TypeScript.**
248. **No tests for new behavior.**
249. **Only testing the happy path.**
250. **No tests for invalid input.**
251. **No tests for null/undefined/missing fields.**
252. **No tests for authorization failures.**
253. **No tests for empty arrays, large arrays, duplicate IDs, unknown enum/union values, or malformed JSON.**
254. **No regression test for a bug fix.**
255. **Changing production code and snapshots together without inspecting the diff.**
256. **Snapshot-testing giant objects instead of asserting behavior.**
257. **Mocking the function under test.**
258. **Mocking away the boundary that actually needs testing.**
259. **Using `as any` in tests to bypass public APIs.**
260. **Using tests to validate impossible states without separating type-level tests from runtime tests.**
261. **Using fake timers without restoring them.**
262. **Leaving flaky tests.**
263. **Skipping tests with `.skip` to merge.**
264. **Marking tests `todo` for shipped behavior.**
265. **No type tests for exported library types.**
266. **No compile test for public examples.**
267. **No integration test for database queries, auth, serialization, or webhooks.**
268. **No property-based/fuzz testing for parsers, validators, permissions, money, dates, or security-critical code.**
269. **Testing implementation details instead of observable behavior.**
270. **Testing with data that cannot occur in production.**
271. **Not testing migrations.**
272. **Not testing rollback or partial failure.**
273. **No `tsc --noEmit` in CI.**
274. **No type-aware ESLint in CI.**
275. **No formatting check.**
276. **No test run before merge.**
277. **No build check before merge.**
278. **CI allowed to pass with TypeScript errors.**
279. **CI allowed to pass with lint errors.**
280. **CI allowed to pass with skipped tests.**
281. **CI allowed to pass with known critical vulnerabilities.**
282. **No lockfile enforcement.**
283. **No dependency review.**
284. **No license check for distributable products.**
285. **No bundle-size check for frontend changes.**
286. **No database migration check.**
287. **No generated-code freshness check.**
288. **Generated types not committed or not regenerated.**
289. **OpenAPI/GraphQL/protobuf clients hand-edited instead of regenerated.**
290. **Path aliases configured in TypeScript but not at runtime/test/bundler level.**
291. **Different module resolution between dev, test, build, and runtime.**
292. **Using `allowJs` without a migration plan and `checkJs`/types at the boundary.**
293. **Using `isolatedModules` incorrectly when a transpiler requires it.**
294. **Using incompatible `target`, `lib`, `module`, or `moduleResolution` settings because the AI copied a random config.**
295. **No pinned TypeScript version.**
296. **No pinned Node version.**
297. **No reproducible local setup.**
298. **Exporting `any` from a public API.**
299. **Exporting overly broad types that force consumers to cast.**
300. **Breaking exported types without semver.**
301. **Changing runtime behavior without changing types.**
302. **Changing types without matching runtime behavior.**
303. **Publishing declarations that reference non-exported internal types.**
304. **Publishing declarations that require consumers to install undeclared type dependencies.**
305. **Putting test/helper/internal types into public declarations.**
306. **Relying on global ambient declarations.**
307. **Using module augmentation as public API without documenting it.**
308. **Exporting mutable singletons.**
309. **Exporting classes when plain objects/functions would provide a safer API.**
310. **Exporting configuration objects that can be prototype-polluted.**
311. **Exporting functions with boolean-flag parameters.**
312. **Exporting positional parameters where an options object is needed.**
313. **Exporting unbranded strings for IDs, tokens, currencies, locales, or permissions.**
314. **Exporting union types without exhaustive handling guidance.**
315. **Exporting unstable internal paths.**
316. **Deep-importing from your own package internals in examples.**
317. **Publishing CJS/ESM/types mismatches.**
318. **Publishing source maps that expose secrets or private paths.**
319. **Publishing packages without `files`/publish allowlist.**
320. **Publishing generated code that was not reviewed.**
321. **Publishing examples that do not compile.**
322. **Publishing examples that are insecure when copy-pasted.**
323. **Typing props as `any`.**
324. **Typing event handlers as `any`.**
325. **Typing state as `any`.**
326. **Typing context as `any`.**
327. **Creating context with fake default values to avoid null handling.**
328. **Using `null!` or `{ } as ContextValue` as a context default.**
329. **Using `useRef<T>(null!)` casually.**
330. **Using `as` to force component props through instead of fixing prop types.**
331. **Using index as React key when order can change.**
332. **Disabling `react-hooks/exhaustive-deps` to silence stale closure bugs.**
333. **Leaving stale closures in effects.**
334. **Effects with async functions that leak or race.**
335. **No cleanup in effects for subscriptions, timers, observers, or requests.**
336. **No abort/cancellation for async effects.**
337. **Using `dangerouslySetInnerHTML` with untrusted data.**
338. **Trusting browser-only validation.**
339. **Trusting hidden form fields.**
340. **Putting secrets in frontend code.**
341. **Putting privileged business logic only in frontend code.**
342. **Using route params as typed values without parsing/validation.**
343. **Using localStorage/sessionStorage values as trusted typed data.**
344. **Over-broad component prop types that accept impossible combinations.**
345. **Boolean-prop soup instead of discriminated prop unions.**
346. **Uncontrolled/controlled input confusion hidden by casts.**
347. **Ignoring accessibility because the type checker does not complain.**
348. **Typing `req.body` as a domain object without validation.**
349. **Typing `req.user` globally without proving auth middleware ran.**
350. **Assuming middleware order instead of encoding it in route composition.**
351. **Using `process.env.X!`.**
352. **Using `Number(process.env.PORT)` without validation.**
353. **Using raw SQL string interpolation.**
354. **Using ORM escape hatches with untrusted strings.**
355. **No transaction for multi-write operations.**
356. **No authorization check near the data access or command boundary.**
357. **Returning database errors directly to clients.**
358. **Logging full request bodies.**
359. **Trusting headers like `x-user-id`, `x-forwarded-for`, or `x-admin` without trusted proxy/auth configuration.**
360. **No rate limiting.**
361. **No body-size limits.**
362. **No file-size limits.**
363. **No timeout limits.**
364. **No pagination limits.**
365. **No tenant scoping in queries.**
366. **No idempotency keys for payment/order/job submission.**
367. **No replay protection for webhooks.**
368. **No schema validation for queue messages.**
369. **No dead-letter handling for queues.**
370. **No graceful shutdown for servers/workers.**
371. **No resource cleanup for DB clients, Redis clients, file handles, browser instances, or workers.**
372. **Blocking the event loop with CPU-heavy work in request handlers.**
373. **Representing money as floating-point numbers.**
374. **Representing dates as ambiguous strings.**
375. **Ignoring time zones.**
376. **Mixing seconds and milliseconds.**
377. **Using `Date` for date-only values without a convention.**
378. **Using unbranded `string` for currencies, locales, countries, permissions, IDs, tokens, and roles.**
379. **Using `number` for database IDs when external systems treat them as strings.**
380. **Using optional fields for values that are required after a specific state transition.**
381. **No state machine for multi-step workflows.**
382. **No invariant checks at construction boundaries.**
383. **Letting invalid domain objects exist “temporarily.”**
384. **Using DTOs as domain models.**
385. **Using database rows as API responses.**
386. **Using API response shapes as form state.**
387. **Using `Partial<T>` for patch semantics where `undefined`, `null`, and absent mean different things.**
388. **Not modeling deleted/archived/disabled states explicitly.**
389. **Not modeling permissions as data.**
390. **Not modeling failure states.**
391. **Not modeling loading/loaded/error UI states as a union.**
392. **Letting impossible UI states be representable.**
393. **Giant files generated by AI with no structure.**
394. **Giant functions.**
395. **Deeply nested conditionals.**
396. **Copy-pasted logic.**
397. **Magic strings.**
398. **Magic numbers.**
399. **Boolean parameters.**
400. **Functions that both compute and perform I/O.**
401. **Hidden global state.**
402. **Mutating inputs unexpectedly.**
403. **Mutating exported objects.**
404. **Monkey-patching built-ins.**
405. **Prototype modification.**
406. **Side-effect imports that are undocumented.**
407. **Barrel files that create circular dependencies or hide import cost.**
408. **Circular dependencies.**
409. **No clear ownership boundaries.**
410. **No separation between domain, transport, persistence, and presentation.**
411. **Business logic embedded in React components or route handlers.**
412. **Database logic embedded in UI code.**
413. **No naming convention for files, types, components, hooks, services, or schemas.**
414. **Comments that restate the code.**
415. **Comments that lie.**
416. **Dead code.**
417. **Unused exports.**
418. **Unused dependencies.**
419. **TODOs with no owner/issue/date.**
420. **`console.log` and `debugger` in committed code.**
421. **Formatting churn mixed with logic changes.**
422. **Refactoring and feature work in the same PR without necessity.**
423. **Unbounded loops over user-controlled input.**
424. **Unbounded recursion.**
425. **Catastrophic regex.**
426. **No pagination.**
427. **Loading entire tables into memory.**
428. **N+1 queries.**
429. **No database indexes for new access patterns.**
430. **Serial network calls when concurrency is safe and required.**
431. **Unbounded concurrency when concurrency is unsafe.**
432. **Repeated JSON serialization of large objects.**
433. **Repeated object spreads in hot loops without measurement.**
434. **Expensive work in React render.**
435. **Creating unstable objects/functions that trigger unnecessary renders.**
436. **Shipping huge frontend dependencies for tiny tasks.**
437. **No bundle-size review.**
438. **Memory leaks from caches, maps, listeners, timers, or closures.**
439. **No cache invalidation strategy.**
440. **Caching per-user sensitive data globally.**
441. **No timeout or circuit breaker for dependency calls.**
442. **Ignoring backpressure.**
443. **Submitting a PR you cannot explain.**
444. **Submitting a PR you did not run.**
445. **Submitting a PR you did not test.**
446. **Submitting a PR with unrelated changes.**
447. **Submitting a PR with formatting churn.**
448. **Submitting a PR with generated files but no generator command.**
449. **Submitting generated code without marking provenance.**
450. **Submitting code that ignores project conventions.**
451. **Submitting huge AI-generated diffs.**
452. **Hiding risky changes in a large diff.**
453. **No migration plan for breaking changes.**
454. **No rollback plan for risky changes.**
455. **No changelog for public behavior changes.**
456. **No docs for new config/env/API behavior.**
457. **No screenshots or recordings for UI changes when needed.**
458. **No performance/security notes for risky code.**
459. **No explanation of added dependencies.**
460. **No license review for new dependencies.**
461. **No issue link for suppressions or workarounds.**
462. **Arguing “the AI wrote it” as an excuse.**
463. **Docs that do not compile.**
464. **README examples that do not type-check.**
465. **README examples that are insecure.**
466. **Docs showing `as any`, `@ts-ignore`, hardcoded secrets, string-concatenated SQL, or unvalidated input.**
467. **Docs that use fake APIs.**
468. **Docs that omit required env vars.**
469. **Docs that omit permissions/scopes.**
470. **Docs that omit error behavior.**
471. **Docs that omit security implications.**
472. **Docs that drift from code.**
473. **Public API docs generated from wrong declarations.**
474. **No migration docs for breaking type changes.**
475. **No examples for discriminated union states.**
476. **No explanation for branded/opaque types.**
477. **No “safe adapter” examples for untyped dependencies.**
478. `eslint-disable`
479. `!` non-null assertions
480. `JSON.parse(...) as T`
481. `fetch(...).then(r => r.json() as T)`
482. `req.body as Something`
483. string-built SQL
484. string-built shell commands
485. unbounded `Promise.all`
486. async `forEach`
487. `catch {}`
488. `console.log` in committed production code
489. generated huge PRs
490. new dependencies with no justification
491. package names suggested by AI but not verified
492. tests removed or weakened
493. type/lint rules disabled
494. no runtime validation at boundaries
495. secrets in code/prompts/logs
496. code the author cannot explain

## References

[1] https://www.typescriptlang.org/docs/handbook/2/basic-types.html "TypeScript: Documentation - The Basics"
[2] https://typescript-eslint.io/rules/no-explicit-any "no-explicit-any | typescript-eslint"
[3] https://typescript-eslint.io/rules/ban-ts-comment/ "ban-ts-comment | typescript-eslint"
[4] https://www.reddit.com/r/typescript/comments/1rcrsqd/tsignores_whole_purpose_is_to_hide_shty_type/ "@ts-ignore's whole purpose is to hide shty type errors that doesn't change anything in the output, but it's just creating another red line : r/typescript"
[5] https://docs.gitlab.com/development/fe_guide/style/typescript/ "TypeScript | GitLab Docs"
[6] https://www.reddit.com/r/typescript/comments/1nowu5e/i_hit_a_vibe_coding_wall_so_now_i_want_to_learn/ "I hit a vibe coding wall. So now I want to learn Typescript for real : r/typescript"
[7] https://typescript-eslint.io/rules/no-unsafe-assignment/ "no-unsafe-assignment | typescript-eslint"
[8] https://google.github.io/styleguide/tsguide.html "Google TypeScript Style Guide"
[9] https://www.typescriptlang.org/tsconfig/strictNullChecks.html "TypeScript: TSConfig Option: strictNullChecks"
[10] https://www.typescriptlang.org/tsconfig/noImplicitAny.html "TypeScript: TSConfig Option: noImplicitAny"
[11] https://www.typescriptlang.org/tsconfig/noUncheckedIndexedAccess.html "TypeScript: TSConfig Option: noUncheckedIndexedAccess"
[12] https://www.typescriptlang.org/tsconfig/skipLibCheck.html "TypeScript: TSConfig Option: skipLibCheck"
[13] https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html?utm_source=chatgpt.com "REST Security Cheat Sheet"
[14] https://zod.dev/?utm_source=chatgpt.com "Zod: Intro"
[15] https://typescript-eslint.io/rules/no-floating-promises "no-floating-promises | typescript-eslint"
[16] https://typescript-eslint.io/rules/no-misused-promises/ "no-misused-promises | typescript-eslint"
[17] https://www.typescriptlang.org/docs/handbook/2/narrowing.html "TypeScript: Documentation - Narrowing"
[18] https://eslint.org/blog/2025/01/differences-between-eslint-and-typescript/ "Differences between ESLint and TypeScript - ESLint - Pluggable JavaScript Linter"
[19] https://cheatsheetseries.owasp.org/cheatsheets/Query_Parameterization_Cheat_Sheet.html?utm_source=chatgpt.com "Query Parameterization Cheat Sheet"
[20] https://github.com/orgs/community/discussions/184568 "What are the differences from a 'vibe coding' dev and a dev who use AI wisely? · community · Discussion #184568 · GitHub"
[21] https://arxiv.org/html/2510.26103v1 "Security Vulnerabilities in AI-Generated Code: A Large-Scale Analysis of Public GitHub Repositories"
[22] https://typescript-eslint.io/getting-started/typed-linting "Linting with Type Information | typescript-eslint"
[23] https://www.typescriptlang.org/docs/handbook/project-references.html?utm_source=chatgpt.com "Project References - TypeScript: Documentation"
[24] https://www.typescriptlang.org/tsconfig/ "TypeScript: TSConfig Reference - Docs on every TSConfig option"
[25] https://typescript-eslint.io/blog/avoiding-anys "Avoiding `any`s with Linting and TypeScript | typescript-eslint"
[26] https://github.com/MetaMask/contributor-docs/blob/main/docs/typescript.md "contributor-docs/docs/typescript.md at main · MetaMask/contributor-docs · GitHub"
[27] https://typescript-eslint.io/rules/no-unsafe-type-assertion "no-unsafe-type-assertion | typescript-eslint"
[28] https://typescript-eslint.io/rules/no-unnecessary-type-assertion "no-unnecessary-type-assertion | typescript-eslint"
[29] https://typescript-eslint.io/rules/no-non-null-assertion/ "no-non-null-assertion | typescript-eslint"
[30] https://www.typescriptlang.org/docs/handbook/release-notes/typescript-3-9.html "TypeScript: Documentation - TypeScript 3.9"
[31] https://github.com/microsoft/TypeScript/issues/25166 "Report unneeded @ts-ignore directives that do nothing · Issue #25166 · microsoft/TypeScript · GitHub"
[32] https://typescript-eslint.io/rules/no-unsafe-function-type/?utm_source=chatgpt.com "no-unsafe-function-type"
[33] https://typescript-eslint.io/rules/no-empty-object-type?utm_source=chatgpt.com "no-empty-object-type"
[34] https://github.com/Expensify/App/blob/main/contributingGuides/STYLE.md "App/contributingGuides/STYLE.md at main · Expensify/App · GitHub"
[35] https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/stable-en/02-checklist/05-checklist "OWASP Secure Coding Practices - Quick Reference Guide | Secure Coding Practices | OWASP Foundation"
[36] https://cheatsheetseries.owasp.org/cheatsheets/Prototype_Pollution_Prevention_Cheat_Sheet.html "Prototype Pollution Prevention - OWASP Cheat Sheet Series"
[37] https://typescript-eslint.io/rules/no-floating-promises/ "no-floating-promises | typescript-eslint"
[38] https://typescript-eslint.io/rules/strict-boolean-expressions "strict-boolean-expressions | typescript-eslint"
[39] https://typescript-eslint.io/rules/switch-exhaustiveness-check/ "switch-exhaustiveness-check | typescript-eslint"
[40] https://eslint.org/docs/latest/rules/no-eval "no-eval - ESLint - Pluggable JavaScript Linter"
[41] https://typescript-eslint.io/rules/no-implied-eval/ "no-implied-eval | typescript-eslint"
[42] https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html "SQL Injection Prevention - OWASP Cheat Sheet Series"
[43] https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html "Cross Site Scripting Prevention - OWASP Cheat Sheet Series"
[44] https://github.com/OWASP/AISVS/blob/main/1.0/en/0x92-Appendix-C_AI_for_Code_Generation.md "AISVS/1.0/en/0x92-Appendix-C_AI_for_Code_Generation.md at main · OWASP/AISVS · GitHub"
[45] https://cheatsheetseries.owasp.org/cheatsheets/NPM_Security_Cheat_Sheet.html "NPM Security - OWASP Cheat Sheet Series"
[46] https://www.reddit.com/r/vibecoding/comments/1t05u9d/im_almost_done_building_a_typescriptelectron_app/ "I'm almost done building a TypeScript/Electron app that I planned to share and I realized I have NO IDEA what is in the license terms for the hundreds of modules I used : r/vibecoding"
[47] https://www.reddit.com/r/opensource/comments/1q853jb/help_how_do_i_deal_with_vibe_coders_that_try_to/ "Help! how do I deal with vibe coders that try to contribute? : r/opensource"
[48] https://cheatsheetseries.owasp.org/cheatsheets/AI_Agent_Security_Cheat_Sheet.html "AI Agent Security - OWASP Cheat Sheet Series"
[49] https://stefvanwijchen.com/the-typescript-vibe-coding-meta/ "The TypeScript vibe coding meta"
[50] https://cheatsheetseries.owasp.org/cheatsheets/Nodejs_Security_Cheat_Sheet.html "Nodejs Security - OWASP Cheat Sheet Series"
[51] https://www.typescriptlang.org/docs/handbook/declaration-files/do-s-and-don-ts.html "TypeScript: Documentation - Do's and Don'ts"
[52] https://www.typescriptlang.org/docs/handbook/2/everyday-types.html "TypeScript: Documentation - Everyday Types"
[53] https://www.typescriptlang.org/docs/handbook/modules/guides/choosing-compiler-options.html "TypeScript: Documentation - Modules - Choosing Compiler Options"
[54] https://github.com/tiktok/ts-bulk-suppress "GitHub - tiktok/ts-bulk-suppress · GitHub"
[55] https://www.reddit.com/r/typescript/comments/1oyj5nd/why_ai_put_everywhere_as_any/ "Why AI put everywhere \"as Any\" : r/typescript"
[56] https://owasp.org/www-project-top-ten/?utm_source=chatgpt.com "OWASP Top Ten Web Application Security Risks"
