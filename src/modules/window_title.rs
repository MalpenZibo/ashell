use guido::prelude::*;

use crate::config::{Config, WindowTitleMode};
use crate::services::compositor::CompositorStateSignals;
use crate::theme::ThemeColors;
use crate::truncate_text;

/// Hard limit matching upstream: prevents Wayland E2BIG errors on
/// pathological titles.
const MAX_TITLE_LENGTH: u32 = 2048;

pub fn view(state: CompositorStateSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let (mode, cfg_len) = with_context::<Config, _>(|c| {
        (
            c.window_title.mode,
            c.window_title.truncate_title_after_length,
        )
    })
    .unwrap();
    let max_len = if cfg_len > 0 {
        cfg_len.min(MAX_TITLE_LENGTH)
    } else {
        MAX_TITLE_LENGTH
    };

    // Only re-renders when active_window changes (per-field signal)
    let title = create_memo(move || {
        state.active_window.with(|w| {
            w.as_ref()
                .map(|w| match mode {
                    WindowTitleMode::Title => w.title().to_string(),
                    WindowTitleMode::Class => w.class().to_string(),
                    WindowTitleMode::InitialTitle => {
                        w.initial_title().map(str::to_string).unwrap_or_else(|e| {
                            log::warn!("{e}");
                            String::new()
                        })
                    }
                    WindowTitleMode::InitialClass => {
                        w.initial_class().map(str::to_string).unwrap_or_else(|e| {
                            log::warn!("{e}");
                            String::new()
                        })
                    }
                })
                .unwrap_or_default()
        })
    });

    container()
        .child(
            text(move || truncate_text(&title.get(), max_len))
                .color(theme.text)
                .font_size(12)
                .nowrap(),
        )
        .overflow(Overflow::Hidden)
        .animate_width(Transition::spring(SpringConfig::SNAPPY))
}
