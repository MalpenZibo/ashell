use crate::{components::icons::IconKind, theme::use_theme};
use iced::{
    Alignment, Element, Length,
    widget::{button as button_fn, container, row, text},
};

pub trait IntoButtonContent<'a, Message: 'static> {
    fn into_content(self) -> Element<'a, Message>;
}

impl<'a, Message: 'static> IntoButtonContent<'a, Message> for &'a str {
    fn into_content(self) -> Element<'a, Message> {
        text(self).align_y(Alignment::Center).into()
    }
}

impl<'a, Message: 'static> IntoButtonContent<'a, Message> for String {
    fn into_content(self) -> Element<'a, Message> {
        text(self).align_y(Alignment::Center).into()
    }
}

impl<'a, Message: 'static> IntoButtonContent<'a, Message> for Element<'a, Message> {
    fn into_content(self) -> Element<'a, Message> {
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonKind {
    Solid,
    #[default]
    Transparent,
    Outline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonHierarchy {
    Primary,
    #[default]
    Secondary,
    Danger,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconPosition {
    #[default]
    Before,
    After,
}

pub struct StyledButton<'a, Message> {
    label: Element<'a, Message>,
    icon: Option<(IconKind, IconPosition)>,
    kind: ButtonKind,
    hierarchy: ButtonHierarchy,
    size: ButtonSize,
    on_press: Option<Message>,
    width: Option<Length>,
    height: Option<Length>,
}

impl<'a, Message: 'static + Clone> StyledButton<'a, Message> {
    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn hierarchy(mut self, hierarchy: ButtonHierarchy) -> Self {
        self.hierarchy = hierarchy;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn icon(mut self, icon: impl Into<IconKind>, position: IconPosition) -> Self {
        self.icon = Some((icon.into(), position));
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }
}

impl<'a, Message: 'static + Clone> From<StyledButton<'a, Message>> for Element<'a, Message> {
    fn from(value: StyledButton<'a, Message>) -> Self {
        let (space, font_size, button_style) = use_theme(|theme| {
            (
                theme.space,
                theme.font_size,
                theme.button_style(value.kind, value.hierarchy),
            )
        });

        let (padding, icon_size) = match value.size {
            ButtonSize::Small => ([space.xxs, space.sm], font_size.sm),
            ButtonSize::Medium => ([space.xs, space.md], font_size.md),
            ButtonSize::Large => ([space.sm, space.xl], font_size.lg),
        };

        let content: Element<'a, Message> = match value.icon {
            Some((icon_kind, IconPosition::Before)) => container(
                row![icon_kind.to_text().size(icon_size), value.label]
                    .spacing(space.xs)
                    .align_y(Alignment::Center),
            )
            .into(),
            Some((icon_kind, IconPosition::After)) => container(
                row![
                    container(value.label).width(Length::Fill),
                    icon_kind.to_text().size(icon_size),
                ]
                .spacing(space.xs)
                .align_y(Alignment::Center),
            )
            .into(),
            None => value.label,
        };

        let mut btn = button_fn(content)
            .padding(padding)
            .style(button_style)
            .height(value.height.unwrap_or(Length::Shrink));

        if let Some(width) = value.width {
            btn = btn.width(width);
        }

        let btn = if let Some(message) = value.on_press {
            btn.on_press(message)
        } else {
            btn
        };

        btn.into()
    }
}

pub fn styled_button<'a, Message: 'static + Clone>(
    content: impl IntoButtonContent<'a, Message>,
) -> StyledButton<'a, Message> {
    StyledButton {
        label: content.into_content(),
        icon: None,
        kind: ButtonKind::default(),
        hierarchy: ButtonHierarchy::default(),
        size: ButtonSize::default(),
        on_press: None,
        width: None,
        height: None,
    }
}
