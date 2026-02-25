use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled,
    Window,
};
use ui::prelude::*;

use crate::TerminalView;

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
            .bg(cx.theme().colors().editor_background)
            .child(self.terminal_view.clone())
    }
}
