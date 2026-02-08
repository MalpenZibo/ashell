use guido::prelude::*;

use crate::services::CompositorState;
use crate::theme;

const MAX_TITLE_LEN: usize = 150;

pub fn view(state: Signal<CompositorState>) -> impl Widget {
    container()
        .overflow(Overflow::Hidden)
        .child(
            text(move || {
                let title = state
                    .with(|s| {
                        s.active_window
                            .as_ref()
                            .map(|w| w.title().to_string())
                    })
                    .unwrap_or_default();
                if title.len() > MAX_TITLE_LEN {
                    format!("{}...", &title[..MAX_TITLE_LEN])
                } else {
                    title
                }
            })
            .color(theme::TEXT)
            .font_size(13.0)
            .nowrap(),
        )
}
