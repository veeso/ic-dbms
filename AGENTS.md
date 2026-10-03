# AGENTS.md

This file is the shared repository contract for contributors and coding agents.
Prioritize correctness, maintainability, test coverage, and idiomatic Rust.

## Project context

ic-dbms is a Rust framework for building relational database canisters on the
Internet Computer. It is the IC adapter for
[wasm-dbms](https://github.com/veeso/wasm-dbms), the runtime-agnostic DBMS
engine, which lives in its own repository and is consumed here through the
published `wasm-dbms`, `wasm-dbms-api`, `wasm-dbms-macros`, and
`wasm-dbms-memory` crates.

The engine provides tables, CRUD operations, queries, transactions, integrity
checks, migrations, and memory management. This repository adds Candid
serialization, the generated canister API, ACL-based access control, the IC
stable memory provider, canister lifecycle management, and client libraries.

## Working rules

- Read the relevant source, documentation, configuration, and tests before
  making changes.
- Keep changes scoped to the requested task and preserve unrelated user work.
- Use existing project patterns before adding new abstractions.
- Add or update focused tests before changing behavior.
- Update user documentation when commands, public APIs, configuration, or
  workflows change.
- Use `git`; never use `jj` or git worktrees.
- Never open a GitHub issue without prior user approval.
- Do not stage or commit local plans. Planning state belongs under the ignored
  `docs/superpowers/`, `.superpowers/`, or `.claude/plans/` directories.

## Workspace structure

```text
crates/
├── ic-dbms-api/            # IC types and wasm-dbms-api re-exports
├── ic-dbms-canister/       # Canister runtime, API helpers, IC memory provider
├── ic-dbms-macros/         # DbmsCanister procedural macro
├── ic-dbms-client/         # Canister client libraries
├── example/                # Reference database canister
└── integration-tests/
    ├── dbms-canister-client-integration/  # Wrapper canister for the client
    └── pocket-ic-tests/                   # PocketIC integration tests
```

The root `Cargo.toml` is a virtual workspace. Shared package metadata and every
dependency version live in `[workspace.package]` and
`[workspace.dependencies]`; member crates inherit them with `workspace = true`
and add only the features they need.

### Dependency graph

```text
ic-dbms-macros    (no workspace or engine dependencies)
ic-dbms-api       -> wasm-dbms-api, wasm-dbms-macros
ic-dbms-canister  -> ic-dbms-api, ic-dbms-macros, wasm-dbms, wasm-dbms-api,
                     wasm-dbms-macros, wasm-dbms-memory
ic-dbms-client    -> ic-dbms-api
```

The `wasm-dbms*` crates come from crates.io.

### Macro system

1. `#[derive(Table)]`, re-exported from wasm-dbms, generates `TableSchema`,
   record, request, and foreign fetcher types.
2. `#[derive(DatabaseSchema)]`, re-exported through
   `ic_dbms_canister::prelude`, generates the schema dispatch implementation.
3. `#[derive(DbmsCanister)]` generates the complete canister API: per-table
   CRUD and aggregate endpoints, the untyped `select`, transaction, ACL, and
   migration endpoints.

IC tables derive `CandidType` and `Deserialize`; the `#[candid]` attribute adds
Candid and Serde derives to the generated types. A canister schema combines
`DatabaseSchema` and `DbmsCanister`, then exports its Candid interface with
`ic_cdk::export_candid!()`.

## Rust conventions

- Follow `rustfmt.toml` and format Rust through `just fmt`.
- Clippy must pass with warnings denied.
- Public library modules and items require rustdoc documentation.
- Avoid `unsafe` unless the task requires it and its safety invariants are
  documented and tested.
- Prefer `#[expect]` with a reason over `#[allow]` for local lint overrides.
- Use `module_name.rs`; never introduce `mod.rs`.
- Keep dependency entries and feature definitions alphabetically sorted.
- Use bare, minimal dependency versions in `Cargo.toml`.

## Command interface

Use [`just`](https://just.systems) recipes when one exists. Do not bypass a
recipe with an ad hoc `cargo` or tool command. If a recurring task has no
recipe, add a focused recipe under `just/` before using it.

Run `just` to list every command. The primary recipes are:

```sh
just build                 # build every workspace target for the host
just build_all             # build the crates for wasm32 and the canisters
just test                  # unit tests, then documentation tests
just integration_test      # PocketIC tests; needs `just build_all_canisters`
just test_all              # unit, documentation, and integration tests
just coverage              # cargo llvm-cov, writes lcov.info
just bench_all             # run the Criterion benchmarks
just fmt                   # dprint fmt (Markdown, Rust, TOML, YAML)
just fmt_check
just lint "-- -D warnings" # alias of `just clippy`
just doc                   # cargo doc with RUSTDOCFLAGS="-D warnings"
just deny                  # cargo deny check
just docs_build            # build the mdBook site into docs/book
just scan_secrets
just check                 # the full local quality gate
just setup_githooks
```

`just check` is the required local quality gate. It runs formatting checks,
Clippy with warnings denied, rustdoc with warnings denied, cargo-deny, and the
unit and documentation tests. Run `just build_all_canisters` and
`just integration_test` as well when a change touches the canister API, the
macros, or the clients.

Canister artifacts (`.wasm`, `.wasm.gz`, `.did`) are written to `.artifact/`,
which is ignored by Git. Set `WASM_DIR` to use another directory.

When invoking compilation or test commands from the CLI, never request
parallelism greater than eight. This is an invocation constraint; do not encode
the local cap in tracked project files.

## Required tools

The local recipes expect:

- Rust and rustup. `rust-toolchain.toml` pins Rust 1.99.0 with the
  `wasm32-unknown-unknown` target. Crate manifests advertise Rust 1.91.1 as the
  published MSRV.
- just.
- dprint 0.56.1 and nightly rustfmt.
- cargo-deny.
- ic-wasm and candid-extractor for canister builds.
- mdBook for the documentation site.
- TruffleHog.
- git-cliff.
- cargo-llvm-cov for coverage.
- zizmor for workflow changes.
- shellcheck for shell hook changes.

PocketIC is downloaded by the integration test harness on first run.

If a required tool is unavailable, report it explicitly. Do not claim its check
passed or silently replace the repository command with a weaker check.

## Documentation

- The documentation site is an mdBook under `docs/`, published to
  <https://ic.wasm-dbms.cc> by `.github/workflows/pages.yml`.
- Register every new page in `docs/SUMMARY.md`.
- Link to the engine documentation at <https://wasm-dbms.cc> for generic
  topics (querying, transactions, relationships, data types, validation)
  instead of duplicating it.
- Use ATX headings and fenced code blocks with language identifiers.
- Keep Markdown lines readable, tables aligned, and files terminated by one
  newline.
- Run `fmt-md-tables -i <file>` after editing a Markdown file that contains a
  table.

## Database API surface (keep in sync)

The `Database` trait and the `DatabaseSchema` dispatch trait are defined in
wasm-dbms. When a wasm-dbms release adds, removes, or renames methods, changes
signatures, adds error variants, or extends `Query`, `Filter`, or `Value`,
propagate the change to every consumer below in the same pull request:

- `crates/ic-dbms-canister/src/api.rs`: generic per-operation helpers consumed
  by the macro-generated canister endpoints.
- `crates/ic-dbms-macros/src/dbms_canister.rs`: the `#[derive(DbmsCanister)]`
  macro, including per-table endpoints such as `select_<table>`,
  `aggregate_<table>`, `insert_<table>`, the shared select, transaction, and
  ACL endpoints.
- `crates/ic-dbms-client/src/client.rs`: the `Client` trait and all
  implementations:
  - `client/ic.rs`: canister-to-canister calls via `ic-cdk`.
  - `client/agent.rs`: external calls via `ic-agent`, behind the `ic-agent`
    feature.
  - `client/pocket_ic.rs`: integration tests, behind the `pocket-ic` feature.
- `crates/integration-tests/dbms-canister-client-integration/src/lib.rs`: the
  wrapper canister exposing the client method end to end.
- `crates/integration-tests/pocket-ic-tests/tests/`: coverage through both the
  direct client and the wrapper canister; register each test file in
  `tests/integration_tests.rs`.

Documentation that must follow the same change:

- `docs/reference/schema.md` for generated Candid endpoints.
- `docs/guides/client-api.md` for `Client` trait methods.
- `docs/reference/errors.md` for error variants.

When in doubt, search for the old method name across the workspace before
finishing; every match needs an update or an explicit deletion.

## GitHub Actions

- Run `zizmor .github/workflows` after every workflow change until it exits
  successfully with no findings.
- Pin every external action to the full commit SHA of its latest stable release
  and record the exact matching tag in a trailing comment.
- Declare explicit least-privilege permissions.
- Set `persist-credentials: false` on checkout steps unless later authenticated
  Git operations are explicitly required.
- Never interpolate attacker-controlled GitHub expressions directly into a
  shell script. Pass values through `env` and quote the shell variable.

## Git and releases

- Use Conventional Commits with an imperative, lower-case description.
- Do not add agent attribution, session links, or agent `Co-Authored-By` lines.
- Inspect the diff before staging or committing changes.
- Generate release notes with `just changelog_preview <version>` and
  `just changelog <version>`.
- Verify packages locally with `just publish "--dry-run --allow-dirty"`. The
  recipe packages `ic-dbms-macros`, `ic-dbms-api`, `ic-dbms-canister`, and
  `ic-dbms-client` in dependency order.
- Live publication uses crates.io trusted publishing through
  `.github/workflows/publish.yml`.
