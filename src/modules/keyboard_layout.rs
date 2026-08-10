use guido::prelude::*;

use crate::config::Config;
use crate::services::compositor::{CompositorCommand, CompositorStateSignals};
use crate::theme::ThemeColors;

/// Active keyboard layout; click cycles to the next one. Hidden when the
/// compositor backend doesn't report layouts (empty string).
pub fn view(state: CompositorStateSignals, svc: Service<CompositorCommand>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let labels =
        with_context::<Config, _>(|c| c.keyboard_layout.labels.clone()).unwrap_or_default();

    let label = create_memo(move || {
        state.keyboard_layout.with(|l| {
            if l.is_empty() {
                None
            } else {
                Some(labels.get(l).cloned().unwrap_or_else(|| l.clone()))
            }
        })
    });

    container().child(move || {
        let svc = svc.clone();
        label.get().map(|l| {
            container()
                .padding([0, 4])
                .on_click(move || svc.send(CompositorCommand::NextLayout))
                .child(text(l).color(theme.text).font_size(14))
        })
    })
}
