#![windows_subsystem = "windows"]

use std::env;
use std::process;
use system76_keyboard_configurator::{
    application,
    daemon::DaemonServer,
};

fn main() {
    let args = env::args().collect::<Vec<_>>();
    for arg in args.iter().skip(1) {
        if arg.as_str() == "--daemon" {
            let server = DaemonServer::new_stdio().expect("Failed to create server");
            server.run().expect("Failed to run server");
            return;
        }
    }

    process::exit(application::run(args));
}
