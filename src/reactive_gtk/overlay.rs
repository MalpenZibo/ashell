use super::{AsyncContext, MaybeSignal, Node, NodeBuilder, Subscription};
use gtk4::traits::WidgetExt;

pub struct Overlay {
    widget: gtk4::Overlay,
    ctx: AsyncContext,
}

pub fn overlay() -> Overlay {
    Overlay {
        widget: gtk4::Overlay::default(),
        ctx: AsyncContext::default(),
    }
}

impl Overlay {
    pub fn children(mut self, children: impl MaybeSignal<Vec<Node>>) -> Self {
        match children.subscribe_with_ctx({
            let widget = self.widget.clone();
            move |mut children, ctx| {
                ctx.cancel();
                while let Some(child) = widget.last_child() {
                    widget.remove_overlay(&child);
                }

                for mut child in children.drain(..) {
                    let child_widget = child.get_widget();
                    widget.add_overlay(child_widget);
                    widget.set_measure_overlay(child_widget, true);
                    ctx.consume(child.get_ctx());
                }
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

impl NodeBuilder for Overlay {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<Overlay> for Node {
    fn from(overlay: Overlay) -> Self {
        Node::new(overlay.widget.into(), overlay.ctx)
    }
}
