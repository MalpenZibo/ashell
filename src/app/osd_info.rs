use crate::{
    app::App, ipc::IpcCommand, modules::settings::audio, osd::OsdKind,
    services::airplane::AirplaneCommand,
};

/// Build OSD display info (kind, normalised value, bar scale, muted) for the given
/// IPC command, reading current state from the Settings services.
pub fn osd_info_for(app: &App, cmd: &IpcCommand) -> Option<(OsdKind, f32, f32, bool)> {
    fn normalise(cur: u32, max: u32) -> f32 {
        if max > 0 {
            cur as f32 / max as f32
        } else {
            0.0
        }
    }

    match cmd {
        IpcCommand::VolumeUp { .. } | IpcCommand::VolumeDown { .. } => {
            // Use slider value — it has the optimistic RequestAndTimeout update,
            // which was computed from real_sink_volume in volume_adjust().
            let vol = app.settings.audio().current_sink_volume().unwrap_or(0);
            let muted = app.settings.audio().is_sink_muted().unwrap_or(false);
            let scale = normalise(app.settings.audio().vol_max(), audio::NORMAL_VOLUME).max(1.0);
            Some((
                OsdKind::Volume,
                normalise(vol, audio::NORMAL_VOLUME),
                scale,
                muted,
            ))
        }
        IpcCommand::VolumeSetMute { state, .. } => {
            let vol = app.settings.audio().real_sink_volume().unwrap_or(0);
            let muted = *state;
            let scale = normalise(app.settings.audio().vol_max(), audio::NORMAL_VOLUME).max(1.0);
            Some((
                OsdKind::Volume,
                normalise(vol, audio::NORMAL_VOLUME),
                scale,
                muted,
            ))
        }
        IpcCommand::VolumeToggleMute { .. } => {
            let vol = app.settings.audio().real_sink_volume().unwrap_or(0);
            // Invert: the toggle was just sent but PulseAudio hasn't
            // round-tripped yet, so the current state is stale.
            let muted = !app.settings.audio().is_sink_muted().unwrap_or(false);
            let scale = normalise(app.settings.audio().vol_max(), audio::NORMAL_VOLUME).max(1.0);
            Some((
                OsdKind::Volume,
                normalise(vol, audio::NORMAL_VOLUME),
                scale,
                muted,
            ))
        }
        IpcCommand::MicrophoneUp { .. } | IpcCommand::MicrophoneDown { .. } => {
            // Use slider value — it has the optimistic RequestAndTimeout update,
            // which was computed from real_source_volume in microphone_adjust().
            let vol = app.settings.audio().current_source_volume().unwrap_or(0);
            let muted = app.settings.audio().is_source_muted().unwrap_or(false);
            Some((
                OsdKind::Microphone,
                normalise(vol, audio::AudioSettings::mic_max()),
                1.0,
                muted,
            ))
        }
        IpcCommand::MicrophoneSetMute { state, .. } => {
            let vol = app.settings.audio().real_source_volume().unwrap_or(0);
            Some((
                OsdKind::Microphone,
                normalise(vol, audio::AudioSettings::mic_max()),
                1.0,
                *state,
            ))
        }
        IpcCommand::MicrophoneToggleMute { .. } => {
            let vol = app.settings.audio().real_source_volume().unwrap_or(0);
            // Invert: the toggle was just sent but PulseAudio hasn't
            // round-tripped yet, so the current state is stale.
            let muted = !app.settings.audio().is_source_muted().unwrap_or(false);
            Some((
                OsdKind::Microphone,
                normalise(vol, audio::AudioSettings::mic_max()),
                1.0,
                muted,
            ))
        }
        IpcCommand::BrightnessUp { .. } | IpcCommand::BrightnessDown { .. } => app
            .settings
            .brightness()
            .current_brightness()
            .map(|(cur, max)| (OsdKind::Brightness, normalise(cur, max), 1.0, false)),
        IpcCommand::SetAirplaneMode { .. } | IpcCommand::ToggleAirplaneMode { .. } => {
            let command = match cmd {
                IpcCommand::SetAirplaneMode { state, .. } => AirplaneCommand::Set(*state),
                _ => AirplaneCommand::Toggle,
            };
            // `muted` carries the enabled state; `value` is unused. The command was
            // just sent, so the state it leads to is derived from the current one.
            app.settings
                .airplane_after(command)
                .map(|active| (OsdKind::Airplane, 0.0, 1.0, active))
        }
        IpcCommand::SetIdleInhibitor { state, .. } => {
            if app.settings.idle_inhibitor().is_some() {
                Some((OsdKind::IdleInhibitor, 0.0, 1.0, *state))
            } else {
                None
            }
        }
        IpcCommand::ToggleIdleInhibitor { .. } => {
            if let Some(idle_inhibitor) = app.settings.idle_inhibitor() {
                let active = idle_inhibitor.is_inhibited();
                Some((OsdKind::IdleInhibitor, 0.0, 1.0, active))
            } else {
                None
            }
        }
        IpcCommand::ToggleVisibility
        | IpcCommand::SetVisibility { .. }
        | IpcCommand::LoadConfig { .. }
        | IpcCommand::NotificationsClear { .. }
        | IpcCommand::NotificationsInvoke { .. }
        | IpcCommand::NotificationsList => None,
    }
}
