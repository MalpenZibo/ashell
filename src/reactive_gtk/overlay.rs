use super::{AsyncContext, Node, NodeBuilder, IntoSignal};
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
    pub fn children(mut self, value: impl IntoSignal<Vec<Node>> + 'static) -> Self {
        self.ctx.subscribe_with_ctx(value, {
            let widget = self.widget.clone();
            move |mut value, ctx| {
                ctx.cancel();
                while let Some(child) = widget.last_child() {
                    widget.remove_overlay(&child);
                }

                for mut value in value.drain(..) {
                    let value_widget = value.get_widget();
                    widget.add_overlay(value_widget);
                    widget.set_measure_overlay(value_widget, true);
                    ctx.consume(value.get_ctx());
                }
            }
        });

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
