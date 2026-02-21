# Thread Target Selector — Cleanup Plan

Remaining actionable items from code review, in suggested order.

---

## 1. Refactor `handle_worktree_creation_requested` (issues #4, #5, #7)

The function is ~360 lines and contains duplicated rollback logic and an unusual
`'setup:` labeled block. Break it apart:

### 1a. Extract `rollback_worktrees` helper

The identical rollback loop appears twice — once after worktree creation failures
and once after post-creation setup failures. Extract into:

```rust
/// Rolls back successfully-created worktrees by removing them with force.
/// Logs errors but does not propagate them (best-effort cleanup).
async fn rollback_worktrees(
    repos_and_paths: &[(Entity<project::git_store::Repository>, PathBuf)],
    cx: &mut AsyncApp,
) {
    let mut rollback_receivers = Vec::new();
    for (repo, path) in repos_and_paths {
        if let Ok(receiver) = cx.update(|_, cx| {
            repo.update(cx, |repo, _cx| repo.remove_worktree(path.clone(), true))
        }) {
            rollback_receivers.push((path.clone(), receiver));
        }
    }
    for (path, receiver) in rollback_receivers {
        match receiver.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::error!("failed to rollback worktree at {}: {e}", path.display()),
            Err(e) => log::error!("failed to rollback worktree at {}: {e}", path.display()),
        }
    }
}
```

Both call sites become `rollback_worktrees(&repos_and_paths, cx).await`.

### 1b. Replace labeled block with async helper returning `Result`

The `'setup: { ... }` block with `break 'setup Some(err)` is emulating
try/catch. Replace with:

```rust
async fn setup_worktree_workspace(
    created_paths: Vec<PathBuf>,
    non_git_paths: Vec<PathBuf>,
    path_remapping: Vec<(PathBuf, PathBuf)>,
    open_file_paths: Vec<PathBuf>,
    dock_structure: DockStructure,
    text: String,
    workspace: WeakEntity<Workspace>,
    window_handle: Option<WindowHandle<MultiWorkspace>>,
    cx: &mut AsyncWindowContext,  // or AsyncApp, depending on what's available
) -> Result<()> {
    let mut all_paths = created_paths;
    let has_non_git = !non_git_paths.is_empty();
    all_paths.extend(non_git_paths.iter().cloned());

    let app_state = workspace
        .upgrade()
        .context("Workspace no longer available")?;
    let app_state = cx.update(|_, cx| app_state.read(cx).app_state().clone())?;

    // ... rest of the setup using `?` instead of break ...
}
```

The caller then becomes:

```rust
if let Err(err) = setup_worktree_workspace(...).await {
    rollback_worktrees(&repos_for_rollback, cx).await;
    this.update_in(cx, |this, _window, cx| {
        this.worktree_creation_status =
            Some(WorktreeCreationStatus::Error(format!("...{err}").into()));
        cx.notify();
    })?;
    return Ok(());
}
```

### 1c. Extract smaller helpers as natural seams emerge

Once 1a and 1b are done, the function will be shorter and more readable. If it's
still long, consider extracting:

- `generate_branch_name() -> String`
- `classify_worktrees(...) -> (Vec<Entity<Repository>>, Vec<PathBuf>)` — the loop
  that sorts visible worktrees into git-enabled repos and non-git paths

These are lower priority; do them only if the function still feels unwieldy after
1a+1b.

---

## 2. Fix silent message swallowing on first send (issue #1)

### Problem

`AcpThreadView::send()` unconditionally emits `FirstSendRequested` and returns
when `entries().is_empty()`. If the `AgentPanel` isn't subscribed (e.g., the
`update_thread_view_subscription` call hasn't fired yet), the send is silently
dropped — the user sees their text sitting in the editor with no response and no
error.

### Fix

The safest fix is to make `AcpThreadView::send()` not depend on an external
subscriber for the normal case. Only intercept when the panel has actually
configured a non-default thread target:

**Option A (preferred): Pass thread target info into the thread view.**

Have `AgentPanel` set a flag or callback on `AcpThreadView` indicating whether
first-send interception is needed. Then in `send()`:

```rust
if self.thread.read(cx).entries().is_empty() && self.needs_first_send_interception {
    cx.emit(AcpThreadViewEvent::FirstSendRequested {
        text: text.to_string(),
    });
    return;
}

self.send_impl(message_editor, window, cx)
```

When `needs_first_send_interception` is false (the default / `LocalProject`
case), `send()` falls through to `send_impl` directly. The subscription race
only matters for the `NewWorktree` path, where the panel is actively setting
things up and the subscription is guaranteed to exist.

`AgentPanel` would set `needs_first_send_interception = true` on the thread view
whenever `thread_target != ThreadTarget::LocalProject`, and reset it to false
after handling the event.

**Option B (simpler but less clean): Fall back after emitting.**

Add a one-frame deferred fallback that checks whether the event was handled:

```rust
if self.thread.read(cx).entries().is_empty() {
    self.first_send_pending = true;
    cx.emit(AcpThreadViewEvent::FirstSendRequested {
        text: text.to_string(),
    });
    // Safety net: if nobody clears first_send_pending, proceed normally
    let message_editor = self.message_editor.clone();
    cx.defer_in(window, move |this, window, cx| {
        if this.first_send_pending {
            this.first_send_pending = false;
            this.send_impl(message_editor, window, cx);
        }
    });
    return;
}
```

And in `proceed_with_send`, clear the flag:

```rust
pub fn proceed_with_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.first_send_pending = false;
    let message_editor = self.message_editor.clone();
    self.send_impl(message_editor, window, cx);
}
```

Option A is preferred because it avoids the interception overhead entirely for
the common case (LocalProject), making the code path simpler and the potential
failure surface smaller.

---

## 3. Clarify the `set_active` TODO comment (issue #3)

### Problem

The TODO says there's no automatic recovery when worktree creation fails, but
the code actually handles this correctly:

- The guard only blocks `WorktreeCreationStatus::Creating`.
- When creation fails, status is set to `Error(...)`, which does NOT match the
  guard.
- So the next `set_active(true)` call will proceed normally and create a thread.

The TODO is misleading and suggests a bug that doesn't exist.

### Fix

Replace the TODO with an accurate comment:

```rust
if active
    && matches!(self.active_view, ActiveView::Uninitialized)
    // Don't auto-create a thread while a worktree is being created —
    // the creation flow will set up the thread in the new workspace.
    // This only blocks `Creating`; an `Error` status won't prevent
    // thread creation on the next activation.
    && !matches!(
        self.worktree_creation_status,
        Some(WorktreeCreationStatus::Creating)
    )
{
```
