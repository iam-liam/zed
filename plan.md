# Cleanup Plan for Thread Target Selector PR

This plan addresses all issues identified during code review of the `AI-34/thread-target-selector-ui` branch. Each task includes the problem, the fix, and the specific files to touch.

---

## Task 1: Fix `proceed_with_send` bypassing `/login`/`/logout` guard

**Problem:** In `AcpThreadView::send()`, the first-send interception fires *before* the `/login` and `/logout` command handling. When `AgentPanel` receives `FirstSendRequested` for a `LocalProject` target, it calls `proceed_with_send()`, which goes straight to `send_impl()` — completely skipping the `/login`/`/logout` check. A user typing `/login` as their first message gets it sent as a literal message to the agent.

**Fix:** Move the first-send interception in `send()` to *after* all guards, right before the final `send_impl` call. This way, by the time the event is emitted, every guard has already passed. `proceed_with_send` remains safe to call `send_impl` directly because the guarantees were established by `send()` before emission.

Concretely, in `crates/agent_ui/src/acp/thread_view/active_thread.rs`, restructure `send()` so the order is:

1. `is_loading_contents` guard (unchanged)
2. `is_editor_empty` / `can_fast_track_queue` / `has_queued` (unchanged)
3. `is_editor_empty` early return (unchanged)
4. `is_generating` → queue (unchanged)
5. `/login` / `/logout` handling (unchanged)
6. **First-send interception** (moved here from position 2) — check `self.thread.read(cx).entries().is_empty()` and emit `FirstSendRequested` with the already-trimmed `text`
7. `send_impl` call (unchanged)

The `FirstSendRequested` event text will now be trimmed (via the existing `text.trim()` earlier in the method), which is fine — `handle_worktree_creation_requested` uses it as message content that gets normalized downstream, and the non-`NewWorktree` path ignores the text entirely.

No changes needed to `proceed_with_send` or `handle_first_send_requested`.

**Files:**
- `crates/agent_ui/src/acp/thread_view/active_thread.rs`

---

## Task 2: Avoid fragile `.last()` lookup for newly-added workspace

**Problem:** In `handle_worktree_creation_requested`, after `Workspace::new_local` with `activate: false`, the code retrieves the new workspace via `multi_workspace.workspaces().last().cloned()`. While `add_workspace` does push to the end, this is a fragile assumption — any concurrent workspace addition between the `new_local` call and this lookup (across `.await` points) would return the wrong workspace.

**Fix:** `Workspace::new_local` already returns `(WindowHandle<MultiWorkspace>, Vec<...>)`. The second element is the opened items, but the `Entity<Workspace>` created inside the function is not returned. Change `new_local` to also return the `Entity<Workspace>` (e.g., as a third tuple element, or change the return type to a struct). Then use that directly in `handle_worktree_creation_requested` instead of the `.last()` lookup.

If changing the `new_local` return type is too invasive (it has many call sites), an alternative is to have `add_workspace` return the `Entity<Workspace>` or its index, and use `new_window_handle.update(cx, |mw, _, cx| { let idx = mw.add_workspace(ws, cx); mw.workspaces()[idx].clone() })` — but this is essentially the same fragility. The cleanest approach is returning it from `new_local`.

**Files:**
- `crates/workspace/src/workspace.rs` (modify `new_local` return type and the code that constructs the workspace inside it)
- `crates/agent_ui/src/agent_panel.rs` (update `handle_worktree_creation_requested` to use the returned workspace)
- All other `new_local` call sites (update to destructure the new return type)

---

## Task 3: Fix `capture_dock_state` zoom semantics divergence

**Problem:** The old `build_serialized_docks` checked zoom via `panel.is_zoomed(window, cx)` (a per-panel boolean field). The new `capture_dock_state` checks `self.zoomed_position == Some(DockPosition::Left)` (a workspace-level field). These diverge when a dock panel was zoomed, then focus moved to the center pane — the panel's internal `is_zoomed` flag stays `true`, but `zoomed_position` is cleared to `None`. The old code preserved "latent zoom" (closed dock remembers zoom for next open); the new code loses it.

**Fix:** Change `capture_dock_state` to query `panel.is_zoomed(window, cx)` like the old code, since `capture_dock_state` also takes `_window` already (just unused). This also requires changing the signature from `&self, _window: &Window, cx: &App` to actually use the `window` parameter.

```rust
let left_dock_zoom = left_dock
    .active_panel()
    .is_some_and(|panel| panel.is_zoomed(window, cx));
```

And the same for `right_dock_zoom` and `bottom_dock_zoom`.

**Files:**
- `crates/workspace/src/workspace.rs`

---

## Task 4: Avoid wasteful re-subscription on every `server_view` notification

