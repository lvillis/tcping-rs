set shell := ["bash", "-euo", "pipefail", "-c"]

patch:
  cargo release patch --no-publish --execute

publish:
  cargo publish

ci:
  cargo fmt --all --check
  cargo check --all-targets --locked
  cargo clippy --all-targets --locked -- -D warnings
  cargo nextest run --locked
