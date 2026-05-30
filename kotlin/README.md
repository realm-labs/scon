# Kotlin SCON

This directory contains Kotlin implementations that are independent of the
Rust workspace.

- `scon-core` is the reusable Kotlin SCON parser/resolver/formatter library.
- `../editors/idea` depends on `scon-core` for semantic behavior, while keeping
  IntelliJ Platform APIs out of the reusable library.

The root `tests/conformance` fixtures are the compatibility contract between
the Rust implementation, this Kotlin implementation, and future language ports.
