use super::*;
use gpui_component::{Sizable, input::Input, text::TextView};

pub(super) fn selectable(id: String, text: &str, window: &mut Window, cx: &mut App) -> TextView {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    TextView::html(
        SharedString::from(id),
        format!("<pre>{escaped}</pre>"),
        window,
        cx,
    )
    .font_family("IBM Plex Mono")
    .text_size(px(13.))
    .line_height(px(19.))
    .whitespace_nowrap()
    .selectable(true)
}
impl Debugger {
    pub(super) fn console(&self, width: f32, window: &mut Window, cx: &mut Context<Self>) -> Div {
        match self.maximized {
            Some(Panel::Network) => return self.network(width, window, cx),
            Some(Panel::Preview) => return self.diff_preview(cx),
            Some(Panel::Timeline) => return self.timeline(width, cx),
            Some(Panel::Logs { native }) => return self.logs(native, false, window, cx),
            _ => {}
        }
        let top = div()
            .flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.network(width, window, cx).flex_basis(relative(0.6)))
            .child(self.diff_preview(cx).flex_basis(relative(0.4)));
        col()
            .flex_1()
            .gap_2()
            .child(top)
            .child(self.logs_row(width, false, window, cx).flex_1())
            .child(self.timeline(width, cx))
    }
    fn interval(&self) -> u64 {
        self.capture.newest().max(1).div_ceil(250) * 250
    }
    fn timeline(&self, width: f32, cx: &mut Context<Self>) -> Div {
        let end = self.interval();
        let offset = if width < 1280. { 52. } else { 168. } + 25.;
        let track_width = width - offset - 25.;
        let mut track = div()
            .id("time-track")
            .relative()
            .bg(color("surface"))
            .h(px(if self.maximized == Some(Panel::Timeline) {
                80.
            } else {
                10.
            }))
            .cursor_pointer()
            .on_click(cx.listener(move |s, e: &ClickEvent, _, cx| {
                s.capture.pin(
                    (((f32::from(e.position().x) - offset) / track_width).clamp(0., 1.)
                        * end as f32) as u64,
                );
                cx.notify()
            }));
        for e in &self.capture.session.events {
            track = track.child(
                div()
                    .absolute()
                    .left(relative(e.at_ms as f32 / end as f32))
                    .w(px(2.))
                    .h_full()
                    .bg(color("secondary")),
            );
        }
        track = track.child(
            div()
                .absolute()
                .left(relative(self.capture.cursor() as f32 / end as f32))
                .w(px(3.))
                .h_full()
                .bg(color("accent")),
        );
        panel()
            .when(self.maximized != Some(Panel::Timeline), |d| {
                d.flex_none().flex_basis(auto()).h(px(64.))
            })
            .px_3()
            .justify_center()
            .child(
                row()
                    .gap_3()
                    .child(heading("TIME"))
                    .child(muted(format!(
                        "{}  /  {} ms",
                        time(self.capture.cursor()),
                        end
                    )))
                    .child(div().flex_1())
                    .child(self.panel_control(Panel::Timeline, cx)),
            )
            .child(track)
            .child(
                row()
                    .justify_between()
                    .children((0..=4).map(|i| muted(format!("{} ms", end * i / 4)))),
            )
    }
    fn network(&self, width: f32, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let events: Vec<_> = self
            .capture
            .visible()
            .filter(|e| matches!(e.data, Data::Network { .. }))
            .collect();
        let cursor = self.capture.cursor();
        let failures = events
            .iter()
            .filter(|e| {
                matches!(
                    e.network_status_at(cursor),
                    Some((Some(400..), _, _)) | Some((_, _, Some(NetworkOutcome::Failed)))
                )
            })
            .count();
        let title = head(
            "NETWORK REQUESTS",
            format!("{} requests · {} failed", events.len(), failures),
        )
        .child(div().flex_1())
        .child(self.panel_control(Panel::Network, cx));
        let waterfall = width >= 1280.;
        let columns = row()
            .flex_none()
            .h(px(32.))
            .px_2()
            .gap_2()
            .border_b_1()
            .border_color(color("divider"))
            .child(muted("Method").w(px(54.)))
            .child(muted("Path").flex_1())
            .child(muted("Status").w(px(66.)))
            .child(muted("Time").w(px(58.)))
            .when(waterfall, |d| d.child(muted("Waterfall").w(px(92.))));
        let mut list = scroll("requests");
        for e in events {
            let Data::Network { method, path, .. } = &e.data else {
                continue;
            };
            let id = e.id.clone();
            let (status, duration_ms, outcome) = e.network_status_at(cursor).unwrap();
            let fail = status.is_some_and(|s| s >= 400) || outcome == Some(NetworkOutcome::Failed);
            let status_label =
                status
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| match outcome {
                        Some(NetworkOutcome::Failed) => "Failed".into(),
                        Some(NetworkOutcome::Cancelled) => "Canceled".into(),
                        Some(NetworkOutcome::Disconnected) => "Lost".into(),
                        _ if self.live => "Pending".into(),
                        _ => "—".into(),
                    });
            list = list.child(
                row()
                    .id(SharedString::from(format!("req-{id}")))
                    .when(self.capture.request.as_ref() == Some(&id), |d| {
                        d.bg(rgb(0x392f23))
                    })
                    .focusable()
                    .tab_stop(true)
                    .focus(|s| s.bg(rgb(0x393833)))
                    .cursor_pointer()
                    .hover(|s| s.bg(color("surface")))
                    .flex_none()
                    .h(px(32.))
                    .px_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(0x303437))
                    .child(mono(method, if method == "GET" { "key" } else { "accent" }).w(px(54.)))
                    .child(mono(path, "text").flex_1().overflow_hidden())
                    .child(mono(status_label, if fail { "error" } else { "string" }).w(px(66.)))
                    .child(
                        mono(
                            duration_ms.map_or("—".into(), |v| format!("{v} ms")),
                            "secondary",
                        )
                        .w(px(58.)),
                    )
                    .when(waterfall, |d| {
                        d.child(
                            div()
                                .w(px(92.))
                                .h(px(8.))
                                .relative()
                                .bg(color("surface"))
                                .when_some(duration_ms, |d, ms| {
                                    d.child(
                                        div()
                                            .absolute()
                                            .left(relative(e.at_ms as f32 / self.interval() as f32))
                                            .w(relative(ms as f32 / self.interval() as f32))
                                            .h_full()
                                            .bg(color(if fail { "error" } else { "string" })),
                                    )
                                }),
                        )
                    })
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.capture.request = Some(id.clone());
                        cx.notify()
                    })),
            );
        }
        if let Some(e) = self.capture.selected(&self.capture.request)
            && let Data::Network { response, path, .. } = &e.data
        {
            list = list.child(
                row()
                    .px_2()
                    .justify_between()
                    .child(heading("RESPONSE"))
                    .child(self.button("close-response", "Close", false, cx, |s, _| {
                        s.capture.request = None
                    })),
            );
            let mut body = format!(
                "{path}\n{}",
                response
                    .as_ref()
                    .map_or("No response captured".into(), |v| {
                        serde_json::to_string_pretty(v).unwrap()
                    })
            );
            for other in self.capture.visible().filter(|o| e.related(o)) {
                if let Some(log) = other.log(false).or_else(|| other.log(true)) {
                    body.push_str(&format!(
                        "\n{}  {}  {}",
                        log.level,
                        log.message,
                        serde_json::json!(log.args)
                    ));
                }
            }
            list = list.child(selectable(format!("response-{}", e.id), &body, window, cx));
        }
        if self
            .capture
            .visible()
            .all(|e| !matches!(e.data, Data::Network { .. }))
        {
            list = list.child(muted("No requests captured").p_3());
        }
        panel().child(title).child(columns).child(list)
    }
    pub(super) fn logs_row(
        &self,
        width: f32,
        related: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        if width >= 1100. {
            return div()
                .flex()
                .min_h_0()
                .gap_2()
                .child(self.logs(false, related, window, cx))
                .child(self.logs(true, related, window, cx));
        }
        col()
            .gap_1()
            .child(
                row()
                    .gap_2()
                    .child(self.button("ch-js", "JS", !self.native, cx, |s, _| s.native = false))
                    .child(self.button("ch-native", "Native", self.native, cx, |s, _| {
                        s.native = true
                    })),
            )
            .child(self.logs(self.native, related, window, cx))
    }
    pub(super) fn logs(
        &self,
        native: bool,
        related: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        const LEVELS: [&str; 4] = ["All", "INFO", "WARN", "ERROR"];
        let channel = usize::from(native);
        let query = self.filters[channel].read(cx).value().to_lowercase();
        let selected = self.capture.selected(&self.capture.action);
        let entries: Vec<_> = self
            .capture
            .visible()
            .filter(|e| !related || selected.is_some_and(|s| s.related(e)))
            .filter_map(|e| Some((e, e.log(native)?)))
            .filter(|(_, l)| {
                related
                    || (self.levels[channel] == 0 || l.level == LEVELS[self.levels[channel]])
                        && format!(
                            "{} {} {}",
                            l.subsystem.as_deref().unwrap_or(""),
                            l.message,
                            serde_json::json!(l.args)
                        )
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();
        let title = format!(
            "{}{} LOGS",
            if related { "RELATED " } else { "" },
            if native { "NATIVE" } else { "JS" }
        );
        let mut bar = head(&title, format!("{} entries", entries.len())).child(div().flex_1());
        if !related {
            bar = bar
                .child(
                    div()
                        .w(px(104.))
                        .child(Input::new(&self.filters[channel]).small().appearance(false)),
                )
                .child(self.button(
                    format!("level-{native}"),
                    LEVELS[self.levels[channel]],
                    false,
                    cx,
                    move |s, _| s.levels[channel] = (s.levels[channel] + 1) % LEVELS.len(),
                ));
        }
        bar = bar.child(self.panel_control(Panel::Logs { native }, cx));
        let mut list = scroll(if native { "native-stream" } else { "js-stream" });
        if entries.is_empty() {
            list = list.child(
                muted(if related {
                    "No related logs"
                } else {
                    "No matching entries"
                })
                .p_3(),
            );
        }
        for (e, log) in entries {
            let id = e.id.clone();
            let open = self.expanded.contains(&id);
            let text = format!(
                "{}  {}  {}  {}",
                time(e.at_ms),
                log.level,
                log.message,
                serde_json::json!(log.args)
            );
            let copy = text.clone();
            let msg = if let Some(tag) = &log.subsystem {
                format!("[{tag}] {}", log.message)
            } else {
                log.message.clone()
            };
            list = list.child(
                row()
                    .flex_none()
                    .min_h(px(34.))
                    .px_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(0x303437))
                    .child(mono(time(e.at_ms), "secondary"))
                    .child(mono(
                        &log.level,
                        match log.level.as_str() {
                            "WARN" => "accent",
                            "ERROR" => "error",
                            _ => "key",
                        },
                    ))
                    .child(mono(msg, "text").flex_1().overflow_hidden())
                    .child(self.button(
                        format!("expand-{id}"),
                        if open { "−" } else { "+" },
                        open,
                        cx,
                        move |s, _| {
                            if !s.expanded.remove(&id) {
                                s.expanded.insert(id.clone());
                            }
                        },
                    ))
                    .child(self.button(
                        format!("copy-{}", e.id),
                        "Copy",
                        false,
                        cx,
                        move |_, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone())),
                    )),
            );
            if open {
                list = list.child(selectable(
                    format!("args-{}", e.id),
                    &format!(
                        "{}\n{}",
                        log.message,
                        serde_json::to_string_pretty(&log.args).unwrap()
                    ),
                    window,
                    cx,
                ));
            }
        }
        panel().child(bar).child(list)
    }
}
