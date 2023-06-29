use std::{rc::Rc, vec};

use components::{simple, Node, Value, VecValue};
// use components::{Column, ColumnWidth, Columns};
use futures_signals::{
    signal::{Mutable, Signal, SignalExt},
    signal_vec::MutableVec,
};
use gtk::{
    gdk::Display, prelude::*, ApplicationWindow, CssProvider, Widget,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use spawner::Handle;

mod components;
mod spawner;

fn activate(application: &gtk::Application) -> ApplicationWindow {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);

    // Before the window is first realized, set it up to be a layer surface
    gtk4_layer_shell::init_for_window(&window);

    // Display above normal windows
    gtk4_layer_shell::set_layer(&window, gtk4_layer_shell::Layer::Overlay);

    // Push other windows out of the way
    gtk4_layer_shell::auto_exclusive_zone_enable(&window);

    // The margins are the gaps around the window's edges
    // Margins and anchors can be set like this...
    // gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Left, 40);
    // gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Right, 40);
    // gtk4_layer_shell::set_margin(&window, gtk4_layer_shell::Edge::Top, 30);

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (gtk4_layer_shell::Edge::Left, true),
        (gtk4_layer_shell::Edge::Right, true),
        (gtk4_layer_shell::Edge::Top, false),
        (gtk4_layer_shell::Edge::Bottom, true),
    ];

    for (anchor, state) in anchors {
        gtk4_layer_shell::set_anchor(&window, anchor, state);
    }

    window
}

fn main() {
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_startup(|app| {
        // The CSS "magic" happens here.
        let provider = CssProvider::new();
        provider.load_from_data(include_str!("style.scss"));
        // We give the CssProvided to the default screen so the CSS rules we added
        // can be applied to our window.
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let window = activate(app);
        window.set_css_classes(&["main"]);

        let mut root = create_root2();

        root.handlers.clear();

        window.set_child(Some(&root.component));

        app.connect_activate(move |_| window.present());

        // startup(app);
    });

    application.run();
}

fn create_root() -> Widget {
    let counter = Mutable::new(0);
    let just = Mutable::new(0);

    let left = gtk::Box::default();
    let center = gtk::Box::default();
    center.set_hexpand(true);
    let right = gtk::Box::default();

    let counter_label = gtk::Label::new(Some(&format!("{}", counter.get())));

    // Set up a widget
    let increment = gtk::Button::default();
    increment.set_label("Increment 1");
    let counter1 = counter.clone();
    increment.connect_clicked(move |e| {
        counter1.replace_with(|c| *c + 1);
    });
    left.append(&increment);

    let increment = gtk::Button::default();
    increment.set_label("Increment 2");
    let just1 = just.clone();
    increment.connect_clicked(move |e| {
        just1.replace_with(|c| *c + 1);
    });
    left.append(&increment);

    let decrement = gtk::Button::default();
    decrement.set_label("Decrement 1");
    let counter2 = counter.clone();
    decrement.connect_clicked(move |e| {
        counter2.replace_with(|c| *c - 1);
    });

    right.append(&decrement);

    let decrement = gtk::Button::default();
    decrement.set_label("Decrement 2");
    let just2 = just.clone();
    decrement.connect_clicked(move |e| {
        just2.replace_with(|c| *c - 1);
    });

    right.append(&decrement);

    // into_dom

    let container = gtk::Box::default();
    container.set_orientation(gtk::Orientation::Horizontal);
    container.append(&left);
    container.append(&center);
    container.append(&right);

    let elem = counter_label.clone();

    crate::spawner::spawn(just.signal().for_each(move |c| {
        println!("Just changed to {}", c);
        elem.set_margin_start(c);

        async {}
    }));

    let elem = counter_label.clone();

    crate::spawner::spawn(counter.signal().for_each(move |c| {
        println!("Counter changed to {}", c);
        elem.set_label(&format!("{}", c));

        async {}
    }));

    center.append(&counter_label);
    // gidle_future::spawn(test);
    // gidle_future::spawn(bb);

    container.into()
}

fn create_root2() -> Node {
    let counter = Mutable::new(0);
    let my_vec: MutableVec<Node> = MutableVec::new();

    let increment = gtk::Button::default();
    increment.set_label("Increment 1");
    let counter1 = counter.clone();
    increment.connect_clicked(move |e| {
        counter1.replace_with(|c| *c + 1);
    });

    let decrement = gtk::Button::default();
    decrement.set_label("Decrement 1");
    let counter2 = counter.clone();
    decrement.connect_clicked(move |e| {
        counter2.replace_with(|c| *c - 1);
    });

    let remove_label = gtk::Button::default();
    remove_label.set_label("Remove Label");
    let my_vec1 = my_vec.clone();
    remove_label.connect_clicked(move |e| {
        let mut vec = my_vec1.lock_mut();
        vec.remove(1);
    });

    my_vec.lock_mut().replace_cloned(vec![
        increment.into(),
        components::Label::default()
            .text(Value::Signal(
                counter.signal().map(|c| format!("Counter: {}", c)),
            ))
            .into(),
        decrement.into(),
        remove_label.into(),
    ]);

    components::Box::default()
        .spacing(simple(10))
        .children(VecValue::Signal(my_vec.signal_vec_cloned()))
        .into()
}
