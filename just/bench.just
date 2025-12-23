[group('test')]
bench bench_name="":
    cargo bench {{ bench_name }}
