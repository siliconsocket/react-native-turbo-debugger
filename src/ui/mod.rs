mod console;
mod investigation;
mod theme;

use crate::{app::Capture, bridge, mock, session::*};
use gpui::{prelude::*, *};
use gpui_component::input::InputState;
use std::collections::HashSet;
use theme::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Panel {
    Network,
    Preview,
    Logs { native: bool },
    Timeline,
    History,
    Inspector,
    Details,
}

pub struct Debugger {
    maximized: Option<Panel>,
    capture: Capture,
    live: bool,
    bridge_error: bool,
    state_view: bool,
    tab: usize,
    native: bool,
    drawer: bool,
    filters: [Entity<InputState>; 2],
    levels: [usize; 2],
    expanded: HashSet<String>,
    focus: FocusHandle,
}

impl Debugger {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let scenario = std::env::var("RNTD_SCENARIO").unwrap_or_default();
        let live = std::env::var("RNTD_LIVE").as_deref() == Ok("1");
        let mut bridge_error = false;
        let capture = if live {
            match bridge::listen() {
                Ok(receiver) => {
                    cx.spawn(async move |this, cx| {
                        loop {
                            Timer::after(std::time::Duration::from_millis(40)).await;
                            let updates: Vec<_> = receiver.try_iter().take(256).collect();
                            if updates.is_empty() {
                                continue;
                            }
                            if this
                                .update(cx, |this, cx| {
                                    for update in updates {
                                        this.capture.ingest(update);
                                    }
                                    cx.notify();
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    })
                    .detach();
                }
                Err(error) => {
                    eprintln!(
                        "Network bridge could not listen on 127.0.0.1:{}: {error}",
                        bridge::PORT
                    );
                    bridge_error = true;
                }
            }
            Capture::live()
        } else {
            let mut capture = Capture::new(mock::scenario(&scenario));
            capture.connected = scenario != "disconnected";
            capture
        };
        let filters = std::array::from_fn(|_| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter text…"));
            cx.subscribe(&input, |_, _, _: &gpui_component::input::InputEvent, cx| {
                cx.notify()
            })
            .detach();
            input
        });
        let focus = cx.focus_handle();
        focus.focus(window);
        Self {
            maximized: None,
            capture,
            live,
            bridge_error,
            state_view: std::env::var("RNTD_VIEW").as_deref() == Ok("state"),
            tab: 0,
            native: false,
            drawer: false,
            filters,
            levels: [0; 2],
            expanded: HashSet::new(),
            focus,
        }
    }

    fn navigate(&mut self, state: bool) {
        self.maximized = None;
        self.state_view = state;
    }

    fn panel_control(&self, panel: Panel, cx: &mut Context<Self>) -> Stateful<Div> {
        let active = self.maximized == Some(panel);
        self.button(
            format!("maximize-{panel:?}"),
            if active { "Restore" } else { "Expand" },
            active,
            cx,
            move |s, _| s.maximized = if active { None } else { Some(panel) },
        )
    }

    fn button(
        &self,
        id: impl Into<String>,
        label: impl Into<String>,
        active: bool,
        cx: &mut Context<Self>,
        f: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        row()
            .id(SharedString::from(id.into()))
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .h(px(27.))
            .px_2()
            .rounded(px(3.))
            .border_1()
            .border_color(color(if active { "accent" } else { "divider" }))
            .bg(if active {
                rgb(0x392f23)
            } else {
                color("surface")
            })
            .text_color(color(if active { "accent" } else { "text" }))
            .hover(|s| s.bg(rgb(0x383c3e)))
            .focus(|s| s.border_color(color("key")))
            .child(label.into())
            .on_click(cx.listener(move |this, _, window, cx| {
                let previous_panel = this.maximized;
                f(this, cx);
                if this.maximized != previous_panel {
                    this.focus.focus(window);
                }
                cx.notify()
            }))
    }
    fn move_selection(&mut self, delta: isize) {
        let ids: Vec<_> = self
            .capture
            .visible()
            .filter(|e| matches!(e.data, Data::Zustand { .. }))
            .map(|e| e.id.clone())
            .collect();
        if ids.is_empty() {
            return;
        }
        let index = ids
            .iter()
            .position(|id| Some(id) == self.capture.action.as_ref())
            .unwrap_or(0);
        self.capture.action =
            Some(ids[index.saturating_add_signed(delta).min(ids.len() - 1)].clone());
    }
}
impl Render for Debugger {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width);
        let rail = width < 1280.;
        let title = if self.state_view {
            "STATE INVESTIGATION"
        } else {
            "SESSION CONSOLE"
        };
        let session = &self.capture.session;
        let header = row()
            .h(px(48.))
            .flex_none()
            .pl(px(88.))
            .pr_4()
            .gap_3()
            .bg(gradient())
            .window_control_area(WindowControlArea::Drag)
            .child(heading("React Native Turbo Debugger"))
            .child(div().flex_1())
            .child(
                row()
                    .size(px(32.))
                    .rounded(px(5.))
                    .justify_center()
                    .bg(color("accent"))
                    .text_color(color("canvas"))
                    .child(match &session.app_icon {
                        Some(path) => img(std::path::PathBuf::from(path))
                            .size_full()
                            .into_any_element(),
                        None => div()
                            .child(
                                session
                                    .app
                                    .split_whitespace()
                                    .filter_map(|part| part.chars().next())
                                    .take(2)
                                    .collect::<String>(),
                            )
                            .into_any_element(),
                    }),
            )
            .child(session.app.clone())
            .child(muted("│"))
            .child(session.device.clone())
            .child(muted("│"))
            .child(mono(
                if self.bridge_error {
                    "○ Port unavailable"
                } else if self.capture.connected {
                    "● Live"
                } else {
                    "○ Disconnected"
                },
                if self.capture.connected {
                    "string"
                } else {
                    "error"
                },
            ));
        let sidebar = col()
            .w(px(if rail { 52. } else { 168. }))
            .flex_none()
            .bg(rgb(0x202326))
            .border_r_1()
            .border_color(color("divider"))
            .p_2()
            .gap_2()
            .child(self.button(
                "console",
                if rail { "›_" } else { "›_    Console" },
                !self.state_view,
                cx,
                |s, _| s.navigate(false),
            ))
            .child(self.button(
                "state",
                if rail { "◇" } else { "◇    State" },
                self.state_view,
                cx,
                |s, _| s.navigate(true),
            ));
        let toolbar = row()
            .h(px(44.))
            .flex_none()
            .gap_2()
            .child(heading(title))
            .child(muted(if self.state_view {
                "Zustand store"
            } else {
                "React Native"
            }))
            .child(div().flex_1())
            .when(self.state_view && rail && self.maximized.is_none(), |d| {
                d.child(self.button("details", "Details", self.drawer, cx, |s, _| {
                    s.drawer = !s.drawer
                }))
            })
            .when(!self.capture.stopped, |d| {
                d.child(self.button(
                    "pause",
                    if self.capture.pinned.is_some() {
                        "▶ Resume"
                    } else {
                        "Ⅱ Pause"
                    },
                    true,
                    cx,
                    |s, _| {
                        if s.capture.pinned.is_some() {
                            s.capture.pinned = None
                        } else {
                            s.capture.pin(s.capture.newest())
                        }
                    },
                ))
            })
            .child(self.button(
                "stop",
                if self.capture.stopped {
                    "■ Stopped"
                } else {
                    "■ Stop"
                },
                false,
                cx,
                |s, _| s.capture.stopped = true,
            ))
            .when(
                !self.live && !self.capture.stopped && self.capture.connected,
                |d| {
                    d.child(self.button("append", "+ Demo event", false, cx, |s, _| {
                        s.capture.append()
                    }))
                },
            );
        let content = if self.state_view {
            self.investigation(width, window, cx)
        } else {
            self.console(width, window, cx)
        };
        col()
            .id("debugger")
            .track_focus(&self.focus)
            .size_full()
            .font_family("IBM Plex Sans")
            .text_size(px(13.))
            .text_color(color("text"))
            .bg(rgb(0x131617))
            .on_key_down(cx.listener(|s, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && s.maximized.take().is_some() {
                    cx.notify();
                    return;
                }
                if event.keystroke.modifiers.platform {
                    match event.keystroke.key.as_str() {
                        "1" => s.navigate(false),
                        "2" => s.navigate(true),
                        "j" => s.native = false,
                        "n" => s.native = true,
                        "d" => s.drawer = !s.drawer,
                        _ => return,
                    }
                } else if s.state_view {
                    match event.keystroke.key.as_str() {
                        "up" => s.move_selection(-1),
                        "down" => s.move_selection(1),
                        "escape" => s.drawer = false,
                        _ => return,
                    }
                } else {
                    return;
                }
                cx.notify()
            }))
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar)
                    .child(col().flex_1().px_3().pb_3().child(toolbar).child(content)),
            )
    }
}
