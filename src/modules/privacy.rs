use guido::prelude::*;

use crate::components::{StaticIcon, icon};
use crate::services::compat::{ServiceSignal, run_readonly_service};
use crate::services::privacy::PrivacyService;
use crate::theme::ThemeColors;

pub fn create() -> ServiceSignal<PrivacyService> {
    run_readonly_service::<PrivacyService>()
}

/// Bar view: mic/webcam/screenshare in-use icons, hidden when idle.
pub fn view(data: ServiceSignal<PrivacyService>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    container().child(move || {
        let (screenshare, webcam, mic) = data.with(|s| {
            s.as_ref()
                .filter(|x| !x.no_access())
                .map(|x| {
                    (
                        x.screenshare_access(),
                        x.webcam_access(),
                        x.microphone_access(),
                    )
                })
                .unwrap_or((false, false, false))
        });
        if !screenshare && !webcam && !mic {
            return None;
        }
        Some(
            container()
                .layout(
                    Flex::row()
                        .spacing(4)
                        .cross_alignment(CrossAlignment::Center),
                )
                .maybe_child(
                    screenshare.then(|| icon().kind(StaticIcon::ScreenShare).color(theme.warning)),
                )
                .maybe_child(webcam.then(|| icon().kind(StaticIcon::Webcam).color(theme.warning)))
                .maybe_child(mic.then(|| icon().kind(StaticIcon::Mic1).color(theme.warning))),
        )
    })
}
