# Package and publish every public crate to crates.io in dependency order.
[group('publish')]
publish args="":
  cargo publish --locked --package ic-dbms-macros --package ic-dbms-api --package ic-dbms-canister --package ic-dbms-client {{ args }}

# List the files included in each packaged crate.
[group('publish')]
package_list args="":
  cargo package --list --exclude-lockfile --package ic-dbms-macros --package ic-dbms-api --package ic-dbms-canister --package ic-dbms-client {{ args }}
