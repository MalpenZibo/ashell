use super::{AsyncContext, Node, NodeBuilder, IntoSignal};
use gtk4::traits::RangeExt;

pub struct Scale {
    widget: gtk4::Scale,
    ctx: AsyncContext,
}

pub fn scale() -> Scale {
    Scale {
        widget: gtk4::Scale::default(),
        ctx: AsyncContext::default(),
    }
}

impl Scale {
    pub fn range(self, value: (f64, f64)) -> Self {
        let (min, max) = value;
        self.widget.set_range(min, max);

        self
    }

    pub fn value(mut self, value: impl IntoSignal<f64> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_value(value);
            }
        });

        self
    }

    pub fn round_digits(mut self, value: impl IntoSignal<i32> + 'static) -> Self {
        self.ctx.subscribe(value, {
            let widget = self.widget.clone();
            move |value| {
                widget.set_round_digits(value);
            }
        });

        self
    }

    pub fn on_change(self, on_change: impl Fn(f64) + 'static) -> Self {
        self.widget.connect_value_changed(move |widget| {
            on_change(widget.value());
        });

        self
    }
}

impl NodeBuilder for Scale {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone().into()
    }
}

impl From<Scale> for Node {
    fn from(scale: Scale) -> Self {
        Node::new(scale.widget.into(), scale.ctx)
    }
}
