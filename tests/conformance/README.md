# SCON v0.1 Conformance Fixtures

This directory contains language-neutral conformance cases for SCON v0.1.
They cover parse and sequential resolution behavior only. Source formatting is
intentionally out of scope.

## Layout

- `manifest.json` lists every case.
- `cases/<category>/<case-id>/main.scon` is the entry file unless the manifest
  says otherwise.
- Valid cases include `expected.json`.
- Invalid cases include `error.json`.
- Include cases may contain additional `.scon` files next to `main.scon`.

## Running A Case

An implementation should:

1. Load `manifest.json`.
2. Resolve each `entry` relative to this `tests/conformance` directory.
3. Parse and resolve that entry file with the entry file's directory as the
   default include root.
4. For `kind = "valid"`, compare the resolved data model to `expected.json`.
5. For `kind = "invalid"`, assert that resolution fails and compare only the
   stable error `code` from `error.json`.

Diagnostic messages, byte ranges, line/column positions, and include stacks are
not part of fixture compatibility.

## Number Expectations

Expected values use JSON numbers. This describes the spec-level resolved data
model and does not require implementations to use Rust's internal numeric
representation.
