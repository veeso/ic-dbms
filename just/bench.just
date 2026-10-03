# Run one Criterion benchmark by name.
[group('bench')]
bench bench_name:
  cargo bench --bench "{{ bench_name }}"

# Run every Criterion benchmark.
[group('bench')]
bench_all:
  just bench "read_table"
  just bench "eager_relation"

# Compile every benchmark without running it.
[group('bench')]
bench_build args="":
  cargo bench --workspace --no-run {{ args }}
