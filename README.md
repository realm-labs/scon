# SCON

SCON is a strict configuration format with shared conformance fixtures and
multiple language/tooling implementations in one repository.

## Repository Layout

- `rust/`: Rust workspace for `scon-core`, `scon-cli`, `scon-lsp`, and fuzzing.
- `kotlin/`: Kotlin build root and reusable `scon-core` implementation.
- `editors/`: editor integrations and Tree-sitter grammar.
- `tests/conformance/`: language-neutral parse and resolve fixture suite.
- `docs/`: specifications, release notes, and tooling documentation.

## Common Checks

```sh
cd rust
cargo fmt --check
cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings
cargo test --workspace --exclude scon-fuzz
```

```sh
cd kotlin
./gradlew :scon-core:test
./gradlew :idea-plugin:buildPlugin
./gradlew :idea-plugin:verifyPlugin
```
