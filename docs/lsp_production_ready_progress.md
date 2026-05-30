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
| M2: Production Analysis API | Not started |  |  |
| M3: AST/Trivia Source Formatter | Not started |  |  |
| M4: LSP Architecture Refactor | Not started |  |  |
| M5: Production Diagnostics And Formatting | Not started |  |  |
| M6: Completion, Hover, Definition, Symbols | Not started |  |  |
| M7: Configuration, Robustness, And Editor Integration | Not started |  |  |
| M8: Release Readiness | Not started |  |  |

## Milestone Log

### M1: Core Source Model

- Status: Complete
- Summary: Added byte spans to tooling-relevant AST nodes, introduced UTF-8/UTF-16 `LineIndex` mapping, added token/trivia/comment collection, and replaced heuristic analysis comments/symbols with parser-backed data.
- Checks: `cargo fmt --check`; `cargo test --workspace --exclude scon-fuzz`; `cargo clippy --workspace --exclude scon-fuzz --all-targets -- -D warnings`
- Commit: 90ede09
- Follow-up: M2 should build structured references, definitions, include-aware diagnostics, and document-store-backed analysis on top of these spans.

### M2: Production Analysis API

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:

### M3: AST/Trivia Source Formatter

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:

### M4: LSP Architecture Refactor

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:

### M5: Production Diagnostics And Formatting

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:

### M6: Completion, Hover, Definition, Symbols

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:

### M7: Configuration, Robustness, And Editor Integration

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:

### M8: Release Readiness

- Status: Not started
- Summary:
- Checks:
- Commit:
- Follow-up:
