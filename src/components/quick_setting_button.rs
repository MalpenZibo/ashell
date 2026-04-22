use crate::{
    components::{
        ButtonSize,
        icons::{IconKind, StaticIcon, icon_button},
    },
    modules::settings::SubMenu,
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Column, MouseArea, Row, button, container, row, text},
};

#[allow(clippy::too_many_arguments)]
pub fn quick_setting_button<'a, Msg: Clone + 'static>(
    icon: impl Into<IconKind>,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    on_right_press: Option<Msg>,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
) -> Element<'a, Msg> {
    let (space, font_size, submenu_btn_style, settings_btn_style) = use_theme(|theme| {
        (
            theme.space,
            theme.font_size,
            theme.quick_settings_submenu_button_style(active),
            theme.quick_settings_button_style(active),
        )
    });

    let main_content = row!(
        icon.into().to_text().size(font_size.lg),
        container(
            Column::with_capacity(2)
                .push(text(title).size(font_size.sm))
                .push(
                    subtitle.map(|s| { text(s).wrapping(text::Wrapping::None).size(font_size.xs) })
                )
                .spacing(space.xxs)
        )
        .clip(true)
    )
    .spacing(space.xs)
    .padding(Padding::ZERO.left(space.xxs))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let btn = button(
        Row::with_capacity(2)
            .push(main_content)
            .push(with_submenu.map(|(menu_type, submenu, msg)| {
                icon_button(if Some(menu_type) == submenu {
                    StaticIcon::Close
                } else {
                    StaticIcon::RightChevron
                })
                .on_press(msg)
                .size(ButtonSize::Small)
                .style(submenu_btn_style)
            }))
            .spacing(space.xxs)
            .align_y(Alignment::Center)
            .height(Length::Fill),
    )
    .padding([space.xxs, space.xs])
    .on_press(on_press)
    .style(settings_btn_style)
    .width(Length::Fill)
    .height(Length::Fixed(50.));

    if let Some(on_right_press) = on_right_press {
        MouseArea::new(btn).on_right_press(on_right_press).into()
    } else {
        btn.into()
    }
}
