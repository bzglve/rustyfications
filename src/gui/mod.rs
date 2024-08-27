pub mod utils;
pub mod window;

use gtk::prelude::*;

pub fn build_ui(application: &gtk::Application) {
    let w = gtk::Window::new();
    w.set_application(Some(application));
}
