# AI History System

Rust workspace for an AI assistant history, snapshot, and restore system with
branching, checkpointing, and audit trails for conversations, tool execution,
and workspace mutations.

## Overview

The workspace tracks:

- session, branch, and turn lifecycle
- file changes via patches and content-addressed blobs
- checkpoints and restore safety snapshots
- workspace state at arbitrary points in history
- fork, rollback, and redo flows

The design is split across 11 crates:

| Crate | Responsibility |
|-------|---------------|
| `ai-history-core` | Shared domain types, invariants, safety rules, and cross-crate traits |
| `ai-history-events` | Event schema, replay-facing types, and integrity metadata |
| `ai-history-fsm` | Interaction state machine transitions |
| `ai-history-store` | SQLite persistence, migrations, and storage orchestration |
| `ai-history-snapshot` | Workspace scanning, manifests, and blob writes |
| `ai-history-diff` | Diff parsing, inverse patches, and merge behavior |
| `ai-history-restore` | Restore planning, execution, verification, and conflict handling |
| `ai-history-context` | Context selection, summaries, and reproducibility data |
| `ai-history-ui-model` | UI-facing DTOs only |
| `ai-history-cli` | CLI command surface |
| `ai-history-testkit` | Shared fixtures, builders, fakes, and CI-oriented validation helpers |

## Public Docs

- `docs/design.md`: architecture, crate boundaries, lifecycle rules, and release sign-off requirements
- `docs/cursor-navigation-semantics.md`: cursor and navigation behavior
- `docs/release-signoff.md`: stable release checklist, validation commands, and sign-off criteria
- `docs/restore-verification.md`: restore verification expectations
- `docs/review-workflow.md`: review and evidence workflow

## CLI Quickstart

Initialize a session:

```bash
cargo run --release -p ai-history-cli -- init --title "my session"
```

Create a checkpoint:

```bash
cargo run --release -p ai-history-cli -- checkpoint --name "before refactor"
```

List checkpoints:

```bash
cargo run --release -p ai-history-cli -- list-checkpoints
```

Restore from a checkpoint:

```bash
cargo run --release -p ai-history-cli -- restore <checkpoint-id>
```

Fork from a checkpoint:

```bash
cargo run --release -p ai-history-cli -- fork --checkpoint-id <id> --name "experiment"
```

Inspect branches and diffs:

```bash
cargo run --release -p ai-history-cli -- branches
cargo run --release -p ai-history-cli -- diff <checkpoint-id>
cargo run --release -p ai-history-cli -- diff <checkpoint-id> --files-only
```

Verify checkpoint integrity:

```bash
cargo run --release -p ai-history-cli -- verify
cargo run --release -p ai-history-cli -- verify <checkpoint-id>
```

## Validation

Local and CI validation use the same commands:

```bash
cargo fmt --all --check
cargo check --release --workspace
cargo test --workspace --release
cargo doc --release --workspace --no-deps
```

The repository also includes `ai-history-testkit` evidence-gate coverage for
completion claims and crate wiring. GitHub Actions runs the same validation in
`.github/workflows/ci.yml`.

Install the repository-managed Git hooks with:

```bash
./scripts/install-git-hooks.sh
```

## Status

The CLI surface includes `init`, `checkpoint`, `list-checkpoints`, `restore`,
`fork`, `branches`, `diff`, and `verify`. `gc`, `doctor`, `export`, and
`import` are still tracked as follow-up work.

## License

This project is licensed under the 0BSD license. See `LICENSE`.

Third-party components retain their own terms.
