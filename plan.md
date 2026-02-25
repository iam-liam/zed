# Visual Tests for Thread Target Selector UI — Handoff Plan

## What This PR Does

PR #49141 (`AI-34/thread-target-selector-ui`) adds three new UI elements to the agent panel:

1. **Thread Target Selector** — A "Start Thread In…" dropdown in the agent panel toolbar, gated behind `AgentV2FeatureFlag`. Options are "Local Project" (default) and "New Worktree". The dropdown is inert — selecting "New Worktree" sets internal state but doesn't create a worktree yet.

2. **Worktree Creation Status Banner** — A status bar that appears below the toolbar. Shows a spinner with "Creating worktree…" or an error message with a warning icon. Currently only scaffolding (`WorktreeCreationStatus` has `#[allow(dead_code)]`).

3. **Worktree Branch Labels on History Rows** — When a thread was created in a worktree, its history row shows a git branch badge (branch icon + branch name like `zed/agent/a4Xiu`).

## What's Been Done

### Code review fixes (all committed, pushed, CI green)

Seven commits were made fixing issues found during code review:

- `c770822a6d` — Fix `is_via_collab` inconsistency in `render_thread_target_selector`
- `263110ad83` — Extract shared `format_relative_time` to eliminate duplication
- `430fbd00ea` — Add iteration guard to `set_selected_index`
- `02d5fde7f4` — Add missing `self.serialize(cx)` call in `set_thread_target`
- `f4044ee138` — Derive `Serialize`/`Deserialize` on `ThreadTarget` directly, eliminating `SerializedThreadTarget`
- `ae4867e459` — Validate thread target on deserialization (fall back to `LocalProject` if conditions no longer met)
- `3776b288c7` — Add integration test + TODO for `set_active` worktree creation guard

Plus two test fix commits for CI:
- `1b5961e0eb` — Initialize `GlobalFs` in the `set_active` test
- `d02f8a7e70` — Use `StubAgentServer` instead of triggering full agent infra in test

### Visual test scaffolding (written but not working yet)

