# Production-Ready SCON LSP Progress

This document records milestone progress for the production-ready SCON LSP work.

Update this document after each milestone is complete. Commit the update together with the milestone implementation. Do not commit `docs/lsp_production_ready_plan.md`.

## Execution Rules

- Before starting any milestone, read:
  - `docs/lsp_production_ready_plan.md`
  - this progress document
- Execute milestones in order.
- Keep the architecture clean and structural.
- Do not add compatibility layers for old internal shapes.
- After completing a milestone, record:
  - status;
  - summary;
  - checks run;
  - commit hash;
  - follow-up work.

## Milestone Status

| Milestone | Status | Commit | Notes |
| --- | --- | --- | --- |
| M1: Core Source Model | Complete | 90ede09 | Span-aware AST, line index, token/comment collection, parser-backed symbols. |
| M2: Production Analysis API | Complete | 6c2e5fc | Structured diagnostics, definitions, references, include references, and source-store analysis. |
| M3: AST/Trivia Source Formatter | Complete | 923eee3 | AST-walk source formatter with token comment preservation and round-trip tests. |
| M4: LSP Architecture Refactor | Complete | d931d4d | Split LSP into server/state/config/feature modules backed by core analysis. |
| M5: Production Diagnostics And Formatting | Complete | 107978e | Precise reference diagnostics, include-location errors, dependent reanalysis tests, and parseable formatting tests. |
| M6: Completion, Hover, Definition, Symbols | Complete | 19f728f | Semantic completion, hover previews, include definition, and nested symbols. |
| M7: Configuration, Robustness, And Editor Integration | Complete | pending | Runtime config, editor settings, logging, and release build verification. |
| M8: Release Readiness | Not started |  |  |

## Milestone Log

### M1: Core Source Model

- Status: Complete
- Summary: Added byte spans to tooling-relevant AST nodes, introduced UTF-8/UTF-16 `LineIndex` mapping, added token/trivia/comment collection, and replaced heuristic analysis comments/symbols with parser-backed data.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`
- Commit: 90ede09
- Follow-up: M2 should build structured references, definitions, include-aware diagnostics, and document-store-backed analysis on top of these spans.

### M2: Production Analysis API

- Status: Complete
- Summary: Replaced the first-pass analysis surface with `AnalyzedDocument`, structured diagnostics, definitions, references, include references, source-store-backed file analysis, and span-backed diagnostic ranges.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`
- Commit: 6c2e5fc
- Follow-up: M3 should replace the remaining line-based formatter with an AST/trivia formatter while keeping resolved canonical formatting separate.

### M3: AST/Trivia Source Formatter

- Status: Complete
- Summary: Replaced the line-based source formatter with an AST-walk formatter that preserves comments, include directives, object/array spreads, substitutions, source order by span sorting, and keeps resolved canonical formatting separate.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`; `cargo +nightly fuzz run format_source -- -runs=10000`
- Commit: 923eee3
- Follow-up: M4 should split `scon-lsp` into structured server/state/config/feature modules and consume the structured core APIs instead of ad hoc scans.

### M4: LSP Architecture Refactor

- Status: Complete
- Summary: Split `scon-lsp` into startup, server dispatch, workspace state, config, position conversion, diagnostics, formatting, hover, completion, definition, and symbols modules with a `DocumentStore`, include graph, reverse dependency tracking, and per-document diagnostics cache.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`
- Commit: d931d4d
- Follow-up: M5 should harden diagnostics and formatting behavior, add protocol tests, and report include diagnostics at the include directive where possible.

### M5: Production Diagnostics And Formatting

- Status: Complete
- Summary: Hardened diagnostics by using substitution/interpolation spans during evaluation, attaching include load failures to include directives, reanalyzing reverse dependencies from open-document state, and testing full-document formatting output parses.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`
- Commit: 107978e
- Follow-up: M6 should expand semantic editor features for path-aware completion, hover previews, include go-to-definition, and nested document symbols.

### M6: Completion, Hover, Definition, Symbols

- Status: Complete
- Summary: Added analysis-derived path/reference completion, include path completion, keyword completion, hover diagnostics and resolved value previews, include go-to-definition, and nested document symbols with stable ranges.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`
- Commit: 19f728f
- Follow-up: M7 should wire user/workspace configuration, robustness behavior, and editor frontend integration details.

### M7: Configuration, Robustness, And Editor Integration

- Status: Complete
- Summary: Added runtime JSON configuration for include root, formatting, resolve-on-change, and max file size; applied initialization and configuration-change settings; added config logging; and updated VS Code, Neovim, and Zed integration settings.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`; `cargo build --workspace --exclude scon-fuzz --release`
- Commit: pending
- Follow-up: M8 should add end-user LSP docs, troubleshooting, release checklist, CI coverage, and fuzz smoke release verification.

### M8: Release Readiness

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:
