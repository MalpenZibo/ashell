use super::{AsyncContext, MaybeSignal, Node, NodeBuilder};
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
    pub fn range(mut self, value: impl MaybeSignal<(f64, f64)>) -> Self {
        if let Some(sub) = value.subscribe({
            let widget = self.widget.clone();
            move |(min, max)| {
                widget.set_range(min, max);
            }
        }) {
            self.ctx.add_subscription(sub);
        }

        self
    }

    pub fn value(mut self, value: impl MaybeSignal<f64>) -> Self {
        if let Some(sub) = value.subscribe({
            let widget = self.widget.clone();
            move |value| {
                widget.set_value(value);
            }
        }) {
            self.ctx.add_subscription(sub);
        }

        self
    }

    pub fn round_digits(mut self, value: impl MaybeSignal<i32>) -> Self {
        if let Some(sub) = value.subscribe({
            let widget = self.widget.clone();
            move |value| {
                widget.set_round_digits(value);
            }
        }) {
            self.ctx.add_subscription(sub);
        }

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
