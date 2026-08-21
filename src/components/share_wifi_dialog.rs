use qrcode::QrCode;

use crate::{
    components::{ButtonKind, styled_button},
    services::network::{PskOutcome, WifiSecurity},
    t,
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, column, container, image, row, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Close,
}

/// Splits a `PskOutcome` into the fields the dialog view consumes, plus the
/// WIFI URI payload to encode. Returns `(password, error, qr_payload)` so the
/// caller can build a single `image::Handle` for the dialog state.
pub fn materialize(
    ssid: &str,
    outcome: PskOutcome,
) -> (Option<String>, Option<String>, Option<String>) {
    match outcome {
        PskOutcome::Password {
            psk,
            security,
            hidden,
        } => {
            let payload = build_wifi_payload(ssid, Some((&psk, security)), hidden);
            (Some(psk), None, Some(payload))
        }
        PskOutcome::Open { hidden } => {
            let payload = build_wifi_payload(ssid, None, hidden);
            (None, None, Some(payload))
        }
        PskOutcome::PasswordUnavailable => (
            None,
            Some(t!("share-wifi-password-unavailable-message").to_string()),
            None,
        ),
        PskOutcome::Unsupported => (
            None,
            Some(t!("share-wifi-unsupported-message").to_string()),
            None,
        ),
        PskOutcome::Error(e) => (None, Some(e), None),
    }
}

pub fn view<'a>(
    ssid: &str,
    password: Option<String>,
    error: Option<String>,
    qr: Option<(image::Handle, u32)>,
) -> Element<'a, Message> {
    let (space, font_size) = use_theme(|theme| (theme.space, theme.font_size));

    let mut col: Column<'a, Message> = column![
        row!(text(t!("share-wifi-title")).size(font_size.xl),)
            .spacing(space.md)
            .align_y(Alignment::Center),
        text(t!("share-wifi-subtitle", ssid = ssid)).width(Length::Fill),
    ]
    .spacing(space.md);

    if let Some((handle, size)) = qr {
        col = col.push(
            container(
                image(handle)
                    .width(Length::Fixed(size as f32))
                    .height(Length::Fixed(size as f32)),
            )
            .center_x(Length::Fill)
            .padding(space.sm),
        );
    }

    match (password.as_deref(), error) {
        (Some(p), _) => {
            col = col.push(
                row!(text(t!("share-wifi-password-label")), text(p.to_string()),)
                    .spacing(space.xs)
                    .align_y(Alignment::Center),
            );
        }
        (None, Some(err)) => {
            col = col.push(
                text(err)
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.palette().danger),
                    })
                    .width(Length::Fill),
            );
        }
        (None, None) => {
            col = col.push(text(t!("share-wifi-password-open-network")).style(
                |theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text),
                },
            ));
        }
    }

    col = col.push(
        row!(
            iced::widget::space::horizontal(),
            styled_button(t!("share-wifi-close"))
                .kind(ButtonKind::Outline)
                .height(Length::Fixed(50.))
                .on_press(Message::Close),
        )
        .width(Length::Fill),
    );

    col.spacing(space.md).padding(space.md).into()
}

fn wifi_escape(s: &str) -> String {
    // ZXing WIFI URI escape: backslash-escape every reserved separator.
    // https://github.com/zxing/zxing/wiki/Barcode-Contents#wifi-network-config
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ';' | ',' | ':' | '"' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn build_wifi_payload(ssid: &str, password: Option<(&str, WifiSecurity)>, hidden: bool) -> String {
    let escaped_ssid = wifi_escape(ssid);
    // `H:true` is only meaningful for hidden networks; scanners ignore the
    // field when absent, so omit it otherwise.
    let hidden_field = if hidden { "H:true;" } else { "" };
    // The URI is terminated by an empty field, hence the trailing ';'.
    match password {
        Some((pw, security)) if !pw.is_empty() => format!(
            "WIFI:T:{};S:{};P:{};{};",
            security.qr_auth_type(),
            escaped_ssid,
            wifi_escape(pw),
            hidden_field
        ),
        _ => format!("WIFI:T:nopass;S:{escaped_ssid};{hidden_field};"),
    }
}

/// Builds an `image::Handle` for the given WIFI URI payload. Returns the handle
/// together with the rendered pixel size so the view can display it at its
/// natural dimensions rather than scaling it. Public so the dialog state can
/// cache the result at construction time and avoid re-creating the handle on
/// every frame (which would defeat `iced`'s texture cache).
///
/// Pixels are pushed manually rather than via `qrcode::render::image::Image` to
/// avoid pulling in the `image` crate — `qrcode` is declared with
/// `default-features = false` in `Cargo.toml` for that reason.
pub fn qr_image_handle(payload: &str) -> Option<(image::Handle, u32)> {
    let qr = QrCode::new(payload.as_bytes()).ok()?;
    let modules = qr.width();
    let dark: Vec<bool> = qr
        .to_colors()
        .into_iter()
        .map(|c| matches!(c, qrcode::Color::Dark))
        .collect();
    let quiet: usize = 4;
    let total_modules = modules + quiet * 2;
    // Aim for ≤220 px but guarantee at least 2 px/module so the code stays
    // scannable. Very high QR versions (large payloads) may exceed 220 px.
    let scale = (220u32 / total_modules as u32).max(2);
    let pixels_per_side = (total_modules as u32) * scale;

    let mut buf = Vec::with_capacity((pixels_per_side * pixels_per_side * 4) as usize);
    for py in 0..pixels_per_side {
        let my = (py / scale) as usize;
        for px in 0..pixels_per_side {
            let mx = (px / scale) as usize;
            let on = if mx < quiet || my < quiet || mx >= quiet + modules || my >= quiet + modules {
                false
            } else {
                dark.get((my - quiet) * modules + (mx - quiet))
                    .copied()
                    .unwrap_or(false)
            };
            if on {
                buf.extend_from_slice(&[0, 0, 0, 255]);
            } else {
                buf.extend_from_slice(&[255, 255, 255, 255]);
            }
        }
    }

    Some((
        image::Handle::from_rgba(pixels_per_side, pixels_per_side, buf),
        pixels_per_side,
    ))
}
