use guido::prelude::*;

use crate::components::{
    ButtonKind, StaticIcon, button, divider, icon, module_item, toggle_button,
};
use crate::modules::{MenuCtx, MenuType, close_menu_fn, menu_toggle};
use crate::services::tray::dbus::{Layout, LayoutProps};
use crate::services::tray::{TrayCmd, TrayItem, start_tray_service};
use crate::services::xdg_icons::XdgIcon;
use crate::theme::ThemeColors;

pub type TrayHandle = (RwSignal<Vec<TrayItem>>, Service<TrayCmd>);

pub fn create() -> TrayHandle {
    let items = create_signal(Vec::<TrayItem>::new());
    let svc = start_tray_service(items.writer());
    (items, svc)
}

// ── Bar view ─────────────────────────────────────────────────────────────────

/// One clickable icon per tray item; click opens the item's DBusMenu popup.
/// Rows are keyed by service name, so per-item state (widget refs, menu
/// bindings) survives unrelated tray updates; icon changes flow through a
/// memo inside the row.
pub fn view(items: RwSignal<Vec<TrayItem>>, svc: Service<TrayCmd>, menu: MenuCtx) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let blocklist =
        with_context::<crate::config::Config, _>(|c| c.tray.blocklist.clone()).unwrap_or_default();

    container()
        .height(fill())
        .layout(Flex::row().cross_alignment(CrossAlignment::Center))
        .children(keyed(
            move || {
                items.with(|list| {
                    list.iter()
                        .map(|i| i.name.clone())
                        .filter(|name| !blocklist.iter().any(|re| re.0.is_match(name)))
                        .collect::<Vec<_>>()
                })
            },
            |name| hash_key(name),
            move |name| tray_button(name, items, svc, menu, theme),
        ))
}

