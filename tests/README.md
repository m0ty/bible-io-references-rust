# Test layout

The files under `unit/` are compiled through `#[path]` declarations in their
corresponding production modules. This keeps all test implementations outside
`src/` while preserving access to private implementation details.

`public_api.rs` is a conventional integration test compiled as an external
consumer of the crate. Additional cross-module integration tests belong beside
it.
