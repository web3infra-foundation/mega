extern crate gtk;

use std::sync::LazyLock;

use adw::prelude::*;
use application::MonobeanApplication;
use gtk::{gio, glib};

mod application;
mod mega;
mod window;
mod components;
mod config;

const APP_ID: &str = "org.Web3Infrastructure.Monobean";
const APP_NAME: &str = "Monobean";
const PREFIX: &str = "/org/Web3Infrastructure/Monobean";

pub static CONTEXT: LazyLock<glib::MainContext> = LazyLock::new(glib::MainContext::default);

fn main() -> glib::ExitCode {
    if let Some(cargo_dir) = std::option_env!("CARGO_MANIFEST_DIR") {
        std::env::set_current_dir(cargo_dir).expect("Failed to set workspace dir");
    }

    let resources = gio::Resource::load("Monobean.gresource").expect("Failed to load resources");
    gio::resources_register(&resources);

    glib::set_application_name(APP_NAME);

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = MonobeanApplication::new(APP_ID, &gio::ApplicationFlags::empty());
    let _guard = CONTEXT.acquire().unwrap();

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    std::process::exit(app.run().into());
}
