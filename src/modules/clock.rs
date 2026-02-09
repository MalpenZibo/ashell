use guido::prelude::*;
use std::time::Duration;

use crate::theme;

pub fn view() -> impl Widget {
    let clock_text = create_signal(format_time());
    let clock_writer = clock_text.writer();

    let _ = create_service::<(), _, _>(move |_rx, ctx| async move {
        while ctx.is_running() {
            clock_writer.set(format_time());
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    text(move || clock_text.get())
        .color(theme::TEXT)
        .font_size(13.0)
}

fn format_time() -> String {
    chrono::Local::now().format("%a %d %b %R").to_string()
}
