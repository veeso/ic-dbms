# Build every workspace target in debug mode.
[group('build')]
build args="":
  cargo build --workspace --all-targets {{ args }}

# Build every workspace target in release mode.
[group('build')]
release args="":
  cargo build --workspace --all-targets --release {{ args }}

# Remove Cargo build artifacts and canister artifacts.
[group('build')]
[confirm("Are you sure you want to clean the build artifacts?")]
clean:
  rm -rf "{{ WASM_DIR }}"
  cargo clean

# Build the library crates for wasm32 and every canister artifact.
[group('build')]
build_all: build_library_wasm32 build_all_canisters

# Build the published library crates for wasm32-unknown-unknown.
[group('build')]
build_library_wasm32:
  cargo build --target wasm32-unknown-unknown --release --package "ic-dbms-macros"
  cargo build --target wasm32-unknown-unknown --release --package "ic-dbms-api"
  cargo build --target wasm32-unknown-unknown --release --package "ic-dbms-canister"
  cargo build --target wasm32-unknown-unknown --release --package "ic-dbms-client"

# Build every canister artifact into WASM_DIR.
[group('build')]
build_all_canisters: build_example_canister build_dbms_canister_client_integration_canister

# Build the example database canister.
[group('build')]
build_example_canister: (build_canister "example" "example")

# Build the wrapper canister used by the client integration tests.
[group('build')]
build_dbms_canister_client_integration_canister: (build_canister "dbms-canister-client-integration" "dbms_canister_client_integration")

# Build, shrink, and gzip one canister, and extract its Candid interface.
[private]
build_canister package_name wasm_name:
  mkdir -p "{{ WASM_DIR }}"
  cargo build --target wasm32-unknown-unknown --release --package "{{ package_name }}"
  ic-wasm "target/wasm32-unknown-unknown/release/{{ wasm_name }}.wasm" -o "{{ WASM_DIR }}/{{ wasm_name }}.wasm" shrink
  candid-extractor "{{ WASM_DIR }}/{{ wasm_name }}.wasm" > "{{ WASM_DIR }}/{{ wasm_name }}.did"
  gzip -k "{{ WASM_DIR }}/{{ wasm_name }}.wasm" --force
