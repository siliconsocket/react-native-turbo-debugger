use gpui::*;
use std::sync::LazyLock;

struct Tokens(serde_json::Value);
static TOKENS: LazyLock<Tokens> = LazyLock::new(|| {
    Tokens(serde_json::from_str(include_str!("../../assets/tokens.json")).unwrap())
});
pub fn color(name: &str) -> Rgba {
    parse(TOKENS.0["colors"][name].as_str().unwrap())
}
fn parse(value: &str) -> Rgba {
    rgb(u32::from_str_radix(&value[1..], 16).unwrap())
}
pub fn gradient() -> Background {
    let stops = &TOKENS.0["titleGradient"];
    linear_gradient(
        180.,
        linear_color_stop(parse(stops[0].as_str().unwrap()), 0.),
        linear_color_stop(parse(stops[1].as_str().unwrap()), 1.),
    )
}
pub fn row() -> Div {
    div().flex().items_center()
}
pub fn col() -> Div {
    div().flex().flex_col().min_h_0().min_w_0()
}
pub fn panel() -> Div {
    col()
        .flex_1()
        .border_1()
        .border_color(color("divider"))
        .rounded(px(3.))
        .bg(color("canvas"))
        .overflow_hidden()
}
pub fn head(title: &str, meta: impl Into<String>) -> Div {
    row()
        .flex_none()
        .h(px(38.))
        .gap_2()
        .px_3()
        .bg(color("surface"))
        .border_b_1()
        .border_color(color("divider"))
        .child(heading(title))
        .child(muted(meta))
}
pub fn heading(s: &str) -> Div {
    div()
        .font_family("Barlow Condensed")
        .text_size(px(18.))
        .child(s.to_owned())
}
pub fn muted(s: impl Into<String>) -> Div {
    div().text_color(color("secondary")).child(s.into())
}
pub fn mono(s: impl Into<String>, tone: &str) -> Div {
    div()
        .font_family("IBM Plex Mono")
        .text_size(px(13.))
        .line_height(px(19.))
        .whitespace_nowrap()
        .text_color(color(tone))
        .child(s.into())
}
pub fn scroll(id: &str) -> Stateful<Div> {
    col()
        .id(SharedString::from(id.to_owned()))
        .flex_1()
        .overflow_scroll()
}
pub fn time(ms: u64) -> String {
    format!(
        "{:02}:{:02}.{:03}",
        ms / 60_000,
        (ms / 1000) % 60,
        ms % 1000
    )
}
