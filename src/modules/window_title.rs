use guido::prelude::*;

use crate::config::Config;
use crate::services::compositor::CompositorStateSignals;
use crate::theme::ThemeColors;

pub fn view(state: CompositorStateSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let max_len = with_context::<Config, _>(|c| c.window_title.truncate_title_after_length as usize).unwrap();

    // Only re-renders when active_window changes (per-field signal)
    let title = create_memo(move || {
        state.active_window.with(|w| {
            w.as_ref()
                .map(|w| w.title().to_string())
                .unwrap_or_default()
        })
    });

    container().child(
        text(move || {
            let t = title.get();
            if t.len() > max_len {
                format!("{}...", &t[..max_len])
            } else {
                t
            }
        })
        .color(theme.text)
        .font_size(13.0)
        .nowrap(),
    )
}
