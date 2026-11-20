use super::{console::selectable, *};
use serde_json::Value;
use std::collections::BTreeSet;

fn changes(before: &Value, after: &Value) -> Div {
    let mut lines = col().py_2().font_family("IBM Plex Mono");
    if before == after {
        return lines.child(muted("No changes").px_3());
    }
    let mut entries = Vec::new();
    if let (Some(left), Some(right)) = (before.as_object(), after.as_object()) {
        for key in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
            if left.get(key) != right.get(key) {
                entries.push((key.as_str(), left.get(key), right.get(key)));
            }
        }
    } else {
        entries.push(("state", Some(before), Some(after)));
    }
    for (name, old, new) in entries {
        lines = lines.child(mono(format!("⌄ {name}"), "key").px_3().py_1());
        for (symbol, value, bg, tone) in [
            ("−", old, 0x372b2c, "error"),
            ("+", new, 0x26372e, "string"),
        ] {
            if let Some(v) = value {
                lines = lines.child(
                    row()
                        .flex_none()
                        .min_h(px(28.))
                        .gap_3()
                        .px_3()
                        .bg(rgb(bg))
                        .child(mono(symbol, tone))
                        .child(mono(format!("{name}:"), "key"))
                        .child(mono(
                            v.to_string(),
                            if v.is_string() { "string" } else { "accent" },
                        )),
                );
            }
        }
    }
    lines
}
impl Debugger {
    pub(super) fn diff_preview(&self, width: f32, cx: &mut Context<Self>) -> Div {
        let mut body = scroll("diff-preview");
        let mut rev = String::new();
        if let Some(Event {
            data:
                Data::Zustand {
                    revision_before,
                    revision_after,
                    before,
                    after,
                    ..
                },
            ..
        }) = self.capture.selected(&self.capture.action)
        {
            rev = format!("revision {revision_before} → {revision_after}");
            if width < 1280. && self.maximized.is_none() {
                body = body.child(muted(std::mem::take(&mut rev)).px_3().pt_1());
            }
            body = body.child(changes(before, after));
        } else {
            body = body.child(muted("No action selected").p_3());
        }
        panel()
            .child(
                head("ZUSTAND STATE DIFF", rev)
                    .child(div().flex_1())
                    .child(self.button("inspect", "Inspect ↗", false, cx, |s, _| s.navigate(true)))
                    .child(self.panel_control(Panel::Preview, cx)),
            )
            .child(body)
    }
    pub(super) fn investigation(
        &self,
        width: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        if let Some(Panel::Logs { native }) = self.maximized {
            return self.logs(native, true, window, cx);
        }
        let events: Vec<_> = self
            .capture
            .visible()
            .filter(|e| matches!(e.data, Data::Zustand { .. }))
            .collect();
        let mut history = panel()
            .when(self.maximized.is_none(), |d| {
                d.flex_none()
                    .flex_basis(auto())
                    .w(px(if width < 1100. { 190. } else { 240. }))
            })
            .child(
                head(
                    if width < 1100. {
                        "HISTORY"
                    } else {
                        "ACTION HISTORY"
                    },
                    events.len().to_string(),
                )
                .child(div().flex_1())
                .child(self.panel_control(Panel::History, cx)),
            );
        let mut list = scroll("history");
        for e in events {
            let Data::Zustand { action, .. } = &e.data else {
                continue;
            };
            let id = e.id.clone();
            list = list.child(
                self.button(
                    format!("history-{id}"),
                    format!("{}    {action}", time(e.at_ms)),
                    self.capture.action.as_ref() == Some(&id),
                    cx,
                    move |s, _| s.capture.action = Some(id.clone()),
                )
                .rounded_none()
                .h(px(32.)),
            );
        }
        history = history.child(list);
        if self.maximized == Some(Panel::History) {
            return history;
        }
        let mut tabs = row()
            .h(px(38.))
            .flex_none()
            .bg(color("surface"))
            .items_center()
            .gap_1()
            .px_2();
        for (i, label) in ["Diff", "Next", "Previous", "Raw"].into_iter().enumerate() {
            tabs = tabs.child(self.button(
                format!("tab-{i}"),
                label,
                self.tab == i,
                cx,
                move |s, _| s.tab = i,
            ));
        }
        tabs = tabs
            .child(div().flex_1())
            .child(self.panel_control(Panel::Inspector, cx));
        let mut code = scroll("code");
        let mut details = scroll("event-details").p_3().gap_2();
        if let Some(e) = self.capture.selected(&self.capture.action)
            && let Data::Zustand {
                store,
                action,
                revision_before,
                revision_after,
                before,
                after,
                payload,
            } = &e.data
        {
            let rev = format!("{revision_before} → {revision_after}");
            code = code.child(
                mono(format!("{store}   ·   revision {rev}"), "secondary")
                    .px_3()
                    .py_2(),
            );
            if self.tab == 0 {
                code = code.child(changes(before, after));
            } else {
                let value = match self.tab {
                    1 => after.clone(),
                    2 => before.clone(),
                    _ => serde_json::to_value(e).unwrap(),
                };
                code = code.child(selectable(
                    format!("snapshot-{}-{}", e.id, self.tab),
                    &serde_json::to_string_pretty(&value).unwrap(),
                    window,
                    cx,
                ));
            }
            for (label, value) in [
                ("Time", time(e.at_ms)),
                ("Store", store.clone()),
                ("Action", action.clone()),
                ("State revision", rev),
                (
                    "Payload",
                    payload
                        .as_ref()
                        .map_or("No payload captured".into(), |p| p.to_string()),
                ),
            ] {
                details = details.child(muted(label)).child(mono(value, "text"));
            }
            let linked: Vec<_> = self
                .capture
                .visible()
                .filter(|o| e.related(o) && matches!(o.data, Data::Network { .. }))
                .collect();
            details = details.child(muted("Related network requests"));
            if linked.is_empty() {
                details = details.child(muted("None"));
            }
            for other in linked {
                if let Data::Network { method, path, .. } = &other.data {
                    let id = other.id.clone();
                    details = details.child(self.button(
                        format!("linked-{id}"),
                        format!("{method} {path}"),
                        false,
                        cx,
                        move |s, _| {
                            s.navigate(false);
                            s.capture.request = Some(id.clone())
                        },
                    ));
                }
            }
        } else {
            code = code.child(muted("No action selected").p_3());
        }
        let details = panel()
            .when(self.maximized.is_none(), |d| {
                d.flex_none().flex_basis(auto()).w(px(264.))
            })
            .child(
                head("EVENT DETAILS", "")
                    .child(div().flex_1())
                    .child(self.panel_control(Panel::Details, cx)),
            )
            .child(details);
        let inspector = panel().child(tabs).child(code);
        match self.maximized {
            Some(Panel::Details) => return details,
            Some(Panel::Inspector) => return inspector,
            _ => {}
        }
        let middle = div()
            .flex()
            .relative()
            .min_h_0()
            .flex_1()
            .gap_2()
            .child(history)
            .child(inspector)
            .when(width >= 1280. || self.drawer, |d| {
                if width >= 1280. {
                    d.child(details)
                } else {
                    d.child(details.absolute().right_0().top_0().bottom_0().occlude())
                }
            });
        col().flex_1().gap_2().child(middle).child(
            self.logs_row(width, true, window, cx)
                .h(px(160.))
                .flex_none(),
        )
    }
}
