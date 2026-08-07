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
        let tick_secs = has_seconds(&format);
        async move {
            while ctx.is_running() {
                clock_writer.set(format_time(&format));
                // Formats without seconds only change once a minute — wake
                // at the minute boundary instead of every second.
                let sleep_ms = if tick_secs {
                    1000
                } else {
                    let now = chrono::Local::now();
                    use chrono::Timelike;
                    (60 - u64::from(now.second())).max(1) * 1000 + 50
                };
                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            }
        }
    });

    text(move || clock_text.get())
        .color(theme.text)
        .font_size(13)
}

/// Whether a strftime format renders seconds (needs a 1s tick).
fn has_seconds(format: &str) -> bool {
    ["%S", "%T", "%X", "%r", "%s", "%.f", "%3f", "%6f", "%9f"]
        .iter()
        .any(|specifier| format.contains(specifier))
}

fn format_time(format: &str) -> String {
    chrono::Local::now().format(format).to_string()
}
