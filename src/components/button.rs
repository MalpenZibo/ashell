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

pub(crate) enum OnPress<'a, Message> {
    Direct(Message),
    Closure(Box<dyn Fn() -> Message + 'a>),
}

pub struct StyledButton<'a, Message> {
    label: Element<'a, Message>,
    icon: Option<(IconKind, IconPosition)>,
    kind: ButtonKind,
    hierarchy: ButtonHierarchy,
    size: ButtonSize,
    on_press: Option<OnPress<'a, Message>>,
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
        self.on_press = Some(OnPress::Direct(on_press));
        self
    }

    pub fn on_press_with(mut self, on_press: impl Fn() -> Message + 'a) -> Self {
        self.on_press = Some(OnPress::Closure(Box::new(on_press)));
        self
    }

    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press.map(OnPress::Direct);
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

        let (icon_element, icon_position) = match value.icon {
            Some((icon_kind, pos)) => (Some(icon_kind.to_text().size(icon_size)), Some(pos)),
            None => (None, None),
        };

        let content = match (icon_element, icon_position) {
            (Some(icon_el), Some(IconPosition::Before)) => container(
                row![icon_el, value.label]
                    .spacing(space.xs)
                    .align_y(Alignment::Center),
            )
            .into(),
            (Some(icon_el), Some(IconPosition::After)) => container(
                row![container(value.label).width(Length::Fill), icon_el,]
                    .spacing(space.xs)
                    .align_y(Alignment::Center),
            )
            .into(),
            _ => value.label,
        };

        let mut btn = button_fn(content)
            .padding(padding)
            .style(button_style)
            .height(value.height.unwrap_or(Length::Shrink));

        if let Some(width) = value.width {
            btn = btn.width(width);
        }

        let btn = match value.on_press {
            Some(OnPress::Direct(message)) => btn.on_press(message),
            Some(OnPress::Closure(closure)) => btn.on_press_with(closure),
            None => btn,
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
