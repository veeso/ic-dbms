# Run all unit tests and integration tests
[group('test')]
test_all: test integration_test

[group('test')]
test test_name="":
  cargo test --lib {{test_name}}
  cargo test --doc

[group('test')]
integration_test test_name="":
  echo "Running integration tests..."
  cargo test {{test_name}} -p pocket-ic-tests
