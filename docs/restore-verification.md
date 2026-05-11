# Restore Verification Contract

## Overview

After a restore operation completes, the system runs verification to confirm the workspace state matches the restore plan. The verifier is strict by default — any deviation from the expected state causes verification to fail.

## What is Verified

For each operation in the restore plan, the verifier checks:

### CreateFile / UpdateFile Operations

- The file exists at the expected path
- The file content hash matches the expected hash from the plan
- The file is readable (no IO errors during verification)

### DeleteFile Operations

- The file does NOT exist (it should have been deleted)

### CreateDir Operations

- The directory exists at the expected path
- The path exists as a directory (not as a file)

### Unexpected Files

The verifier also scans the entire workspace for files not mentioned in the plan. Any such file causes verification to fail with an `UnexpectedFile` error. This ensures that:

1. No extra files are left behind from a previous state
2. The workspace exactly matches what the plan expected

## Verification Error Categories

The verifier returns detailed error types rather than a generic boolean:

| Error Type | Meaning |
|------------|---------|
| `UnexpectedFile { path }` | A file exists in the workspace that is not part of the restore plan |
| `MissingFile { path }` | A file the plan expected to exist does not exist |
| `ContentMismatch { path, expected, actual }` | The file exists but content hash doesn't match |
| `MissingDirectory { path }` | A directory the plan expected to exist does not exist |
| `DirectoryShouldNotExist { path }` | A path exists as a file but the plan expected a directory |
| `FileShouldNotExist { path }` | A file that should have been deleted still exists |
| `IoError { path, message }` | IO error (permission denied, read failure, etc.) accessing the file |

## Unexpected File Handling

Unexpected files are treated as verification failures because:

1. **Correctness**: A restore plan defines an exact target state. Extra files suggest the restore was incomplete or the workspace was not properly isolated.

2. **Safety**: In branching scenarios, leaving extra files could cause confusion about which branch's state is active.

3. **Repeatability**: An exact workspace state is required for reliable checkpoint/replay.

If your use case intentionally allows extra files, you may need to adjust the plan to include them or use a less strict verification mode.

## IO and Permission Failures

IO errors during verification are surfaced distinctly as `IoError` rather than being confused with content mismatches or missing files. This allows callers to distinguish:

- Permission denied errors (may need user action to fix)
- Read errors (disk issues, file vanished mid-verify)
- Permission-related issues that block proper verification

## Verification API

```rust
// Primary API — returns detailed errors
pub fn verify_restore(
    plan: &RestorePlan,
    workspace_root: &Path,
) -> CoreResult<(bool, Vec<RestoreVerificationError>)>;

// Simple backward-compatible wrapper
pub fn verify_restore_simple(
    plan: &RestorePlan,
    workspace_root: &Path,
) -> CoreResult<bool>;
```

## Example Verification Results

### Success Case

```
verify_restore(plan, workspace) -> Ok((true, []))
```

### Multiple Errors

```
verify_restore(plan, workspace) -> Ok((false, [
    UnexpectedFile { path: "extra.log" },
    ContentMismatch { path: "config.txt", expected: "abc123", actual: "def456" },
    MissingFile { path: "missing.txt" },
]))
```

## Relationship to Safety Checkpoints

The verification step runs after a safety checkpoint is captured but before the restore operations are committed. If verification fails:

1. The transaction is rolled back
2. The workspace is restored to its pre-restore state
3. An error is returned describing what went wrong

This ensures failed restores never leave the workspace in a partially modified state.

## Failed-Restore Recovery Sequence

When a restore operation fails mid-transaction, the system uses `rollback_restore()` to recover the workspace to its pre-restore state. This function implements the safety-checkpoint replay mechanism:

### Recovery Steps

1. **Load Safety Checkpoint**: The system loads the safety checkpoint from the store using its ID. This checkpoint was captured before the restore began and contains a reference to the safety manifest.

2. **Load Safety Manifest**: The safety manifest is retrieved from the store. This manifest describes the exact workspace state at the time the safety checkpoint was created.

3. **Scan Current Workspace**: The current workspace is scanned to produce a current manifest describing what actually exists now (which may differ from the safety checkpoint if the restore made partial changes).

4. **Build Restore Plan**: Using the current manifest and safety manifest, a restore plan is built that will transform the current workspace back to the safety checkpoint state.

5. **Execute Restore Plan**: The plan is executed transactionally. If any operation fails, the transaction is rolled back and an error is returned.

### Limits of the Guarantee

The rollback guarantee has the following limits:

1. **Manifest must exist**: If the safety manifest cannot be loaded (corrupted, deleted), recovery fails with a "Safety manifest not found" error.

2. **Blob availability**: If the blobs referenced by the safety manifest are not available in the blob store, the restore cannot proceed. The caller must ensure the blob store contains all necessary content.

3. **Workspace mutation scope**: Rollback only restores files to their checkpoint state. It does not:
   - Restore deleted branches or turns
   - Recover in-progress edits that were never checkpointed
   - Undo external changes made to files outside the restore plan

4. **Partial restore window**: Between the start of a failed restore and the completion of rollback, the workspace may be in a mixed state. Applications should not assume the workspace is either at the pre-restore or post-restore state during this window.

5. **No atomic multi-checkpoint restore**: If a restore involves multiple checkpoints, rollback will only recover to the single safety checkpoint specified, not to any broader transactional boundary.

### Error Handling

| Error Condition | Result |
|-----------------|--------|
| Safety checkpoint not found | `CoreError::Storage("Safety checkpoint {id} not found")` |
| Safety manifest not found | `CoreError::Storage("Safety manifest {id} not found")` |
| Blob not found during restore | Restore fails, rollback attempted |
| Workspace IO error during restore | Restore fails, rollback attempted |
| Permission denied | Returned as `CoreError::Io` with details |

### Rollback API

```rust
pub fn rollback_restore<B: BlobStore + ?Sized>(
    session_id: SessionId,
    safety_checkpoint_id: CheckpointId,
    workspace_root: &Path,
    store: &SqliteStore,
    blob_store: &B,
) -> CoreResult<()>
```

The function returns `Ok(())` on successful recovery, or an error if the checkpoint/manifest cannot be loaded or the restore cannot complete. On error, the workspace may be in an inconsistent state — callers should treat this as a fatal error requiring manual intervention.
