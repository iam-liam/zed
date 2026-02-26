use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, WindowControlArea, actions, px,
};
use ui::utils::{TRAFFIC_LIGHT_PADDING, platform_title_bar_height};
use ui::{IconButton, IconName, IconSize, Tooltip, prelude::*};

use crate::TerminalView;

actions!(
    detached_terminal,
    [
        /// Reattaches the terminal back to the dock panel.
        ReattachTerminal,
    ]
);

pub enum DetachedTerminalEvent {
    ReattachRequested { terminal_view: Entity<TerminalView> },
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

    fn reattach(&mut self, _: &ReattachTerminal, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DetachedTerminalEvent::ReattachRequested {
            terminal_view: self.terminal_view.clone(),
        });
    }

    fn render_tab_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = self.terminal_view.read(cx).terminal().read(cx).title(false);
        let titlebar_color = cx.theme().colors().title_bar_background;

        h_flex()
            .id("detached-tab-bar")
            .window_control_area(WindowControlArea::Drag)
            .w_full()
            .h(platform_title_bar_height(window))
            .when(cfg!(target_os = "macos"), |this| {
                this.on_click(|event, window, _| {
                    if event.click_count() == 2 {
                        window.titlebar_double_click();
                    }
                })
            })
            .when_else(
                cfg!(target_os = "macos") && !window.is_fullscreen(),
                |this| this.pl(px(TRAFFIC_LIGHT_PADDING)),
                |this| this.pl_2(),
            )
            .bg(titlebar_color)
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .content_stretch()
            .child(
                div()
                    .id("detached-tab-bar-content")
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .overflow_x_hidden()
                    .w_full()
                    .pr_2()
                    .child(
                        h_flex()
                            .gap(DynamicSpacing::Base04.rems(cx))
                            .px(DynamicSpacing::Base04.rems(cx))
                            .child(
                                Icon::new(IconName::Terminal)
                                    .size(IconSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(Label::new(title).size(LabelSize::Small).color(Color::Muted)),
                    )
                    .child(
                        IconButton::new("reattach", IconName::Return)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Reattach to dock"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reattach(&ReattachTerminal, window, cx);
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("detached-terminal-window")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::reattach))
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(self.render_tab_bar(window, cx))
            .child(div().flex_1().min_h_0().child(self.terminal_view.clone()))
    }
}
