# Build the mdBook documentation into docs/book.
[group('docs')]
docs_build args="":
  mdbook build docs {{ args }}

# Serve the mdBook documentation locally with live reload.
[group('docs')]
docs_serve args="":
  mdbook serve docs {{ args }}
