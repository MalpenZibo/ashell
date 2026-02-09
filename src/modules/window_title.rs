use guido::prelude::*;

use crate::services::CompositorState;
use crate::theme;

const MAX_TITLE_LEN: usize = 150;

pub fn view(state: Signal<CompositorState>) -> impl Widget {
    // Only re-renders when the actual title string changes
    let title = create_memo(move || {
        state.with(|s| {
            s.active_window
                .as_ref()
                .map(|w| w.title().to_string())
                .unwrap_or_default()
        })
    });

    container().overflow(Overflow::Hidden).child(
        text(move || {
            let t = title.get();
            if t.len() > MAX_TITLE_LEN {
                format!("{}...", &t[..MAX_TITLE_LEN])
            } else {
                t
            }
        })
        .color(theme::TEXT)
        .font_size(13.0)
        .nowrap(),
    )
}
