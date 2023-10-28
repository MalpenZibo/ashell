use super::{AsyncContext, MaybeSignal, Node, NodeBuilder};

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
    pub fn text(mut self, value: impl MaybeSignal<String>) -> Self {
        if let Some(sub) = value.subscribe({
            let widget = self.widget.clone();
            move |value| {
                widget.set_text(&value);
            }
        }) {
            self.ctx.add_subscription(sub);
        }

        self
    }

    pub fn ellipsize(mut self, mode: impl MaybeSignal<EllipsizeMode>) -> Self {
        if let Some(sub) = mode.subscribe({
            let widget = self.widget.clone();
            move |mode| {
                widget.set_ellipsize(mode.into());
            }
        }) {
            self.ctx.add_subscription(sub);
        }

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
