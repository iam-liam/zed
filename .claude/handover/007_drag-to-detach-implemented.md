---
timestamp: 2026-02-26T13:15:00+0000
session_id: checkpoint_7
phase: Phase 3
status: completed
categorization_hint: PROGRESS
significance: incremental
tags: [detachable-panels, drag-to-detach, terminal-view, gpui, multi-window]
author: Liam
---

# Phase 3: Drag-to-Detach Terminal Tabs

## Summary

Implemented drag-to-detach for terminal tabs. Dragging a terminal tab outside the Zed window boundary detaches it into a standalone window. Drops inside the window use normal Zed tab move/split behavior.

## What Was Done

### Drag-to-Detach Implementation

Added to `crates/terminal_view/src/terminal_panel.rs`:

- `pending_drag_terminal: Option<Entity<TerminalView>>` field on `TerminalPanel`
- `on_drag_move::<DraggedTab>` handler tracks which terminal is being dragged
- `on_mouse_up_out(MouseButton::Left)` detects mouse release outside the panel
- Viewport bounds check (`window.viewport_size()`) distinguishes inside vs outside window drops
- Dispatches `DetachTerminal` action through workspace to avoid re-entrancy panic
- `detach_terminal` action handler checks `pending_drag_terminal` first, falls back to active terminal

### Key Technical Decisions

**Re-entrancy prevention:** Direct `workspace.update()` from within TerminalPanel's render cycle causes "cannot read TerminalPanel while being updated" panic. Solution: dispatch `DetachTerminal` action via `window.dispatch_action()` which routes through the workspace's action dispatch system.

**Specific terminal targeting:** `pending_drag_terminal` stores the exact `Entity<TerminalView>` being dragged. The action handler checks this before falling back to the active terminal, ensuring the correct tab is detached even if it's not the active one.

**Inside vs outside detection:** `event.position` from `MouseUpEvent` is in window-local coordinates. Compare against `window.viewport_size()` — if position is negative or beyond viewport, mouse is outside the window.

### Refactoring

Extracted `detach_terminal_view_in_workspace()` from `detach_terminal()` to allow both entry points (action handler, drag handler) to share the core detach/window-open/subscribe logic. The workspace action handler now finds the source pane dynamically rather than assuming `active_pane`.

## Branch State

- `detachable-panels-v2` — working branch with Phase 3 commit on top of PR #50197
- PR #50197 — Phase 1+2, submitted, awaiting review

## Files Modified

| File | Change |
|------|--------|
| `crates/terminal_view/src/terminal_panel.rs` | +82/-13 lines: drag tracking, viewport bounds check, refactored detach logic |

## Future Polish

- Ghost tab popup window at cursor position when dragging outside window bounds
- Visual indicator at window edge showing "drop zone" for detach

## Next Steps

1. Agent panel detach (Phase 4) — same entity-sharing pattern
2. Ghost tab window for drag-to-detach visual feedback
3. Monitor PR #50197 for reviewer feedback
