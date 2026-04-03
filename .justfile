set shell := ["bash", "-euo", "pipefail", "-c"]

patch:
    cargo release patch --no-publish --execute

publish:
  cargo publish

ci:
  cargo fmt --all --check
