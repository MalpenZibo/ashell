use crate::{
    components::icons::{IconButtonSize, IconKind, StaticIcon, icon_button},
    modules::settings::SubMenu,
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Column, MouseArea, Row, button, container, row, text},
};

#[allow(clippy::too_many_arguments)]
pub fn quick_setting_button<'a, Msg: Clone + 'static>(
    theme: &'a AshellTheme,
    icon: impl Into<IconKind>,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    on_right_press: Option<Msg>,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
) -> Element<'a, Msg> {
    let main_content = row!(
        icon.into().to_text().size(theme.font_size.lg),
        container(
            Column::with_capacity(2)
                .push(text(title).size(theme.font_size.sm))
                .push(subtitle.map(|s| {
                    text(s)
                        .wrapping(text::Wrapping::None)
                        .size(theme.font_size.xs)
                }))
                .spacing(theme.space.xxs)
        )
        .clip(true)
    )
    .spacing(theme.space.xs)
    .padding(Padding::ZERO.left(theme.space.xxs))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let btn = button(
        Row::with_capacity(2)
            .push(main_content)
            .push(with_submenu.map(|(menu_type, submenu, msg)| {
                icon_button(
                    theme,
                    if Some(menu_type) == submenu {
                        StaticIcon::Close
                    } else {
                        StaticIcon::RightChevron
                    },
                )
                .on_press(msg)
                .size(IconButtonSize::Small)
                .style(theme.quick_settings_submenu_button_style(active))
            }))
            .spacing(theme.space.xxs)
            .align_y(Alignment::Center)
            .height(Length::Fill),
    )
    .padding([theme.space.xxs, theme.space.xs])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(theme.quick_settings_button_style(active))
    .width(Length::Fill)
    .height(Length::Fixed(50.));

    if let Some(on_right_press) = on_right_press {
        MouseArea::new(btn).on_right_press(on_right_press).into()
    } else {
        btn.into()
    }
}
