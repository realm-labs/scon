# SCON

SCON is a strict configuration format with shared conformance fixtures and
multiple language/tooling implementations in one repository.

## Repository Layout

- `rust/`: Rust workspace for `scon-core`, `scon-cli`, `scon-lsp`, and fuzzing.
- `kotlin/`: Kotlin build root, reusable `scon-core` implementation, and
  serialization adapters.
- `go/`: Go SCON core implementation and reflection-based typed adapter.
- `typescript/`: TypeScript SCON core implementation and Zod adapter.
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
./gradlew :scon-core:test :scon-kotlinx-serialization:test
./gradlew :idea-plugin:buildPlugin
./gradlew :idea-plugin:verifyPlugin
```

```sh
cd go
go test ./...
go vet ./...
```

```sh
cd typescript
pnpm install --frozen-lockfile
pnpm test
pnpm typecheck
```
