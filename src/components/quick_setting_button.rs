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
use iced_anim::{AnimationBuilder, transition::Easing};

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
    let target: f32 = if active { 1.0 } else { 0.0 };
    let icon_kind: IconKind = icon.into();
    let (space, font_size, animations_enabled) =
        use_theme(|theme| (theme.space, theme.font_size, theme.animations_enabled));

    let build_btn = move |t: f32,
                          title: String,
                          subtitle: Option<String>,
                          icon_kind: IconKind,
                          on_press: Msg,
                          with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>|
          -> Element<'a, Msg> {
        let (submenu_btn_style, settings_btn_style) = use_theme(|theme| {
            (
                theme.quick_settings_submenu_button_style(t),
                theme.quick_settings_button_style(t),
            )
        });
        let main_content = row!(
            icon_kind.to_text().size(font_size.lg),
            container(
                Column::with_capacity(2)
                    .push(text(title).size(font_size.sm))
                    .push(
                        subtitle
                            .map(|s| { text(s).wrapping(text::Wrapping::None).size(font_size.xs) })
                    )
                    .spacing(space.xxs)
            )
            .clip(true)
        )
        .spacing(space.xs)
        .padding(Padding::ZERO.left(space.xxs))
        .width(Length::Fill)
        .align_y(Alignment::Center);

        button(
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
        .height(Length::Fixed(50.))
        .into()
    };

    let btn: Element<'a, Msg> = if animations_enabled {
        AnimationBuilder::new(target, move |t| {
            build_btn(
                t,
                title.clone(),
                subtitle.clone(),
                icon_kind.clone(),
                on_press.clone(),
                with_submenu.clone(),
            )
        })
        .animation(Easing::EASE.quick())
        .into()
    } else {
        build_btn(target, title, subtitle, icon_kind, on_press, with_submenu)
    };

    if let Some(on_right_press) = on_right_press {
        MouseArea::new(btn).on_right_press(on_right_press).into()
    } else {
        btn
    }
}
