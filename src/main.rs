use reactive_gtk::{App, Surface};
use shell_bar::create_shell_bar;

mod audio;
mod battery;
mod clock;
mod launcher;
mod net;
mod reactive_gtk;
mod screenshare;
mod shell_bar;
mod system_info;
mod title;
mod updates;
mod utils;
mod workspaces;

#[tokio::main]
async fn main() {
    let shell = App::new(Some("malpenzibo.ashell"));
    let surface = Surface::layer(true, (true, true, true, false), Some("eDP-1")).height(34);

    shell.run(surface, create_shell_bar);
}
