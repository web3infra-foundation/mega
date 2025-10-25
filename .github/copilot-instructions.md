# Mega – Repository Custom Instructions for GitHub Copilot

## What this repo is

Mega is an unofficial open-source implementation of Google Piper: a monorepo/monolithic codebase management system with Git compatibility, FUSE mounting, and Buck2 integration. When proposing designs or code, optimize for very large repos, content-addressed storage, and Git internals (packfiles, MIDX, commit-graph, delta chains). License is Apache-2.0 + MIT; keep new files compatible with both.

## Languages & defaults

- Prefer Rust (Edition 2021+). Use async/await with Tokio, and structured logging with tracing.
- Serialization: serde; CLI: clap; errors: thiserror (library) and anyhow (bin/tests/tools).
- Avoid unsafe unless there’s a measurable win (perf or FFI). If unsafe is required, add a // SAFETY: rationale comment and tests.
- For small utilities, you may use Python or Bash, but default to Rust when feasible.


## Build & run

- Primary build: Buck2. Provide buck2 rules/macros and examples when adding code or docs.
- Crate local build: cargo build -p <crate> is welcomed for dev loops; ensure parity with Buck2 rules.
- When generating commands, prefer:

```bash
buck2 build //... for CI-style full builds
buck2 test //... for tests
cargo nextest run -p <crate> when suggesting fast local test runs
```

## Key Components (Modules)

This is a monorepo containing many components. Some key ones include:

* **`scorpio`**: A FUSE (Filesystem in Userspace) implementation, allowing the monorepo to be mounted as a local filesystem. This is a complex Rust component.
* **`mono` / `ceres` / `jupiter` / `moon`**: These are various services and libraries within the monorepo, primarily written in Rust and TypeScript.

## Coding style & quality

- Run rustfmt defaults.
- Treat clippy warnings as errors for new/changed code; prefer #[must_use], #[deny(unsafe_op_in_unsafe_fn)] where appropriate.
- Return Result<T, E>; avoid unwrap()/expect() in library code except in tests or when invariant-proofed and documented.
- Prefer iterator and slice APIs to heap-allocating vectors in hot paths. Use SmallVec, bytes, or no_std-friendly patterns when beneficial.

## Performance & memory

- Mega targets huge repos; prioritize O(n) single-pass algorithms, streaming IO, mmap when safe, and bounded allocations.
- When dealing with Git objects/packs, consider delta-chain depth, fanout tables, OID (SHA-1 vs SHA-256), and zstd/deflate trade-offs. Include micro-benchmarks for hot paths via criterion.

## Filesystems & FUSE (Scorpio)

- Treat FUSE paths as authoritative views over the monorepo; keep operations atomic and consistent.
- Minimize kernel round-trips; batch lookups and use negative dentry caching where possible.
- Add tests that simulate rename/replace, deep trees, and large directory fanout.

## API & CLI guidelines

- Public APIs: lean toward stable, versioned boundaries; avoid leaking internal pack/layout details unless clearly documented.
- CLI should default to safe, read-only modes for destructive commands; provide --dry-run and --json where reasonable.

## Testing

- Write unit + integration + property tests. Use proptest for object/pack fuzzing, and insta for snapshots when output is textual.
- Add concurrency tests (Tokio multi-task) where IO/state is involved.
- Prefer hermetic tests with temp dirs; avoid network unless explicitly required.

## Observability & errors

- Use tracing spans/fields; do not log secrets.
- Provide actionable error messages (context via anyhow::Context), and map OS/IO errors precisely.

## Security & compliance

- No plaintext secrets, keys, or tokens in code or tests.
- Default to Rustls for TLS; avoid new C FFI unless justified.
- Keep third-party code in third-party/ with license notices; check compatibility with Apache-2.0/MIT.

## Documentation

- Add module-level docs (//!) and public item docs (///) for all public items.
- For complex algorithms (e.g., pack rewriter, multi-pack-index), include a short overview, invariants, and example.
- English comments preferred; Chinese allowed for tricky parts (bilingual welcome).

## Git workflow

We favor Trunk-Based Development and Conventional Commits.

### Commit messages

- Concise imperative subject; body explains why and outlines bench/test evidence.
- Use present tense (e.g., "Add feature X" not "Added feature X").
- Keep lines under 72 characters.
- Reference issues/PRs liberally (e.g., "Fix #123" or "Add feature X (#123)").

### How to talk back to us (Copilot prompts)

- When the user asks for code, emit Rust first, then Buck2 snippets/macros if relevant.
- When suggesting design options, list trade-offs (perf, memory, portability, compatibility with Git).
- Prefer minimal, composable abstractions; avoid premature global singletons.

### Non-goals

- Do not recommend migrating away from Git without explicit request.
- Do not propose language rewrites away from Rust unless integrating with existing systems.

### How to Assist Developers (Guidelines for Copilot)

* **DON'T:** Propose language rewrites away from Rust unless integrating with existing systems.
* **DO:** When generating code, provide solutions in **Rust** for backend/core logic and **TypeScript** for frontend/tooling/SDKs.
* **DO:** Ensure all commit message examples adhere to the **Conventional Commits** standard.
* **DO:** When asked about building or testing, refer to the **Buck2** build system. Avoid suggesting root-level `cargo` or `npm` commands unless they are specific to a sub-package.
* **DON'T:** Confuse this project with "mega.nz" (the cloud storage service) or other projects named "Mega." This project is "mega" for version control infrastructure.
* **DO:** Point users to `CONTRIBUTING.md` for contribution guidelines and setup instructions.