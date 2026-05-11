# Cursor Navigation Semantics

## Overview

The ai-history system maintains two distinct concepts for tracking position within conversation history: **branch head** and **view cursor**. Understanding the distinction is critical for correct behavior of navigation commands.

## Branch Head vs View Cursor

### Branch Head

The branch head is the tip of a branch's conversation timeline — the most recent finalized turn on that branch. It represents the authoritative state of the conversation branch and is persisted as part of branch metadata.

- Stored in the `branches` table as `head_turn_id`
- Changes only when new turns are created or when a branch is explicitly reset
- Used by forking, continuing, and restore operations as the reference point
- Immutable during pure navigation operations

### View Cursor

The view cursor is a transient positional marker that tracks where the user is viewing in the conversation timeline. It is stored separately from branch head and can be moved without affecting branch state.

- Stored in the `cursor_position` table
- Does NOT persist across sessions (transient)
- Independent of branch head — a branch can have head at turn T5 while cursor is at T2
- Navigation commands (`navigate-back`, `navigate-forward`) move only the cursor

### Key Invariant

```
cursor.position() may be AtHead or AtTurn(some_turn_id)
branch.head_turn_id is ALWAYS the actual tip of the branch
```

Navigation never mutates `branch.head_turn_id`.

## Navigation Commands

### Navigate Back

Moves the cursor to a parent turn in the conversation history.

```
Current: cursor at T5 (branch head is T5)
Command: navigate-back
Result: cursor moves to T4 (or earlier parent)
```

**Guarantees:**
- Branch head unchanged
- No checkpoints created or deleted
- No events emitted
- No workspace files modified

### Navigate Forward

When at a turn with multiple children (multiple possible forward paths), the system detects **ambiguous forward history** and reports it rather than choosing arbitrarily.

```
Current: cursor at T3 which has children [T4_main, T4_fork]
Command: navigate-forward
Result: reports ambiguity with both targets listed
        no state change occurs
```

**Guarantees:**
- Branch head unchanged
- No checkpoints created or deleted
- No events emitted
- No workspace files modified

### Single Forward Target

When a turn has exactly one child, navigation proceeds automatically:

```
Current: cursor at T3 which has single child T4
Command: navigate-forward
Result: cursor moves to T4
```

## Ambiguous Forward History

### Detection

Forward navigation queries `get_turn_forward_children(turn_id)` which returns all direct child turns. When more than one child exists, the history is ambiguous.

### Reporting

The system returns an `AmbiguousForwardTargets` object containing:
- All possible target turns
- Branch ID for each target (for cross-branch scenarios)
- Whether each target represents an "abandoned future" (fork branch that was not continued)

### User Choice Required

Ambiguous forward navigation requires explicit user choice. The system will not auto-select when multiple valid paths exist.

## Preservation Guarantees

Navigation operations are **read-only with respect to history state**:

| Aspect | During Navigation |
|--------|-----------------|
| Branch head | Unchanged |
| Checkpoints | Unchanged |
| Events | Unchanged |
| Workspace files | Unchanged |
| Turn data | Unchanged |

Only the transient cursor position changes.

## Fork and Continue Behavior

### Fork Current

Creates a new branch from the branch head. The original branch's head is preserved as the `abandoned_head_turn_id` in the new branch metadata.

```
Original branch: main (head T5)
Command: fork-current feature-x
Result: new branch "feature-x" with parent_turn_id = T5
        main.head_turn_id unchanged (still T5)
```

### Fork From Cursor

Creates a new branch from the cursor position rather than branch head. Useful for exploring alternative paths from an earlier point.

### Continue From Cursor

Creates a new turn on a new branch, continuing from the cursor position on the original branch.

## Workspace Restoration Policy

When forking or continuing:

- **Dirty workspace**: The system does NOT automatically restore files. The user is responsible for managing workspace state.
- **Explicit restore**: Only `fork-from-cursor --restore-workspace` attempts workspace restoration to the cursor turn's snapshot.

This is an explicit policy decision to avoid unexpected file mutations during exploratory branching.

## Branch-Fork Semantics

### fork-current

Creates a new branch from the **active branch head** without modifying the original branch.

**Behavior:**
- Original branch head remains unchanged (still points to the same turn)
- New branch is created as a child of the original branch
- The original branch head becomes an **abandoned future** on the new branch
- No workspace files are restored or modified
- New branch has no head turn until a turn is explicitly added

**Policy: Dirty Workspace**
The system does NOT automatically restore the workspace to the fork point. The user's files remain as-is. This is intentional — exploratory branching should not mutate workspace state unexpectedly.

**Abandoned Futures:**
When `fork-current` is called, the original branch's head turn is recorded as an abandoned future in the new branch's metadata. This preserves the lineage of "what could have been continued" and enables potential future operations like redo or cross-branch navigation.

### fork-from-cursor

Creates a new branch from the **cursor position** (which may be behind the branch head).

