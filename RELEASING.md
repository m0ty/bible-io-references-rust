# Releasing

1. Update the version in `Cargo.toml` and the dated entry in `CHANGELOG.md`.
2. Run `cargo fmt --all -- --check`.
3. Run `cargo test --all-features` and `cargo test --no-default-features`.
4. Run `cargo clippy --all-targets --all-features -- -D warnings`.
5. Run rustdoc with warnings denied.
6. Commit the exact release source.
7. Run `cargo publish --dry-run --locked` from that clean commit.
8. Tag and push the verified release commit.
9. Publish with `cargo publish --locked`.

Do not use `--allow-dirty` for an actual release: crates.io metadata should
point to the exact source in the repository.
