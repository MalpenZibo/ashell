use super::{AsyncContext, MaybeSignal, Node, NodeBuilder, Subscription};

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
    pub fn start(mut self, child: impl MaybeSignal<Option<Node>>) -> Self {
        let sub = child.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |mut child: Option<Node>, ctx| {
                ctx.cancel();
                if let Some(child) = child.as_mut() {
                    ctx.consume(child.get_ctx());
                }
                let child_widget = child.as_ref().map(|child| child.get_widget().clone());
                widget.set_start_widget(child_widget.as_ref());
            }
        });

        match sub {
            Subscription::Dynamic(sub) => {
                self.ctx.add_subscription(sub);
            }
            Subscription::Static(mut ctx) => {
                self.ctx.consume(&mut ctx);
            }
        };

        self
    }

    pub fn center(mut self, child: impl MaybeSignal<Option<Node>>) -> Self {
        let sub = child.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |mut child: Option<Node>, ctx| {
                ctx.cancel();
                if let Some(child) = child.as_mut() {
                    ctx.consume(child.get_ctx());
                }
                let child_widget = child.as_ref().map(|child| child.get_widget().clone());
                widget.set_center_widget(child_widget.as_ref());
            }
        });

        match sub {
            Subscription::Dynamic(sub) => {
                self.ctx.add_subscription(sub);
            }
            Subscription::Static(mut ctx) => {
                self.ctx.consume(&mut ctx);
            }
        };

        self
    }

    pub fn end(mut self, child: impl MaybeSignal<Option<Node>>) -> Self {
        let sub = child.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |mut child: Option<Node>, ctx| {
                ctx.cancel();
                if let Some(child) = child.as_mut() {
                    ctx.consume(child.get_ctx());
                }
                let child_widget = child.as_ref().map(|child| child.get_widget().clone());
                widget.set_end_widget(child_widget.as_ref());
            }
        });

        match sub {
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