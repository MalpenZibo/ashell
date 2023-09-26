use gdk4::{Display, Monitor};
use gtk4::{prelude::*, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::LayerShell;
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

fn setup_layer(window: &gtk4::Window) {
    window.init_layer_shell();
    window.set_layer(gtk4_layer_shell::Layer::Overlay);

    window.auto_exclusive_zone_enable();

    let display = Display::default().expect("Failed to get default display");
    let monitors = display.monitors();

    let mut target: Option<Monitor> = None;
    for m in monitors.iter::<Monitor>() {
        let monitor = m.unwrap();
        let connector = monitor.connector().unwrap();
        if connector == "eDP-1" {
            target = Some(monitor);
        }
    }

    if let Some(target) = target {
        window.set_monitor(&target);
    }

    window.set_anchor(gtk4_layer_shell::Edge::Left, true);
    window.set_anchor(gtk4_layer_shell::Edge::Top, true);
    window.set_anchor(gtk4_layer_shell::Edge::Right, true);
}
