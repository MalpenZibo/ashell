use super::{AsyncContext, IntoSignal, Node, NodeBuilder, AsStr};

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

pub struct Label {
    widget: gtk4::Label,
    ctx: AsyncContext,
}

pub fn label() -> Label {
    Label {
        widget: gtk4::Label::default(),
        ctx: AsyncContext::default(),
    }
}

impl Label {
    pub fn text<T: AsStr>(mut self, value: impl IntoSignal<T> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                let value = value.with_str(|s| s.to_string());
                widget.set_text(&value);
            }
        });

        self
    }

    pub fn ellipsize(mut self, value: impl IntoSignal<EllipsizeMode> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_ellipsize(value.into());
            }
        });

        self
    }

    pub fn text_halign(mut self, value: impl IntoSignal<TextAlign> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_xalign(value.into());
            }
        });

        self
    }

    pub fn text_valign(mut self, value: impl IntoSignal<TextAlign> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_yalign(value.into());
            }
        });

        self
    }
}

impl NodeBuilder for Label {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<Label> for Node {
    fn from(label: Label) -> Self {
        Node::new(label.widget.into(), label.ctx)
    }
}
