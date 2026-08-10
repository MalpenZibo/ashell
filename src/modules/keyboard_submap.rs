use guido::prelude::*;

use crate::services::compositor::CompositorStateSignals;
use crate::theme::ThemeColors;

/// Active keyboard submap (Hyprland); hidden when none is active.
pub fn view(state: CompositorStateSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    let submap = create_memo(move || state.submap.with(|s| s.clone().filter(|s| !s.is_empty())));

    container().child(move || {
        submap
            .get()
            .map(|s| text(s).color(theme.text).font_size(14))
    })
}
