# SCON Fuzzing

Install `cargo-fuzz` once:

```sh
cargo install cargo-fuzz
```

Run fuzz commands from the Rust package root:

```sh
cd rust
```

Run the parser target:

```sh
cargo +nightly fuzz run parse_str
```

Run the formatter invariants target:

```sh
cargo +nightly fuzz run format_source
```

Short local smoke runs:

```sh
cargo +nightly fuzz run parse_str -- -runs=10000
cargo +nightly fuzz run format_source -- -runs=10000
```

Reproduce a crash:

```sh
cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<crash-file>
```
