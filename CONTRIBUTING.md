# Contributing to ic-dbms

Thank you for your interest in contributing to **ic-dbms**! This document
describes how to set up a local development environment, the workflow used for
changes, and the conventions every contribution must follow. AI-assisted
contributions must also follow the [AI policy](./AI_POLICY.md).

By participating in this project you agree to abide by the
[Code of Conduct](./CODE_OF_CONDUCT.md).

## Ways to contribute

- Fixing bugs and reporting reproducible issues.
- Improving documentation under `docs/` (rendered at <https://ic.wasm-dbms.cc>).
- Adding test coverage to existing crates.
- Implementing new features (please open an issue first to discuss the design).
- Reviewing open pull requests.

If you are unsure whether a change is welcome, open a GitHub issue and ask
before investing time in a large patch.

Changes to the database engine itself (queries, transactions, memory
management, the `Table` macro) belong in the
[wasm-dbms](https://github.com/veeso/wasm-dbms) repository.

## Repository layout

| Path                        | Purpose                                                        |
| --------------------------- | -------------------------------------------------------------- |
| `crates/ic-dbms-api/`       | IC-specific types and `wasm-dbms-api` re-exports.              |
| `crates/ic-dbms-canister/`  | Canister runtime, API helpers, and the stable memory provider. |
| `crates/ic-dbms-macros/`    | The `DbmsCanister` derive macro.                               |
| `crates/ic-dbms-client/`    | Client libraries for the generated canister.                   |
| `crates/example/`           | Reference database canister.                                   |
| `crates/integration-tests/` | Wrapper canister and PocketIC integration tests.               |
| `docs/`                     | mdBook documentation.                                          |
| `just/`                     | Modular `Justfile` recipes for build, test, and release work.  |

## Development environment

### Required tooling

- **Rust toolchain** pinned by [`rust-toolchain.toml`](./rust-toolchain.toml)
  (currently `1.99.0`) with the `wasm32-unknown-unknown` target. `rustup`
  installs it automatically the first time you run a `cargo` command in the
  repository.
- **Nightly Rust** with `rustfmt` (used by dprint for Rust formatting only):

  ```sh
  rustup toolchain install nightly --component rustfmt
  ```

- [`just`](https://github.com/casey/just), the task runner used by every
  workflow in CI.
- [`dprint`](https://dprint.dev/), the formatter for Rust, Markdown, TOML, and
  YAML.
- [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) for dependency
  policy checks.
- [`ic-wasm`](https://github.com/dfinity/ic-wasm), which shrinks canister WASM
  artifacts.
- [`candid-extractor`](https://crates.io/crates/candid-extractor), which
  extracts `.did` files from built canisters.
- [`mdBook`](https://rust-lang.github.io/mdBook/) to build the documentation.
- [`git-cliff`](https://git-cliff.org/) for the generated changelog.
- [`zizmor`](https://docs.zizmor.sh) for GitHub Actions security checks.
- [`TruffleHog`](https://github.com/trufflesecurity/trufflehog) for secret
  scanning.

Install the Cargo-based IC tools with:

```sh
cargo install ic-wasm candid-extractor
```

[PocketIC](https://github.com/dfinity/pocketic), the local IC replica used by
the integration tests, is downloaded automatically by the test harness on first
run. It supports Linux and macOS.

### First-time setup

```sh
git clone https://github.com/veeso/ic-dbms.git
cd ic-dbms

# Install the tracked Git hooks
just setup_githooks

# Run the repository's normal quality gate
just check
```

## Common commands

A non-exhaustive cheat sheet (run `just --list` for everything):

| Command                         | Description                                                 |
| ------------------------------- | ----------------------------------------------------------- |
| `just build_all`                | Builds the crates for wasm32 and every canister artifact.   |
| `just test`                     | Runs unit and doc tests across the workspace.               |
| `just test <name>`              | Filters unit tests by substring.                            |
| `just integration_test [name]`  | Runs the PocketIC integration tests.                        |
| `just test_all`                 | Runs unit, doc, and integration tests.                      |
| `just fmt`                      | Formats supported files with dprint.                        |
| `just fmt_check`                | Checks Rust, Markdown, TOML, and YAML formatting.           |
| `just clippy`                   | Runs workspace Clippy checks.                               |
| `just doc`                      | Builds warning-free workspace documentation.                |
| `just deny`                     | Runs cargo-deny policy checks.                              |
| `just docs_build`               | Builds the mdBook site into `docs/book/`.                   |
| `just scan_secrets .`           | Scans the repository for secrets with TruffleHog.           |
| `just check`                    | Runs the normal local quality gate.                         |
| `just coverage`                 | Runs the workspace coverage command used by CI.             |
| `just changelog_preview 0.10.0` | Previews generated release notes.                           |
| `just clean`                    | Removes `.artifact/` and `target/` (asks for confirmation). |

Before opening a pull request, run the full suite:

```sh
just build_all   # wasm32 library builds and canister artifacts
just test_all    # unit tests, doc tests, and PocketIC integration tests
just check       # formatting, clippy, docs, dependency policy, and tests
```

## Workflow

1. Discuss non-trivial design changes before implementation and link the
   relevant issue in the pull request when one exists.
2. Fork the repository and create a topic branch from `main`.
3. Implement your change with tests.
4. Run `just check` and the relevant build and test recipes locally.
5. Update documentation under `docs/` and preview generated release notes if
   user-visible behaviour changes (see [Changelog](#changelog)).
6. Open a pull request against `main` describing the motivation, the approach,
   and any follow-ups.

## Conventions

### Code style

- Format supported files with dprint: `just fmt`; CI fails on any diff from
  `just fmt_check`.
- Rust formatting is delegated to the configured nightly `rustfmt` command.
- Lint clean under `just clippy "-- -D warnings"`.
- Prefer `where` clauses over inline trait bounds on generic parameters.
- No `unsafe` without an accompanying `// SAFETY:` comment that justifies the
  invariants.
- Keep public items documented; the project relies on `docs.rs` for the API
  reference.
- `Cargo.toml` files keep dependencies alphabetically sorted and inherit
  versions from the workspace.

### Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/).
Examples:

```text
feat(client): add aggregate method to the Client trait
fix(canister): reject anonymous principals in ACL grants
docs(guides): clarify ACL bootstrap flow
chore(ci): cache cargo registry between jobs
```

The release notes are generated from these prefixes by `git-cliff` (see
[`cliff.toml`](./cliff.toml)).

### Branches

- Feature branches: `feat/<issue>-<slug>`.
- Bug-fix branches: `fix/<issue>-<slug>`.
- Documentation-only: `docs/<slug>`.

### Pull requests

- Keep PRs focused; split unrelated changes into separate PRs.
- Reference the GitHub issue in the PR description (`Closes #N`).
- The PR title should also follow Conventional Commits.
- All CI jobs must be green before review.

### Documentation

User-facing changes must update the relevant pages under `docs/guides/` and
`docs/reference/`, and new pages must be registered in
[`docs/SUMMARY.md`](./docs/SUMMARY.md). Generic engine topics are documented in
the wasm-dbms repository; link to <https://wasm-dbms.cc> instead of duplicating
them.

Design notes and implementation plans live in `.superpowers/` and are never
committed.

### Changelog

User-visible changes are generated from Conventional Commit history. Preview
the next notes with `just changelog_preview <version>` and generate the entry
with `just changelog <version>`.

### Database API surface

The `Database` trait and `DatabaseSchema` dispatch trait are defined in
wasm-dbms and mirrored by several consumers here. When a wasm-dbms release
changes either of them, update **every** surface below in the same PR:

- `crates/ic-dbms-canister/src/api.rs`: generic helpers used by the canister
  macro.
- `crates/ic-dbms-macros/src/dbms_canister.rs`: the `#[derive(DbmsCanister)]`
  endpoint generator.
- `crates/ic-dbms-client/src/client.rs` and the three implementations under
  `client/` (`ic.rs`, `agent.rs`, `pocket_ic.rs`).
- `crates/integration-tests/dbms-canister-client-integration/src/lib.rs`: the
  wrapper canister.
- `crates/integration-tests/pocket-ic-tests/tests/`: coverage for both the
  direct client and the wrapper canister; remember to register the new test in
  `tests/integration_tests.rs`.

Documentation that must follow the same change: `docs/reference/schema.md`,
`docs/guides/client-api.md`, and `docs/reference/errors.md`.

## Testing guidelines

- Every public function should have at least one unit test exercising the
  happy path and the most relevant failure modes.
- Use the in-memory `MemoryProvider` for fast unit tests; reach for PocketIC
  only when you need true canister semantics (cycles, inter-canister calls,
  upgrades).
- Doc tests are part of CI (`just doc_test`); keep code samples in `///` blocks
  compiling.
- Benchmarks live under `crates/ic-dbms-canister/benches/`; CI builds them but
  does not measure performance. Run them locally with `just bench_all`.

## Reporting bugs and requesting features

Use the issue templates under [`.github/ISSUE_TEMPLATE/`](./.github/ISSUE_TEMPLATE/).
For bug reports include:

- Crate version (or commit SHA).
- Minimal reproduction (preferably a failing test).
- Expected vs. actual behaviour.
- Environment (PocketIC, local replica, mainnet).

## Security issues

Do **not** open public issues for security vulnerabilities. Email
<christian.visintin@veeso.dev> with the details and a way to reproduce the
problem. You will receive an acknowledgement within a few business days.

## License

By contributing you agree that your contributions will be licensed under the
[MIT License](./LICENSE) that covers the project.
