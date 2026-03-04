use guido::prelude::*;

use crate::components::{IconKind, StaticIcon, expandable_panel, icon};
use crate::config::{Config, UpdatesModuleConfig};
use crate::services::updates::{
    UpdatesCmd, UpdatesData, UpdatesDataSignals, start_updates_service,
};
use crate::theme::ThemeColors;

pub fn create() -> (UpdatesDataSignals, Service<UpdatesCmd>) {
    let config = with_context::<Config, _>(|c| {
        c.updates.clone().unwrap_or_else(|| UpdatesModuleConfig {
            check_cmd: "checkupdates".to_string(),
            update_cmd: String::new(),
            interval: 3600,
        })
    })
    .unwrap();
    let data = UpdatesDataSignals::new(UpdatesData::default());
    let svc = start_updates_service(data.writers(), config);
    (data, svc)
}

/// Bar view: icon + count
pub fn view(data: UpdatesDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let is_checking = data.is_checking;
    let updates = data.updates;

    container()
        .layout(
            Flex::row()
                .spacing(4)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(
            icon().ic(move || -> IconKind {
                if is_checking.get() {
                    StaticIcon::Refresh
                } else if updates.with(|u| u.is_empty()) {
                    StaticIcon::NoUpdatesAvailable
                } else {
                    StaticIcon::UpdatesAvailable
                }
                .into()
            })
            .color(theme.text)
            .font_size(14),
        )
        .child(move || {
            let count = updates.with(|u| u.len());
            if count > 0 {
                Some(
                    text(move || updates.with(|u| u.len().to_string()))
                        .color(theme.text)
                        .font_size(13),
                )
            } else {
                None
            }
        })
}

/// Menu view: update list + action buttons
pub fn menu_view(
    data: UpdatesDataSignals,
    svc: Service<UpdatesCmd>,
    close_menu: impl Fn() + 'static + Clone,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let updates = data.updates;
    let is_checking = data.is_checking;
    let svc_update = svc.clone();
    let close_menu_update = close_menu.clone();

    container()
        .width(fill())
        .layout(Flex::column().spacing(8))
        // Update list or "Up to date" message
        .child(move || {
            let list = updates.with(|u| u.clone());
            if list.is_empty() {
                return Some(
                    container()
                        .padding(8)
                        .child(text("Up to date ;)").color(theme.text).font_size(14)),
                );
            }
            let mut scroll = container()
                .width(fill())
                .height(at_most(300))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(6));
            for update in &list {
                let pkg = update.package.clone();
                let mut from = update.from.clone();
                from.truncate(18);
                let mut to = update.to.clone();
                to.truncate(18);
                let version_str = format!("{from} -> {to}");
                scroll = scroll.child(
                    container()
                        .width(fill())
                        .layout(Flex::column().spacing(2))
                        .child(text(pkg).color(theme.text).font_size(12))
                        .child(text(version_str).color(theme.primary).font_size(11)),
                );
            }
            Some(
                container().child(
                    expandable_panel()
                        .header(
                            text(move || {
                                format!("{} Updates available", updates.with(|u| u.len()))
                            })
                            .color(theme.text)
                            .font_size(14),
                        )
                        .body(scroll),
                ),
            )
        })
        // Divider
        .child(
            container()
                .width(fill())
                .height(1)
                .background(Color::rgba(1.0, 1.0, 1.0, 0.15)),
        )
        // Action buttons
        .child({
            let svc_update = svc_update.clone();
            let close_menu_update = close_menu_update.clone();
            menu_button("Update", None, move || {
                svc_update.send(UpdatesCmd::RunUpdate);
                close_menu_update();
            })
        })
        .child({
            let svc_check = svc.clone();
            menu_button_with_indicator("Check now", is_checking, move || {
                svc_check.send(UpdatesCmd::CheckNow);
            })
        })
}

fn menu_button(
    label: &'static str,
    _icon: Option<StaticIcon>,
    on_click: impl Fn() + 'static,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);
    container()
        .width(fill())
        .padding([6, 8])
        .corner_radius(8)
        .on_click(move || on_click())
        .on_hover(move |h| hovered.set(h))
        .background(move || {
            if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.1)
            } else {
                Color::TRANSPARENT
            }
        })
        .child(text(label).color(theme.text).font_size(14))
}

fn menu_button_with_indicator(
    label: &'static str,
    is_checking: Signal<bool>,
    on_click: impl Fn() + 'static,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);
    container()
        .width(fill())
        .padding([6, 8])
        .corner_radius(8)
        .on_click(move || on_click())
        .on_hover(move |h| hovered.set(h))
        .background(move || {
            if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.1)
            } else {
                Color::TRANSPARENT
            }
        })
        .layout(
            Flex::row()
                .main_alignment(MainAlignment::SpaceBetween)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(text(label).color(theme.text).font_size(14))
        .child(move || {
            if is_checking.get() {
                Some(icon().ic(StaticIcon::Refresh).color(theme.text).font_size(14))
            } else {
                None
            }
        })
}
