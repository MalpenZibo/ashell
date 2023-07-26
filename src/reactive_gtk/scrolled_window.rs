use futures_signals::signal::{Signal, SignalExt};

use super::{
    spawner::{spawn, Handle},
    Component, Node,
};

pub enum PolicyType {
    Always,
    Automatic,
    Never,
}

impl From<PolicyType> for gtk::PolicyType {
    fn from(value: PolicyType) -> Self {
        match value {
            PolicyType::Always => gtk::PolicyType::Always,
            PolicyType::Automatic => gtk::PolicyType::Automatic,
            PolicyType::Never => gtk::PolicyType::Never,
        }
    }
}

#[derive(Default, Clone)]
pub struct ScrolledWindow {
    element: gtk::ScrolledWindow,
    handlers: Vec<Handle<()>>,
}

impl ScrolledWindow {
    pub fn child<C: Into<Node>>(mut self, child: C) -> Self {
        let mut child = child.into();
        self.element.set_child(Some(&child.component));

        self.handlers.extend_from_slice(&child.handlers);
        child.handlers.clear();

        self
    }

    pub fn child_signal<C: Into<Node>, S: Signal<Item = C> + 'static>(mut self, child: S) -> Self {
        let element = self.element.clone();

        let handler = spawn(child.for_each(move |child| {
            let child = child.into();
            element.set_child(Some(&child.component));

            async {}
        }));

        self.handlers.push(handler);

        self
    }

    pub fn vscrollbar_policy(self, policy: PolicyType) -> Self {
        self.element.set_vscrollbar_policy(policy.into());

        self
    }

    pub fn vscrollbar_policy_signal<S: Signal<Item = PolicyType> + 'static>(
        mut self,
        policy: S,
    ) -> Self {
        let element = self.element.clone();

        let handler = spawn(policy.for_each(move |policy| {
            element.set_vscrollbar_policy(policy.into());

            async {}
        }));

        self.handlers.push(handler);

        self
    }

    pub fn hscrollbar_policy(self, policy: PolicyType) -> Self {
        self.element.set_hscrollbar_policy(policy.into());

        self
    }

    pub fn hscrollbar_policy_signal<S: Signal<Item = PolicyType> + 'static>(
        mut self,
        policy: S,
    ) -> Self {
        let element = self.element.clone();

        let handler = spawn(policy.for_each(move |policy| {
            element.set_hscrollbar_policy(policy.into());

            async {}
        }));

        self.handlers.push(handler);

        self
    }
}

impl Component for ScrolledWindow {
    fn get_widget(&self) -> gtk::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<ScrolledWindow> for Node {
    fn from(value: ScrolledWindow) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
