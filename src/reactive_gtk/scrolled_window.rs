use super::{AsyncContext, MaybeSignal, Node, NodeBuilder, Subscription};
use gtk4::traits::WidgetExt;

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
    pub fn child<N: Into<Node>>(mut self, child: impl MaybeSignal<Option<N>>) -> Self {
        match child.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |child, ctx| {
                let mut child = child.map(|child| child.into());
                ctx.cancel();
                if let Some(child) = child.as_mut() {
                    ctx.consume(child.get_ctx());
                }
                let child_widget = child.as_ref().map(|child| child.get_widget().clone());
                widget.set_child(child_widget.as_ref());
            }
        }) {
            Subscription::Dynamic(sub) => {
                self.ctx.add_subscription(sub);
            }
            Subscription::Static(mut ctx) => {
                self.ctx.consume(&mut ctx);
            }
        };

        self
    }

    pub fn vscrollbar_policy(mut self, value: impl MaybeSignal<PolicyType>) -> Self {
        match value.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |value, ctx| {
                ctx.cancel();
                widget.set_vscrollbar_policy(value.into());
            }
        }) {
            Subscription::Dynamic(sub) => {
                self.ctx.add_subscription(sub);
            }
            Subscription::Static(mut ctx) => {
                self.ctx.consume(&mut ctx);
            }
        };

        self
    }

    pub fn hscrollbar_policy(mut self, value: impl MaybeSignal<PolicyType>) -> Self {
        match value.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |value, ctx| {
                ctx.cancel();
                widget.set_hscrollbar_policy(value.into());
            }
        }) {
            Subscription::Dynamic(sub) => {
                self.ctx.add_subscription(sub);
            }
            Subscription::Static(mut ctx) => {
                self.ctx.consume(&mut ctx);
            }
        };

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