**Behavior:**
- Uses the cursor's turn ID as the fork point, not the branch head
- Original branch is unchanged
- The turn at cursor position becomes an abandoned future on the new branch (when cursor is behind head)
- When cursor is at branch head, no abandoned future is recorded (the fork starts at the same point)
- Only `fork-from-cursor --restore-workspace` attempts to restore files
- Default (`restore_workspace=false`) leaves the workspace dirty

**Policy: No Auto-Restore**
By default, forking from cursor does not restore workspace files. The user must explicitly request restore via the `--restore-workspace` flag.

### continue-from-cursor

Creates a new branch and appends a new turn, continuing the conversation from the cursor position.

**Behavior:**
- Creates a new branch forked from the original at cursor position
- Adds a new turn (created from user's prompt) to the new branch
- Original branch is unaffected; its head turn becomes an abandoned future
- No automatic workspace restoration

**Policy: Dirty Workspace**
Continuing from cursor leaves the current workspace dirty. No automatic file restoration is performed.

## Ancestry and Branch Relationships

### Parent-Child Branch Relationships

When a branch is created via fork or continue:
- The new branch records its parent branch ID
- The parent branch is unchanged
- The new branch starts with no head turn (until turns are added)

### Abandoned Futures

An **abandoned future** is a turn that existed on the original branch but was bypassed when a fork or continue created a new branch path. These are preserved in branch metadata.

When `fork-current feature` is called at T5:
- `feature` branch records `abandoned_futures = [T5]`
- The original `main` branch is unchanged

This allows:
- Clear lineage tracking (which turns were superseded)
- Potential redo operations targeting the abandoned turn
- Navigation that acknowledges the alternative history

### Dirty Workspace Policy

The dirty workspace policy means:

1. **fork-current**: No workspace restore. User's current files are preserved as-is on both branches (the original branch continues from its current state; the fork starts fresh without files being placed).

2. **fork-from-cursor**: No auto-restore by default. User must explicitly pass `--restore-workspace` to trigger snapshot restoration.

3. **continue-from-cursor**: No workspace restore. The new turn is created on the new branch without affecting current workspace files.

**Rationale:**
- Exploratory branching should not cause unexpected file mutations
- Users may have unsaved work they do not want overwritten
- Making restore explicit gives users control over when files change

## Continue-from-History Policy

When a user submits a new prompt while viewing an earlier turn (not the branch head):

1. A new branch is created from the viewed turn
2. The new prompt is added as a new turn on the new branch
3. The original branch's head becomes an abandoned future on the new branch
4. No workspace files are restored

The user explicitly controls when files change by:
- Using `fork-from-cursor --restore-workspace` to get files from that point
- Manually navigating and triggering restore before continuing

## Turn Synthesis Policy (Explicit Non-Goal)

The fork/continue/edit-and-rerun operations **do not synthesize new turn IDs**. This is an explicit design decision to prevent creating unattached turns that lack proper ancestry and event bindings.

### Why Turn Synthesis Is Forbidden

1. **Ancestry requirement**: Every turn must have a valid `parent_turn_id` that points to an existing turn in the turn chain. Fabricated turn IDs would break this invariant.

2. **Event binding**: Turns are created through the store/events system and must have corresponding events (UserPromptSubmitted, AssistantDone, etc.). Returning a placeholder turn ID would leave the turn orphaned from the event log.

3. **Verification requirement**: New turns must be verifiable against the event store. Synthesized IDs cannot be verified.

### Supported Continue/Edit Flow

When `continue-from-cursor` or `edit-and-rerun` is called:

1. The **branch fork succeeds** - a new branch is created with correct parentage
2. The **turn creation returns an error** - indicating that the caller must create the turn through proper store/events channels
3. The caller then uses the store API to create the real turn with correct parent_turn_id, actor, content, etc.

### Error Cases

| Operation | Condition | Error |
|-----------|-----------|-------|
| `continue-from-cursor` | Cannot validate cursor position | `TurnNotFound` |
| `continue-from-cursor` | No head turn on branch | `NoHeadTurn` |
| `edit-and-rerun` | Turn is a root turn (no parent) | `NoParentTurn` |
| `edit-and-rerun` | Cannot validate parent turn | `NoParentTurn` |
| `fork-from-cursor` | Branch not found | `NoActiveBranch` |

### Regression Test Coverage

These error cases are verified by unit tests in `fork_conversation.rs`:
- `test_continue_from_cursor_returns_error_not_fabricated_turn` - verifies error not fabricated ID
- `test_edit_and_rerun_rejects_root_turn` - verifies root turns are rejected
- `test_edit_and_rerun_rejects_when_parent_lookup_fails` - verifies missing parent is rejected
- `test_continue_from_cursor_no_head_fails` - verifies no-head branch is rejected
- `test_fork_from_cursor_no_active_branch_fails` - verifies invalid branch is rejected
