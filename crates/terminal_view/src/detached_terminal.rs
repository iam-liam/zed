use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, actions, px,
};
use ui::utils::TRAFFIC_LIGHT_PADDING;
use ui::{IconButton, IconName, IconSize, Tab, Tooltip, prelude::*};

use crate::TerminalView;

const CONTENT_PADDING_TOP: f32 = 4.0;

actions!(
    detached_terminal,
    [
        /// Reattaches the terminal back to the dock panel.
        ReattachTerminal,
        /// Closes the terminal and its detached window.
        CloseTerminal,
    ]
);

pub enum DetachedTerminalEvent {
    ReattachRequested { terminal_view: Entity<TerminalView> },
    Closed { terminal_view: Entity<TerminalView> },
}

impl EventEmitter<DetachedTerminalEvent> for DetachedTerminalWindow {}

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

    pub fn terminal_view(&self) -> &Entity<TerminalView> {
        &self.terminal_view
    }

    fn reattach(&mut self, _: &ReattachTerminal, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DetachedTerminalEvent::ReattachRequested {
            terminal_view: self.terminal_view.clone(),
        });
    }

    fn close_terminal(&mut self, _: &CloseTerminal, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DetachedTerminalEvent::Closed {
            terminal_view: self.terminal_view.clone(),
        });
        window.remove_window();
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .terminal_view
            .read(cx)
            .terminal()
            .read(cx)
            .breadcrumb_text
            .clone();

        h_flex()
            .id("detached-tab-bar")
            .h(Tab::container_height(cx))
            .w_full()
            .px(DynamicSpacing::Base06.rems(cx))
            .items_center()
            .justify_between()
            .bg(cx.theme().colors().tab_bar_background)
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .gap(DynamicSpacing::Base04.rems(cx))
                    .ml(px(TRAFFIC_LIGHT_PADDING))
                    .child(Label::new(title).size(LabelSize::Small).color(Color::Muted)),
            )
            .child(
                h_flex()
                    .gap(DynamicSpacing::Base04.rems(cx))
                    .child(
                        IconButton::new("reattach", IconName::Return)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Reattach to dock"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reattach(&ReattachTerminal, window, cx);
                            })),
                    )
                    .child(
                        IconButton::new("close-terminal", IconName::Close)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Close terminal"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_terminal(&CloseTerminal, window, cx);
                            })),
                    ),
            )
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
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(self.render_tab_bar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .pt(px(CONTENT_PADDING_TOP))
                    .px(DynamicSpacing::Base06.rems(cx))
                    .child(self.terminal_view.clone()),
            )
    }
}