Code has been written in three files, but the screenshots are all identical (the state mutations aren't taking effect). Here's what exists:

#### 1. `crates/zed/src/visual_test_runner.rs`

A new function `run_thread_target_selector_visual_tests` (starts around line 3124) that attempts to take 5 screenshots:

| Screenshot Name | What It Should Show |
|---|---|
| `thread_target_selector_default` | Agent panel toolbar with "Local Project" button + chevron |
| `thread_target_selector_new_worktree` | Same toolbar with "New Worktree" shown as the selected target |
| `worktree_creation_status_creating` | A banner below toolbar with spinner + "Creating worktree…" |
| `worktree_creation_status_error` | A banner with warning icon + error message |
| `history_with_worktree_branch` | History view with entries showing git branch badges |

The test is registered as "Test 11" in `run_visual_tests` (around line 548), gated behind `#[cfg(feature = "visual-tests")]`. It was moved to run before Test 9 because Test 9 (`run_tool_permissions_visual_tests`) panics with "Workspaces are root Windows" — that's a pre-existing bug, not something we introduced.

A `StubSessionList` struct (around line 3082) implements `AgentSessionList` to provide fake history entries with `worktree_branch` metadata for the history screenshot.

#### 2. `crates/agent_ui/src/agent_panel.rs`

Three test-only helper methods were added inside the `#[cfg(any(test, feature = "test-support"))]` impl block (around line 3799):

- `set_thread_target_for_tests(target, cx)` — Sets `self.thread_target` directly (bypasses git repo validation)
- `set_worktree_creation_status_for_tests(status, cx)` — Sets `self.worktree_creation_status`
- `open_history_for_tests(window, cx)` — Exposes the private `open_history()` method

#### 3. `crates/agent_ui/src/agent_ui.rs`

`ThreadTarget` and `WorktreeCreationStatus` were added to the public re-exports (line 52):
```rust
pub use crate::agent_panel::{
    AgentPanel, AgentPanelEvent, ConcreteAssistantPanelDelegate, ThreadTarget,
    WorktreeCreationStatus,
};
```

## The Problem: All Screenshots Are Identical

When you run the visual tests:
```
cargo run -p zed --features visual-tests --bin zed_visual_test_runner
```

All 5 screenshots are saved to `target/visual_tests/` but they're all byte-for-byte identical — they show the Zed welcome screen instead of the agent panel with the target selector. The state mutations (`set_thread_target_for_tests`, `set_worktree_creation_status_for_tests`, switching to history view) are not taking effect visually before the screenshots are captured.

## What Needs to Be Debugged

The root cause is likely one or more of these:

### 1. The agent panel isn't actually visible/focused in the window

The test opens the panel with `workspace.open_panel::<AgentPanel>(window, cx)` and then injects a stub thread with `panel.open_external_thread_with_server(...)`. But the screenshot shows the welcome screen, not the agent panel at all. Compare with the working `run_agent_thread_view_test` (around line 1957) — it follows a very similar pattern and DOES show the agent panel in its screenshots.

Things to check:
- Is the panel actually being added and opened? The `open_panel` call might not be focusing it in the right pane.
- Compare the exact sequence with `run_agent_thread_view_test` line by line — that one works and produces correct agent panel screenshots.

### 2. The `update_flags` call for `agent-v2` might need to happen earlier

The feature flag `AgentV2FeatureFlag` gates the thread target selector rendering (see `render_toolbar` in `agent_panel.rs` around line 2999: `.when(has_v2_flag, |this| { this.child(self.render_thread_target_selector(cx)) })`). The `cx.update_flags(true, vec!["agent-v2".to_string()])` call happens in the test function, but it might need to happen before the panel is loaded.

### 3. The `window.refresh()` + `run_until_parked()` cycle might not be sufficient

After mutating panel state, the test does:
```rust
cx.update_window(workspace_window.into(), |_, window, _cx| {
    window.refresh();
})?;
cx.run_until_parked();
```

But other working tests (e.g., `run_agent_thread_view_test`) also do this and it works fine for them. The issue is more likely #1 — the panel just isn't visible.

### 4. The history view switch might not be working

`open_history_for_tests` calls `self.open_history(window, cx)`. Verify this method actually switches the `active_view` to the history view and that the history entity has sessions loaded.

## How to Run the Visual Tests

```bash
# Build and run (debug mode — NEVER use --release)
cargo run -p zed --features visual-tests --bin zed_visual_test_runner

# Screenshots are saved to:
ls target/visual_tests/thread_target_selector_*.png
ls target/visual_tests/worktree_creation_status_*.png
ls target/visual_tests/history_with_worktree_branch.png
```

The test runner runs ALL visual tests sequentially. Our test is "Test 11". Tests 1-8 run first (all will say "FAILED" because there are no baseline images — that's expected and fine). Test 9 panics (pre-existing bug), so our test was moved to run before it.

To iterate faster while debugging, you can temporarily comment out Tests 1-8 in `run_visual_tests` so only Test 11 runs.

## Key Files

| File | What's in it |
|---|---|
| `crates/zed/src/visual_test_runner.rs` | The visual test runner binary. Our test: `run_thread_target_selector_visual_tests` (~line 3124). Registration: ~line 548. |
| `crates/agent_ui/src/agent_panel.rs` | The `AgentPanel` struct. Rendering: `render_toolbar` (~line 2960), `render_thread_target_selector` (~line 2507), `render_worktree_creation_status` (~line 3014). Test helpers: `set_thread_target_for_tests` etc (~line 3799). |
| `crates/agent_ui/src/agent_ui.rs` | Re-exports `ThreadTarget` and `WorktreeCreationStatus` (line 52). |
| `crates/agent_ui/src/acp/thread_history.rs` | History rendering including `render_worktree_branch_row` (~line 44) and `worktree_branch_from_meta` (~line 33). |

## Reference: A Working Visual Test

`run_agent_thread_view_test` (line 1957 in `visual_test_runner.rs`) is the closest working example. It:
1. Creates a project and workspace window (500x900)
2. Loads `AgentPanel` via `AgentPanel::load(...)`
3. Adds and opens the panel: `workspace.add_panel(panel.clone(), window, cx)` then `workspace.open_panel::<AgentPanel>(window, cx)`
4. Injects a `StubAgentServer` and opens a thread: `panel.open_external_thread_with_server(...)`
5. Sends a message, waits, refreshes, captures screenshot

The screenshots it produces (`agent_thread_with_image_collapsed.png`) DO show the agent panel correctly. Diffing our test against that one line-by-line should reveal what's different.

## What Success Looks Like

When working correctly, running the visual test runner should produce 5 distinct PNG files in `target/visual_tests/`:

1. **`thread_target_selector_default.png`** — Shows the agent panel with a toolbar containing a "Local Project ▾" button
2. **`thread_target_selector_new_worktree.png`** — Same but the button says "New Worktree ▾"
3. **`worktree_creation_status_creating.png`** — Shows a horizontal banner below the toolbar with a spinner animation and "Creating worktree…" text
4. **`worktree_creation_status_error.png`** — Shows a banner with a ⚠️ warning icon and "Failed to create worktree: branch already exists" in warning color
5. **`history_with_worktree_branch.png`** — Shows the thread history list with some entries having a small git branch icon + branch name badge below the thread title

Each file should be visually distinct (different MD5 hashes). The "FAILED — Baseline not found" messages are expected and fine since we don't have baselines yet.

## After Fixing the Visual Tests

Once the screenshots look correct:

1. Commit the visual test code changes
2. Push to the branch
3. CI should stay green (the visual test runner is a separate binary, not part of `cargo test`)
4. Don't worry about creating baseline images — the user said baselines aren't being used yet