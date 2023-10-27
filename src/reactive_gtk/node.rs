use super::{AsyncContext, MaybeSignal};
use gtk4::traits::{GestureExt, WidgetExt};

pub struct Node {
    widget: gtk4::Widget,
    ctx: AsyncContext,
}

impl Node {
    pub fn get_widget(&self) -> &gtk4::Widget {
        &self.widget
    }

    pub fn get_ctx(&mut self) -> &mut AsyncContext {
        &mut self.ctx
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

trait NodeBuilder: Sized {
    fn get_widget(&self) -> &gtk4::Widget;

    fn get_ctx(&mut self) -> &mut AsyncContext;

    fn class(mut self, value: impl MaybeSignal<Vec<String>>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_css_classes(
                    value
                        .iter()
                        .map(AsRef::as_ref)
                        .collect::<Vec<&str>>()
                        .as_slice(),
                );
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn vexpand(mut self, value: impl MaybeSignal<bool>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_vexpand(value);
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn hexpand(mut self, value: impl MaybeSignal<bool>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_hexpand(value);
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn valign(mut self, value: impl MaybeSignal<Align>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_valign(value.into());
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn halign(mut self, value: impl MaybeSignal<Align>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_halign(value.into());
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn active(mut self, value: impl MaybeSignal<bool>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_sensitive(value);
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn visible(mut self, value: impl MaybeSignal<bool>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_visible(value);
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

        self
    }

    fn size(mut self, value: impl MaybeSignal<(i32, i32)>) -> Self {
        let sub = value.subscribe({
            let widget = self.get_widget().clone();
            move |value| {
                widget.set_size_request(value.0, value.1);
            }
        });

        if let Some(sub) = sub {
            self.get_ctx().add_subscription(sub);
        }

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
