use gpui::*;
use gpui_component::{Root, Theme, ThemeMode};
use react_native_turbo_debugger::ui::Debugger;
use std::borrow::Cow;

fn main() {
    Application::new().run(|cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        let fonts: &[&'static [u8]] = &[
            include_bytes!("../assets/fonts/BarlowCondensed-SemiBold.ttf"),
            include_bytes!("../assets/fonts/IBMPlexSans.ttf"),
            include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
        ];
        cx.text_system()
            .add_fonts(fonts.iter().map(|f| Cow::Borrowed(*f)).collect())
            .expect("bundled fonts");
        let dims = std::env::var("RNTD_SIZE")
            .ok()
            .and_then(|v| {
                let (w, h) = v.split_once('x')?;
                Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?))
            })
            .unwrap_or((1440., 900.));
        let bounds = Bounds::centered(None, size(px(dims.0), px(dims.1)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1024.), px(700.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("React Native Turbo Debugger".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.), px(22.))),
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Debugger::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open debugger window");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit()
            }
        })
        .detach();
        cx.activate(true);
    });
}
