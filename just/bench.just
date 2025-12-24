[group('test')]
bench bench_name="":
    echo "Running benchmark: {{ bench_name }}"
    cargo bench --bench "{{ bench_name }}"

bench_all:
    just bench "read_table"
