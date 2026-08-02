# Releasing

## One-time crates.io setup

The initial version of a new crate must be published manually:

```bash
cargo login
cargo publish --locked
```

After the initial publication, configure a trusted publisher in the crate's
crates.io settings with these values:

- GitHub owner: `m0ty`
- Repository: `bible-io-references-rust`
- Workflow: `publish.yml`
- Environment: leave blank

The workflow uses GitHub OIDC to obtain a short-lived crates.io token, so no
`CARGO_REGISTRY_TOKEN` GitHub secret is needed for later versions.

## Publishing a version

1. Update the version in `Cargo.toml` and the dated entry in `CHANGELOG.md`.
2. Run `cargo fmt --all -- --check`.
3. Run `cargo test --all-features --locked` and
   `cargo test --no-default-features --locked`.
4. Run `cargo clippy --all-targets --all-features --locked -- -D warnings`.
5. Run `cargo doc --all-features --no-deps --locked` with `RUSTDOCFLAGS` set
   to `-D warnings -D missing_docs`.
6. Commit and push the exact release source.
7. Tag that commit with the matching `Release-x.x.x` tag and push it:

```bash
git tag Release-1.2.0
git push origin Release-1.2.0
```

The `Publish` workflow checks that the tag version exactly matches
`Cargo.toml`, runs the release checks, packages the crate, and publishes it to
crates.io. Published versions are immutable, so every release needs a new
SemVer version and tag.

Do not use `--allow-dirty` for an actual release: crates.io metadata should
point to the exact source in the repository.
