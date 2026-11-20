use super::*;
use gpui_component::{
    highlighter::{HighlightTheme, SyntaxHighlighter},
    input::Rope,
    text::{TextView, TextViewStyle},
};
use serde::Serialize;
use serde_json::Value;
use std::sync::{Arc, LazyLock};

pub(super) fn syntax_theme() -> Arc<HighlightTheme> {
    static THEME: LazyLock<Arc<HighlightTheme>> = LazyLock::new(|| {
        Arc::new(
            serde_json::from_str(include_str!("../../assets/monokai.json"))
                .expect("valid syntax theme"),
        )
    });
    THEME.clone()
}

pub(super) fn json_block(
    id: String,
    value: &impl Serialize,
    window: &mut Window,
    cx: &mut App,
) -> TextView {
    let json = serde_json::to_string_pretty(value).expect("valid JSON data");
    // JSON escapes newlines inside strings, so its content cannot close the fence.
    let markdown = format!("```json\n{json}\n```");
    let block_style = monospace().bg(color("canvas")).style().clone();
    TextView::markdown(SharedString::from(id), markdown, window, cx)
        .style(TextViewStyle::default().code_block(block_style))
        .whitespace_nowrap()
        .selectable(true)
}

pub(super) fn json_line(value: &Value) -> Div {
    let json = value.to_string();
    let theme = syntax_theme();
    let token = match value {
        Value::Null => Some("constant"),
        Value::Bool(_) => Some("boolean"),
        Value::Number(_) => Some("number"),
        _ => None,
    };
    let styles = if let Some(token) = token {
        theme
            .style(token)
            .map(|style| (0..json.len(), style))
            .into_iter()
            .collect()
    } else {
        let mut highlighter = SyntaxHighlighter::new("json");
        highlighter.update(None, &Rope::from_str(&json));
        highlighter.styles(&(0..json.len()), &theme)
    };
    monospace()
        .text_color(color("text"))
        .child(StyledText::new(json).with_highlights(styles))
}
