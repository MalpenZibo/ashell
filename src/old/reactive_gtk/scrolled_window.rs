use super::{AsyncContext, IntoSignal, Node, NodeBuilder};

#[derive(Copy, Clone)]
pub enum PolicyType {
    Always,
    Automatic,
    Never,
}

impl From<PolicyType> for gtk4::PolicyType {
    fn from(value: PolicyType) -> Self {
        match value {
            PolicyType::Always => gtk4::PolicyType::Always,
            PolicyType::Automatic => gtk4::PolicyType::Automatic,
            PolicyType::Never => gtk4::PolicyType::Never,
        }
    }
}

pub struct ScrolledWindow {
    widget: gtk4::ScrolledWindow,
    ctx: AsyncContext,
}

pub fn scrolled_window() -> ScrolledWindow {
    ScrolledWindow {
        widget: gtk4::ScrolledWindow::default(),
        ctx: AsyncContext::default(),
    }
}

impl ScrolledWindow {
    pub fn child<N: Into<Node>>(mut self, value: impl IntoSignal<Option<N>> + 'static) -> Self {
        self.ctx.subscribe_with_ctx(value, {
            let widget = self.widget.clone();
            move |value, ctx| {
                let mut value = value.map(|value| value.into());
                ctx.cancel();
                if let Some(value) = value.as_mut() {
                    ctx.consume(value.get_ctx());
                }
                let value_widget = value.as_ref().map(|value| value.get_widget().clone());
                widget.set_child(value_widget.as_ref());
            }
        });

        self
    }

    pub fn vscrollbar_policy(mut self, value: impl IntoSignal<PolicyType> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_vscrollbar_policy(value.into());
            }
        });

        self
    }

    pub fn hscrollbar_policy(mut self, value: impl IntoSignal<PolicyType> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_hscrollbar_policy(value.into());
            }
        });

        self
    }
}

impl NodeBuilder for ScrolledWindow {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<ScrolledWindow> for Node {
    fn from(scrolled_window: ScrolledWindow) -> Self {
        Node::new(scrolled_window.widget.into(), scrolled_window.ctx)
    }
}