/// Stable identity key for keyed rows.
fn hash_key(value: impl std::hash::Hash) -> u64 {
    use std::hash::{DefaultHasher, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn tray_button(
    name: String,
    items: RwSignal<Vec<TrayItem>>,
    svc: Service<TrayCmd>,
    menu: MenuCtx,
    theme: ThemeColors,
) -> impl Widget {
    let wr = create_widget_ref();
    let content = {
        let name = name.clone();
        let svc = svc;
        move || menu_view(name.clone(), items, svc, close_menu_fn(menu)).into_any()
    };
    let toggle = menu_toggle(MenuType::Tray(name.clone()), wr, menu, content);

    // Memo narrows the rebuild: the icon widget is replaced only when the
    // icon itself changes, not on every tray update
    let item_icon = create_memo(move || {
        items.with(|l| {
            l.iter()
                .find(|i| i.name == name)
                .and_then(|i| i.icon.clone())
        })
    });

    container().widget_ref(wr).height(fill()).child(
        module_item()
            .on_click(toggle)
            .child(move || Some(icon_view(item_icon.get(), theme))),
    )
}

fn icon_view(item_icon: Option<XdgIcon>, theme: ThemeColors) -> AnyWidget {
    match item_icon {
        Some(XdgIcon::Image(src)) => image(src).height(14).into_any(),
        Some(XdgIcon::Svg(src)) => image(src).width(16).height(16).into_any(),
        None => icon()
            .kind(StaticIcon::Point)
            .color(theme.text)
            .font_size(14)
            .into_any(),
    }
}

// ── Menu view ────────────────────────────────────────────────────────────────

fn is_separator(layout: &Layout) -> bool {
    layout.1.type_.as_deref() == Some("separator")
}

fn is_visible(layout: &Layout) -> bool {
    layout.1.visible != Some(false)
}

/// Hidden entries dropped, leading/trailing/doubled separators collapsed.
fn renderable_children(children: &[Layout]) -> impl Iterator<Item = &Layout> {
    let end = children
        .iter()
        .rposition(|child| is_visible(child) && !is_separator(child))
        .map_or(0, |last| last + 1);

    children[..end]
        .iter()
        .filter(|child| is_visible(child))
        .scan(true, |prev_sep, child| {
            let sep = is_separator(child);
            let keep = !(sep && *prev_sep);
            *prev_sep = sep;
            Some(keep.then_some(child))
        })
        .flatten()
}

/// DBusMenu rendered from the item's current layout. The closure re-runs —
/// and the menu content is rebuilt — whenever the layout signal or the open
/// submenus change; the popup re-measures and repositions itself when the
/// height changes.
fn menu_view(
    name: String,
    items: RwSignal<Vec<TrayItem>>,
    svc: Service<TrayCmd>,
    close: Callback,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let open_submenus = create_signal(Vec::<i32>::new());

    container().width(fill()).child(move || {
        let menu_layout =
            items.with(|list| list.iter().find(|i| i.name == name).map(|i| i.menu.clone()))?;

        let mut col = container()
            .width(fill())
            .height(at_most(600))
            .scrollable(ScrollAxis::Vertical)
            .layout(Flex::column().spacing(4));
        for voice in renderable_children(&menu_layout.2) {
            col = col.child(menu_voice(&name, voice, open_submenus, &svc, close, theme));
        }
        Some(col)
    })
}

fn menu_voice(
    name: &str,
    layout: &Layout,
    open_submenus: RwSignal<Vec<i32>>,
    svc: &Service<TrayCmd>,
    close: Callback,
    theme: ThemeColors,
) -> AnyWidget {
    let id = layout.0;
    match &layout.1 {
        // Checkmark entry: toggle + label, stays open on click
        LayoutProps {
            label: Some(label),
            toggle_type: Some(toggle_type),
            toggle_state: Some(state),
            children_display: None,
            ..
        } if toggle_type == "checkmark" => {
            let on_toggle = {
                let svc = *svc;
                let name = name.to_owned();
                move || svc.send(TrayCmd::MenuClick(name.clone(), id))
            };
            button()
                .kind(ButtonKind::Transparent)
                .fill_width(true)
                .content(
                    container()
                        .width(fill())
                        .layout(
                            Flex::row()
                                .spacing(8)
                                .main_alignment(MainAlignment::SpaceBetween)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(text(label.replace('_', "")).color(theme.text).font_size(13))
                        .child(toggle_button().active(*state > 0).on_toggle({
                            let on_toggle = on_toggle.clone();
                            move || on_toggle()
                        })),
                )
                .on_click(on_toggle)
                .into_any()
        }
        // Submenu: header toggles expansion, children render indented below
        LayoutProps {
            children_display: Some(display),
            label: Some(label),
            ..
        } if display == "submenu" => {
            let is_open = open_submenus.with(|s| s.contains(&id));
            let header = button()
                .kind(ButtonKind::Transparent)
                .fill_width(true)
                .content(
                    container()
                        .width(fill())
                        .layout(
                            Flex::row()
                                .main_alignment(MainAlignment::SpaceBetween)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(text(label.replace('_', "")).color(theme.text).font_size(13))
                        .child(
                            icon()
                                .kind(if is_open {
                                    StaticIcon::MenuOpen
                                } else {
                                    StaticIcon::MenuClosed
                                })
                                .color(theme.text)
                                .font_size(12),
                        ),
                )
                .on_click({
                    let svc = *svc;
                    let name = name.to_owned();
                    move || {
                        let mut opening = false;
                        open_submenus.update(|s| {
                            if let Some(pos) = s.iter().position(|i| *i == id) {
                                s.remove(pos);
                            } else {
                                s.push(id);
                                opening = true;
                            }
                        });
                        if opening {
                            // Lazy apps only populate submenu children now
                            svc.send(TrayCmd::AboutToShow(name.clone(), id));
                        }
                    }
                });

            let mut col = container()
                .width(fill())
                .layout(Flex::column().spacing(4))
                .child(header);
            if is_open {
                let mut body = container()
                    .width(fill())
                    .padding(Padding::all(0.0).left(12.0))
                    .layout(Flex::column().spacing(4));
                for child in renderable_children(&layout.2) {
                    body = body.child(menu_voice(name, child, open_submenus, svc, close, theme));
                }
                col = col.child(body);
            }
            col.into_any()
        }
        // Plain entry: click sends the event and closes the menu
        LayoutProps {
            label: Some(label), ..
        } if !label.is_empty() => {
            let svc = *svc;
            let name = name.to_owned();
            button()
                .kind(ButtonKind::Transparent)
                .fill_width(true)
                .content(text(label.replace('_', "")).color(theme.text).font_size(13))
                .on_click(move || {
                    svc.send(TrayCmd::MenuClick(name.clone(), id));
                    close.run();
                })
                .into_any()
        }
        LayoutProps { type_: Some(t), .. } if t == "separator" => divider().into_any(),
        _ => container().into_any(),
    }
}
