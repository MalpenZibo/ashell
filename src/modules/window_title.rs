use guido::prelude::*;

use crate::services::CompositorStateSignals;
use crate::theme;

const MAX_TITLE_LEN: usize = 150;

pub fn view(state: CompositorStateSignals) -> impl Widget {
    // Only re-renders when active_window changes (per-field signal)
    let title = create_memo(move || {
        state.active_window.with(|w| {
            w.as_ref()
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
