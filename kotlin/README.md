# Kotlin SCON

This directory contains the Kotlin build root and Kotlin implementations. It is
independent of the Rust workspace under `../rust`.

- `scon-core` is the reusable Kotlin SCON parser/resolver/formatter library.
- `../editors/idea` depends on `scon-core` for semantic behavior, while keeping
  IntelliJ Platform APIs out of the reusable library.

Useful commands:

```sh
./gradlew :scon-core:test :scon-kotlinx-serialization:test
./gradlew :idea-plugin:test
./gradlew :idea-plugin:buildPlugin
./gradlew :idea-plugin:verifyPlugin
```

The root `tests/conformance` fixtures are the compatibility contract between
the Rust implementation, this Kotlin implementation, and future language ports.
