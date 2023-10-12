use shell_bar::create_shell_bar;

mod audio;
mod battery;
mod brightness;
mod clock;
mod components;
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
    // let surface = Surface::layer(true, (true, true, true, false), Some("eDP-1")).height(34);
    let surface = Surface::window();

    shell.run(surface, create_shell_bar);
}
