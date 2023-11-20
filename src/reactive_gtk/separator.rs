use super::{AsyncContext, IntoSignal, Node, NodeBuilder, Orientation};
use gtk4::traits::OrientableExt;

pub struct Separator {
    widget: gtk4::Separator,
    ctx: AsyncContext,
}

pub fn separator() -> Separator {
    Separator {
        widget: gtk4::Separator::default(),
        ctx: AsyncContext::default(),
    }
}

impl Separator {
    pub fn orientation(mut self, value: impl IntoSignal<Orientation> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_orientation(value.into());
            }
        });

        self
    }
}

impl NodeBuilder for Separator {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<Separator> for Node {
    fn from(separator: Separator) -> Self {
        Node::new(separator.widget.into(), separator.ctx)
    }
}
