use clap::Parser;
use gui::launch_gui;
use tray::launch_tray;

mod comms;
mod gui;
mod tray;

fn main() {
    env_logger::init();

    let args = Args::parse();

    match args.gui {
        true => launch_gui(),
        false => launch_tray(),
    }
}

/// Args parsed from CLI.
#[derive(clap::Parser)]
struct Args {
    /// Whether to launch the GUI instead of the tray.
    #[arg(long)]
    gui: bool,
}
