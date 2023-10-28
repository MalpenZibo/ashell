use super::{Node, AsyncContext, NodeBuilder, MaybeSignal, Subscription};
use gtk4::traits::{BoxExt, OrientableExt, WidgetExt};

#[derive(Copy, Clone)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl From<Orientation> for gtk4::Orientation {
    fn from(value: Orientation) -> Self {
        match value {
            Orientation::Horizontal => gtk4::Orientation::Horizontal,
            Orientation::Vertical => gtk4::Orientation::Vertical,
        }
    }
}

pub struct Container {
    widget: gtk4::Box,
    ctx: AsyncContext,
}

pub fn container() -> Container {
    Container {
        widget: gtk4::Box::default(),
        ctx: AsyncContext::default(),
    }
}

impl Container {
    pub fn orientation(mut self, orientation: impl MaybeSignal<Orientation>) -> Self {
        if let Some(handle) = orientation.subscribe({
            let widget = self.widget.clone();

            move |value| {
                widget.set_orientation(value.into());
            }
        }) {
            self.ctx.add_subscription(handle);
        }

        self
    }

    pub fn spacing(mut self, spacing: impl MaybeSignal<i32>) -> Self {
        if let Some(handle) = spacing.subscribe({
            let widget = self.widget.clone();

            move |value| {
                widget.set_spacing(value);
            }
        }) {
            self.ctx.add_subscription(handle);
        }

        self
    }

    pub fn homogeneous(mut self, homogeneous: impl MaybeSignal<bool>) -> Self {
        if let Some(handle) = homogeneous.subscribe({
            let widget = self.widget.clone();

            move |value| {
                widget.set_homogeneous(value);
            }
        }) {
            self.ctx.add_subscription(handle);
        }

        self
    }

    pub fn children(mut self, children: impl MaybeSignal<Vec<Node>>) -> Self {
        let sub = children.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |mut children, ctx| {
                ctx.cancel();
                while let Some(child) = widget.last_child() {
                    widget.remove(&child);
                }

                for mut child in children.drain(..) {
                    widget.append(child.get_widget());
                    ctx.consume(child.get_ctx());
                }
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

impl NodeBuilder for Container {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<Container> for Node {
    fn from(container: Container) -> Self {
        Node::new(container.widget.into(), container.ctx)
    }
}
