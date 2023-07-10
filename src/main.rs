use std::vec;

use reactive_gtk::{Button, Label, Node};
// use components::{Column, ColumnWidth, Columns};
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::MutableVec,
};
use gtk::{
    gdk::Display, prelude::*, ApplicationWindow, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use shell_bar::create_shell_bar;

mod battery;
mod clock;
mod launcher;
mod net;
mod reactive_gtk;
mod shell_bar;
mod system_info;
mod title;
mod updates;
mod utils;
mod workspaces;

fn activate(application: &gtk::Application) -> ApplicationWindow {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);
    window.set_default_size(-1, 34);

    // Before the window is first realized, set it up to be a layer surface
    gtk4_layer_shell::init_for_window(&window);

    // Display above normal windows
    gtk4_layer_shell::set_layer(&window, gtk4_layer_shell::Layer::Overlay);

    // Push other windows out of the way
    gtk4_layer_shell::auto_exclusive_zone_enable(&window);

    let display = gdk::Display::default().expect("Failed to get default display");
    let monitors = display.monitors();

    let mut target: Option<gdk::Monitor> = None;
    for m in monitors.iter::<gdk::Monitor>() {
        let monitor = m.unwrap();
        let connector = monitor.connector().unwrap();
        if connector == "eDP-1" {
            target = Some(monitor);
        }
    }

    if let Some(target) = target {
        gtk4_layer_shell::set_monitor(&window, &target);
    }

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (gtk4_layer_shell::Edge::Left, true),
        (gtk4_layer_shell::Edge::Right, true),
        (gtk4_layer_shell::Edge::Top, true),
        (gtk4_layer_shell::Edge::Bottom, false),
    ];

    for (anchor, state) in anchors {
        gtk4_layer_shell::set_anchor(&window, anchor, state);
    }

    window
}

#[tokio::main]
async fn main() {
    let application = gtk::Application::new(Some("malpenzibo.ashell"), Default::default());

    // let mut handlers: Vec<Handler<()>> = vec![];

    application.connect_startup(|app| {
        // The CSS "magic" happens here.
        let provider = CssProvider::new();
        provider.load_from_data(grass::include!("./src/style.scss"));
        // We give the CssProvided to the default screen so the CSS rules we added
        // can be applied to our window.
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let window = activate(app);
        window.set_css_classes(&["main"]);

        // let mut root = create_root2();
        let mut root = create_shell_bar();

        root.handlers.clear();

        window.set_child(Some(&root.component));

        app.connect_activate(move |_| window.present());
    });

    application.run();
}

fn compile_scss() -> String {
    let main_scss = include_str!("style.scss");
    let options = grass::Options::default().load_path(std::path::Path::new("src"));

    grass::from_string(main_scss, &options).expect("SCSS compilation failed")
}

fn create_root2() -> Node {
    let counter = Mutable::new(0);
    let my_vec: MutableVec<Node> = MutableVec::new();

    let counter1 = counter.clone();
    let increment = Button::default()
        .child(Label::default().text("Increment 1"))
        .on_click(move || {
            counter1.replace_with(|c| *c + 1);
        });

    let counter2 = counter.clone();
    let decrement = Button::default()
        .child(Label::default().text("Decrement 1"))
        .on_click(move || {
            counter2.replace_with(|c| *c - 1);
        });

    let my_vec1 = my_vec.clone();
    let remove_label = Button::default()
        .child(Label::default().text("Remove Label"))
        .on_click(move || {
            let mut vec = my_vec1.lock_mut();
            vec.remove(1);
        });

    my_vec.lock_mut().replace_cloned(vec![
        increment.into(),
        Label::default()
            .text_signal(counter.signal().map(|c| format!("Counter: {}", c)))
            .into(),
        decrement.into(),
        remove_label.into(),
    ]);

    reactive_gtk::Box::default()
        .orientation(reactive_gtk::Orientation::Horizontal)
        .children(vec![
            Label::default().text("Hello World!").into(),
            reactive_gtk::Box::default()
                .spacing(10)
                .children_signal(my_vec.signal_vec_cloned())
                .into(),
        ])
        .into()
}
