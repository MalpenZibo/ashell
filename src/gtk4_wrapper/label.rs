use gtk4::Widget;
use leptos::{create_effect, MaybeSignal, SignalGet};

use super::Component;

#[derive(Clone, Copy)]
pub enum EllipsizeMode {
    None,
    Start,
    Middle,
    End,
}

impl From<EllipsizeMode> for gtk4::pango::EllipsizeMode {
    fn from(mode: EllipsizeMode) -> Self {
        match mode {
            EllipsizeMode::None => gtk4::pango::EllipsizeMode::None,
            EllipsizeMode::Start => gtk4::pango::EllipsizeMode::Start,
            EllipsizeMode::Middle => gtk4::pango::EllipsizeMode::Middle,
            EllipsizeMode::End => gtk4::pango::EllipsizeMode::End,
        }
    }
}

#[derive(Clone, Copy)]
pub enum TextAlign {
    Start,
    Center,
    End,
}

impl From<TextAlign> for f32 {
    fn from(align: TextAlign) -> Self {
        match align {
            TextAlign::Start => 0.,
            TextAlign::Center => 0.5,
            TextAlign::End => 1.,
        }
    }
}

#[derive(Default, Clone)]
pub struct Label(gtk4::Label);

impl Component for Label {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

pub fn label() -> Label {
    Label::default()
}

impl Label {
    pub fn text(self, text: impl Into<MaybeSignal<String>> + 'static) -> Self {
        create_effect({
            let label = self.0.clone();
            let text = text.into();

            move |_| {
                label.set_text(text.get().as_str());
            }
        });

        self
    }

    pub fn ellipsize(self, ellipsize: impl Into<MaybeSignal<EllipsizeMode>>) -> Self {
        create_effect({
            let label = self.0.clone();
            let ellipsize = ellipsize.into();

            move |_| {
                label.set_ellipsize(ellipsize.get().into());
            }
        });

        self
    }

    pub fn text_halign(self, align: impl Into<MaybeSignal<TextAlign>>) -> Self {
        create_effect({
            let label = self.0.clone();
            let align = align.into();

            move |_| {
                label.set_xalign(align.get().into());
            }
        });

        self
    }

    pub fn text_valign(self, align: impl Into<MaybeSignal<TextAlign>>) -> Self {
        create_effect({
            let label = self.0.clone();
            let align = align.into();

            move |_| {
                label.set_yalign(align.get().into());
            }
        });

        self
    }
}

impl From<Label> for Widget {
    fn from(label: Label) -> Self {
        label.0.into()
    }
}
