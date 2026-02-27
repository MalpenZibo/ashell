use guido::prelude::*;
use std::time::Duration;

use crate::config::Config;
use crate::theme::ThemeColors;

pub fn view() -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let format = with_context::<Config, _>(|c| c.clock.format.clone()).unwrap();
    let clock_text = create_signal(format_time(&format));
    let clock_writer = clock_text.writer();

    let _ = create_service::<(), _, _>(move |_rx, ctx| {
        let format = format.clone();
        async move {
            while ctx.is_running() {
                clock_writer.set(format_time(&format));
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    });

    text(move || clock_text.get())
        .color(theme.text)
        .font_size(13.0)
}

fn format_time(format: &str) -> String {
    chrono::Local::now().format(format).to_string()
}
