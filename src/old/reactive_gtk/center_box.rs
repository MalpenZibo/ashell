use super::{AsyncContext, IntoSignal, Node, NodeBuilder};

pub struct Centerbox {
    widget: gtk4::CenterBox,
    ctx: AsyncContext,
}

pub fn centerbox() -> Centerbox {
    Centerbox {
        widget: gtk4::CenterBox::new(),
        ctx: AsyncContext::default(),
    }
}

impl Centerbox {
    pub fn start<N: Into<Node>>(mut self, value: impl IntoSignal<Option<N>> + 'static) -> Self {
        self.ctx.subscribe_with_ctx(value, {
            let widget = self.widget.clone();
            move |value, ctx| {
                let mut value = value.map(|child| child.into());
                ctx.cancel();
                if let Some(value) = value.as_mut() {
                    ctx.consume(value.get_ctx());
                }
                let value_widget = value.as_ref().map(|child| child.get_widget().clone());
                widget.set_start_widget(value_widget.as_ref());
            }
        });

        self
    }

    pub fn center<N: Into<Node>>(mut self, value: impl IntoSignal<Option<N>> + 'static) -> Self {
        self.ctx.subscribe_with_ctx(value, {
            let widget = self.widget.clone();
            move |value, ctx| {
                let mut value = value.map(|child| child.into());
                ctx.cancel();
                if let Some(value) = value.as_mut() {
                    ctx.consume(value.get_ctx());
                }
                let value_widget = value.as_ref().map(|child| child.get_widget().clone());
                widget.set_center_widget(value_widget.as_ref());
            }
        });

        self
    }

    pub fn end<N: Into<Node>>(mut self, value: impl IntoSignal<Option<N>> + 'static) -> Self {
        self.ctx.subscribe_with_ctx(value, {
            let widget = self.widget.clone();
            move |value, ctx| {
                let mut value = value.map(|child| child.into());
                ctx.cancel();
                if let Some(value) = value.as_mut() {
                    ctx.consume(value.get_ctx());
                }
                let value_widget = value.as_ref().map(|child| child.get_widget().clone());
                widget.set_end_widget(value_widget.as_ref());
            }
        });

        self
    }
}

impl NodeBuilder for Centerbox {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<Centerbox> for Node {
    fn from(centerbox: Centerbox) -> Self {
        Node::new(centerbox.widget.into(), centerbox.ctx)
    }
}
