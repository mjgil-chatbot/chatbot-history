# Review Workflow: Evidence-Backed Audit States

This document describes how the evidence scanner derives audit states for
release-checklist completion claims.

## Audit States

The evidence scanner classifies each scanned checklist row into one of five
states:

| State | Meaning | Repository Evidence |
|-------|---------|---------------------|
| `Complete` | All required evidence found in repository | Source files exist AND tests exist AND docs exist |
| `Partial` | Some evidence found, but incomplete | Source OR tests OR docs missing |
| `Placeholder` | Source files exist but implementation is stub/TODO | Source found + placeholder keywords detected |
| `Missing` | No source files found for claimed feature | No source file links in implementation column |
| `Overclaimed` | Task claims Complete but provides no evidence | No source links AND no supporting evidence |

## How Status Is Derived

1. **Source file links** are extracted from the implementation column (looks for `.rs`, `.toml`, `.md` patterns in backticks)
2. **Placeholder keywords** are detected: `TODO`, `FIXME`, `not implemented`, `placeholder`, `stub`, `coming soon`
3. **Tests and docs** are checked via text keywords in their respective columns, or by verifying linked files exist

## Downgrade Rules

A checklist row marked "Complete" in the release sign-off document will be
downgraded when:

- **No source links found** → `Missing` or `Overclaimed`
- **Source links point to nonexistent files** → `Missing`
- **Source exists but contains placeholder keywords** → `Placeholder`
- **Source exists but tests missing** → `Partial`
- **Source exists but documentation missing** → `Partial`

## Interpreting Scan Results

When running the evidence scanner, each `EvidenceResult` contains:

```rust
pub struct EvidenceResult {
    pub task_number: u32,
    pub status: EvidenceStatus,        // The derived audit state
    pub source_files_found: Vec<String>,
    pub source_files_missing: Vec<String>,
    pub tests_found: bool,
    pub docs_found: bool,
    pub errors: Vec<String>,           // Why status was downgraded
}
```

The `errors` field explains the reasoning behind any non-Complete status.

## CI Integration

The evidence gate can be invoked through the release-gate path to validate
release checklist claims:

```rust
use ai_history_testkit::evidence_gate::{EvidenceScanner, EvidenceConfig};

// Configure scanner with workspace root
let config = EvidenceConfig {
    workspace_root: "/path/to/repo".to_string(),
    tasks_doc_path: "docs/release-signoff.md".to_string(),
    // ...
};

let scanner = EvidenceScanner::new(config);
let results = scanner.scan_all_complete_tasks(tasks_content);

// Fail CI if any checklist rows have non-Complete status
for result in results {
    if result.status != EvidenceStatus::Complete {
        eprintln!("Task {} has status {:?}: {:?}", result.task_number, result.status, result.errors);
    }
}
```

## Review Process

1. **Run scanner** against the release sign-off checklist when it contains completion rows
2. **Inspect results** where status != Complete
3. **Verify errors** explain the actual gap (missing files, placeholder text, etc.)
4. **Decide action**:
   - Add missing source/test/doc evidence to the repository
   - Update checklist status to reflect actual state
   - If evidence was incorrectly parsed, improve scanner patterns

## Test Coverage

Audit tests verify the following scenarios:

- `missing-document`: Complete row with no source links → Missing
- `source-only-partial`: Source found but no tests/docs → Partial
- `placeholder-implementation`: Source exists but has TODO/FIXME → Placeholder
- `overclaimed-complete`: Complete claim with no evidence → Overclaimed
- `real-complete`: All evidence exists and passes → Complete

These tests are in `crates/ai-history-testkit/src/audit_tests.rs`.
