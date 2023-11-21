use super::{AsStr, AsyncContext, IntoSignal};
use gtk4::traits::{GestureExt, WidgetExt};

pub struct Node {
    widget: gtk4::Widget,
    ctx: AsyncContext,
}

impl Node {
    pub fn new(widget: gtk4::Widget, ctx: AsyncContext) -> Self {
        Self { widget, ctx }
    }

    pub fn get_widget(&self) -> &gtk4::Widget {
        &self.widget
    }

    pub fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }
}

impl NodeBuilder for Node {
    fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
    }

    fn get_widget(&self) -> gtk4::Widget {
        self.widget.clone()
    }
}

#[derive(Copy, Clone)]
pub enum Align {
    Fill,
    Baseline,
    Start,
    Center,
    End,
}

impl From<Align> for gtk4::Align {
    fn from(value: Align) -> Self {
        match value {
            Align::Fill => gtk4::Align::Fill,
            Align::Baseline => gtk4::Align::Baseline,
            Align::Start => gtk4::Align::Start,
            Align::Center => gtk4::Align::Center,
            Align::End => gtk4::Align::End,
        }
    }
}

pub trait NodeBuilder: Sized {
    fn get_widget(&self) -> gtk4::Widget;

    fn get_ctx(&mut self) -> &mut AsyncContext;

    fn class<S: AsStr>(mut self, value: impl IntoSignal<Vec<S>> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                let value = value
                    .iter()
                    .map(|s| s.with_str(|s| s.to_string()))
                    .collect::<Vec<String>>();
                widget.set_css_classes(&value.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            }
        });

        self
    }

    fn vexpand(mut self, value: impl IntoSignal<bool> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_vexpand(value);
            }
        });

        self
    }

    fn hexpand(mut self, value: impl IntoSignal<bool> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_hexpand(value);
            }
        });

        self
    }

    fn valign(mut self, value: impl IntoSignal<Align> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_valign(value.into());
            }
        });

        self
    }

    fn halign(mut self, value: impl IntoSignal<Align> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_halign(value.into());
            }
        });

        self
    }

    fn active(mut self, value: impl IntoSignal<bool> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_sensitive(value);
            }
        });

        self
    }

    fn visible(mut self, value: impl IntoSignal<bool> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_visible(value);
            }
        });

        self
    }

    fn size(mut self, value: impl IntoSignal<(i32, i32)> + 'static) -> Self {
        let widget = self.get_widget();
        self.get_ctx().subscribe(value, {
            move |value| {
                widget.set_size_request(value.0, value.1);
            }
        });

        self
    }

    fn on_click(self, on_click: impl Fn() + 'static) -> Self {
        let gesture = gtk4::GestureClick::new();

        gesture.connect_released(move |gesture, _, _, _| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);

            on_click();
        });

        let widget = self.get_widget();

        widget.add_controller(gesture);

        self
    }
}
