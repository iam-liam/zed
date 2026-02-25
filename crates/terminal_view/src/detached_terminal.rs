use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use ui::prelude::*;

use crate::TerminalView;

// Content insets for the detached window. Top clears the traffic lights
// (will be replaced by a tab bar in the future). Sides provide breathing room.
const CONTENT_PADDING_TOP: f32 = 38.0;
const CONTENT_PADDING_SIDE: f32 = 8.0;

pub struct DetachedTerminalWindow {
    terminal_view: Entity<TerminalView>,
    focus_handle: FocusHandle,
}

impl DetachedTerminalWindow {
    pub fn new(terminal_view: Entity<TerminalView>, cx: &mut Context<Self>) -> Self {
        cx.observe(&terminal_view, |_, _, cx| cx.notify()).detach();

        Self {
            terminal_view,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for DetachedTerminalWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DetachedTerminalWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("detached-terminal-window")
            .size_full()
            .pt(px(CONTENT_PADDING_TOP))
            .px(px(CONTENT_PADDING_SIDE))
            .bg(cx.theme().colors().editor_background)
            .child(self.terminal_view.clone())
    }
}