**Problem:** In `set_active_view`, the `observe_in` callback on `server_view` re-creates `_thread_view_subscription` on *every* notification (message received, status change, etc.). `subscribe_to_active_thread_view` reads the active thread and subscribes to it. If the active thread hasn't changed, this tears down and re-creates an identical subscription for no reason.

**Fix:** Track the `EntityId` of the currently-subscribed thread view. In the `observe_in` callback, compare the current active thread's entity ID against the stored one — only re-subscribe if it actually changed. Add a field like `subscribed_thread_view_id: Option<EntityId>` to `AgentPanel`.

**Files:**
- `crates/agent_ui/src/agent_panel.rs`

---

## Task 5: Handle orphaned worktrees when panel is dropped mid-creation

**Problem:** If the async task in `handle_worktree_creation_requested` fails *after* worktree creation succeeds (e.g., the panel/workspace is dropped while waiting for `panels_task`), the `this.update_in(...)` calls return `Err`, which gets swallowed by `log_err()` at the bottom. The created git worktrees remain on disk with no workspace using them.

**Fix:** After the worktree creation loop succeeds, if any subsequent step fails (workspace gone, panel gone, etc.), perform the same rollback logic that currently only runs on creation failure. The simplest approach: restructure the async block so that `repos_and_paths` is available in a cleanup path. One way is to wrap the post-creation logic in a helper async block and, if it fails, run the rollback.

**Files:**
- `crates/agent_ui/src/agent_panel.rs`

---

## Task 6: Remove `plan.md` and `fix-plan.md` from the branch

**Problem:** `plan.md` is a working document that should not be merged.

*(`fix-plan.md` has already been removed in a prior commit.)*

---

## Task 7: Clean up `_external_thread` visibility

**Problem:** `_external_thread` was changed from private to `pub(crate)` to support the test helper `open_external_thread_with_server`. But the underscore prefix conventionally signals "private," which contradicts `pub(crate)` visibility.

**Fix:** Either:
- (a) Rename to `external_thread_impl` or similar (dropping the underscore) now that it's `pub(crate)`, or
- (b) Keep it private and have `open_external_thread_with_server` call the public `external_thread` method with appropriate parameters instead.

Option (b) is preferable if possible — check whether `open_external_thread_with_server` can use the existing public `external_thread` method.

**Files:**
- `crates/agent_ui/src/agent_panel.rs`

---

## Task 8: Guard against spurious `NewWorktree` fallback during deserialization

**Problem:** When restoring `ThreadTarget::NewWorktree` from serialized state, the validation checks `!project.repositories(cx).is_empty()`. But during startup, git repo scanning is async — repositories may not be populated yet, causing a spurious fallback to `LocalProject`.

**Fix:** This is a known limitation and the fallback is safe (user can re-select). Add a brief comment explaining *why* this can happen so future readers don't try to "fix" it by waiting for repos. No code change needed unless we want to defer validation until repos load, which adds complexity for minimal benefit.

**Files:**
- `crates/agent_ui/src/agent_panel.rs` (comment-only)

---

## Task 9: Fix `format_relative_time` inconsistent suffix and negative duration

**Problem:** Days render as `"3d"` (no suffix) while hours/minutes render as `"5h ago"` / `"3m ago"`. Also, future timestamps produce nonsensical output like `"-1h ago"`.

**Fix:** Either add `" ago"` to the days case for consistency (`"3d ago"`), or remove it from hours/minutes for compact display (`"5h"`, `"3m"`). Also handle negative durations gracefully — clamp to `"Just now"` if the duration is negative.

**Files:**
- `crates/agent_ui/src/acp/thread_history.rs`

---

## Task 10: Note unrelated changes for PR description

**Problem:** The PR includes changes to `update_command_palette_filter` (moving `filter.show_namespace("assistant")` out of the `if agent_enabled` block) and removes `CopyCode` test assertions. These are unrelated to the thread target selector.

**Fix:** Either split these into a separate commit with a clear commit message explaining the intent, or mention them in the PR description so reviewers understand they're intentional.

**Files:**
- `crates/agent_ui/src/agent_ui.rs` (already changed — just needs documentation/separate commit)

---

## Execution Order

1. **Task 1** — Fix `proceed_with_send` guard bypass (correctness, high priority)
2. **Task 3** — Fix zoom semantics in `capture_dock_state` (correctness, medium priority)
3. **Task 2** — Fix `.last()` workspace lookup (correctness, medium priority)
4. **Task 5** — Handle orphaned worktrees (correctness, medium priority)
5. **Task 4** — Cache thread view subscription (performance)
6. **Task 9** — Fix `format_relative_time` (minor UX)
7. **Task 7** — Clean up `_external_thread` naming (hygiene)
8. **Task 8** — Add deserialization comment (documentation)
9. **Task 10** — Document/split unrelated changes (hygiene)
10. **Task 6** — Delete `plan.md` and commit

---

## Task 6 (final step)

```sh
git rm plan.md
git commit -m "Remove plan.md working document"
```
