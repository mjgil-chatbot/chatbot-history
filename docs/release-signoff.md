# Release Sign-Off

This document is the stable public release checklist for the repository. The
older pass-by-pass task ledgers were removed so the remaining documentation
surface stays focused on architecture, behavior, and release validation.

## Required Validation

Local validation and GitHub Actions should run the same commands:

```bash
cargo fmt --all --check
cargo check --release --workspace
cargo test --workspace --release
cargo doc --release --workspace --no-deps
```

## Sign-Off Criteria

A release is ready only when all of the following are true:

- Architecture and crate boundaries in `docs/design.md` match the current implementation.
- Public behavior docs are current for restore verification, cursor navigation, and audit workflow.
- Source, test, and documentation evidence all exist for any feature claimed as complete in a release checklist row.
- CI passes with the workspace commands listed above.
- The repository contains no machine-specific paths, private operational notes, or temporary task-tracking documents.

## Evidence Review

The evidence gate reads this document when a release adds a markdown status
table with completion claims. Rows marked `Complete` must be backed by real
repository evidence instead of text-only declarations.

When a release needs explicit checklist rows:

- Use a markdown table with a `Status` column.
- Limit `Complete` to items with matching source, tests, and documentation.
- Prefer stable feature names that map clearly to repository code and docs.

Detailed downgrade rules and scanner behavior are documented in
`docs/review-workflow.md`.

## Canonical Documentation Set

The public documentation set for release review is:

- `README.md`
- `docs/design.md`
- `docs/cursor-navigation-semantics.md`
- `docs/restore-verification.md`
- `docs/review-workflow.md`
- `docs/release-signoff.md`
