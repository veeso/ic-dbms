# Run unit, documentation, and PocketIC integration tests.
[group('test')]
test_all: test integration_test

# Run workspace unit tests, then documentation tests.
[group('test')]
test args="":
  cargo test --workspace --lib {{ args }}
  just test_macros {{ args }}
  just doc_test

# Run the macro crate unit tests without workspace feature unification.
[group('test')]
test_macros args="":
  cargo test --package ic-dbms-macros --lib {{ args }}

# Run workspace documentation tests.
[group('test')]
doc_test args="":
  cargo test --workspace --doc {{ args }}

# Run the PocketIC integration tests; run `just build_all_canisters` first.
[group('test')]
integration_test args="":
  cargo test --package pocket-ic-tests {{ args }}

# Generate an LCOV coverage report for the unit-tested crates.
[group('test')]
coverage output="lcov.info" args="":
  cargo llvm-cov \
    --no-fail-fast \
    --workspace \
    --lib \
    --lcov \
    --output-path {{ output }} \
    --ignore-filename-regex '.*(client|macros|example|pocket-ic-tests).*' \
    --exclude "dbms-canister-client-integration" \
    --exclude "example" \
    --exclude "ic-dbms-client" \
    --exclude "ic-dbms-macros" \
    --exclude "pocket-ic-tests" \
    --exclude-from-report "dbms-canister-client-integration" \
    --exclude-from-report "example" \
    --exclude-from-report "ic-dbms-client" \
    --exclude-from-report "ic-dbms-macros" \
    --exclude-from-report "pocket-ic-tests" \
    {{ args }}
