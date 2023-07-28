use gdk::Display;
use gtk::{prelude::*, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use reactive_gtk::Surface;
use relm4::prelude::*;
use shell_bar::create_shell_bar;

mod audio;
mod battery;
mod brightness;
mod clock;
mod launcher;
mod net;
mod reactive_gtk;
mod screenshare;
mod settings;
mod shell_bar;
mod system_info;
mod title;
mod updates;
mod utils;
mod workspaces;

#[tokio::main]
async fn main() {
    use reactive_gtk::{App, Surface};
    let shell = App::new(Some("malpenzibo.ashell"));
    let surface = Surface::layer(true, (true, true, true, false), Some("eDP-1")).height(34);

    shell.run(surface, create_shell_bar);
}

// fn main() {
//     let app = RelmApp::new("malpenzibo.ashell");
//     app.run::<App>(0);
// }

fn setup_layer(window: &gtk::Window) {
    gtk4_layer_shell::init_for_window(window);
    gtk4_layer_shell::set_layer(window, gtk4_layer_shell::Layer::Overlay);

    gtk4_layer_shell::auto_exclusive_zone_enable(window);

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
        gtk4_layer_shell::set_monitor(window, &target);
    }

    gtk4_layer_shell::set_anchor(window, gtk4_layer_shell::Edge::Left, true);
    gtk4_layer_shell::set_anchor(window, gtk4_layer_shell::Edge::Top, true);
    gtk4_layer_shell::set_anchor(window, gtk4_layer_shell::Edge::Right, true);
}

struct App {
    counter: u8,
}

#[derive(Debug)]
enum Msg {
    Increment,
    Decrement,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = u8;
    type Input = Msg;
    type Output = ();

    view! {
        gtk::Window {
            set_title: Some("Simple app"),
            set_default_size: (-1, 34),

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,
                set_margin_all: 5,

                gtk::Button {
                    set_label: "Increment",
                    connect_clicked => Msg::Increment,
                },

                gtk::Button {
                    set_label: "Decrement",
                    connect_clicked => Msg::Decrement,
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Counter: {}", model.counter),
                    set_margin_all: 5,
                }
            }
        }
    }

    // Initialize the component.
    fn init(
        counter: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = App { counter };

        setup_layer(root);

        let provider = CssProvider::new();
        provider.load_from_data(grass::include!("./src/style.scss"));
        // We give the CssProvided to the default screen so the CSS rules we added
        // can be applied to our window.
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Insert the code generation of the view! macro here
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            Msg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            Msg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
    }
}
