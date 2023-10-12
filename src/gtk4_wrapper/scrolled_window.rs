use super::Component;
use gtk4::Widget;
use leptos::{create_effect, MaybeSignal, SignalGet};

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

#[derive(Default, Clone)]
pub struct ScrolledWindow(gtk4::ScrolledWindow);

impl Component for ScrolledWindow {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

pub fn scrolled_window() -> ScrolledWindow {
    ScrolledWindow::default()
}

impl ScrolledWindow {
    pub fn child(self, child: impl Into<MaybeSignal<Widget>>) -> Self {
        create_effect({
            let scrolled_window = self.0.clone();
            let child = child.into();

            move |_| {
                scrolled_window.set_child(Some(&child.get()));
            }
        });

        self
    }

    pub fn vscrollbar_policy(self, policy: impl Into<MaybeSignal<PolicyType>>) -> Self {
        create_effect({
            let scrolled_window = self.0.clone();
            let policy = policy.into();

            move |_| {
                scrolled_window.set_vscrollbar_policy(policy.get().into());
            }
        });

        self
    }

    pub fn hscrollbar_policy(self, policy: impl Into<MaybeSignal<PolicyType>>) -> Self {
        create_effect({
            let scrolled_window = self.0.clone();
            let policy = policy.into();

            move |_| {
                scrolled_window.set_hscrollbar_policy(policy.get().into());
            }
        });

        self
    }
}

impl From<ScrolledWindow> for Widget {
    fn from(scrolled_window: ScrolledWindow) -> Self {
        scrolled_window.0.into()
    }
}
