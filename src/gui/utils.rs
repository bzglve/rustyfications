use gtk::prelude::WidgetExt;
use gtk_layer_shell::{Edge, LayerShell};

use crate::{types::RuntimeData, NEW_ON_TOP};

use super::window::Window;

pub fn init_layer_shell(window: &impl LayerShell) {
    window.init_layer_shell();

    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);

    window.set_margin(Edge::Right, 5);
}

pub fn margins_update(runtime_data: RuntimeData) {
    let runtime_data = runtime_data.borrow();
    let windows = runtime_data.windows.iter();

    let iter: Box<dyn Iterator<Item = (&u32, &Window)>> = if NEW_ON_TOP {
        Box::new(windows.rev())
    } else {
        Box::new(windows)
    };

    let mut indent = 5;
    for (_, window) in iter {
        window.inner.set_margin(Edge::Top, indent);
        indent += window.inner.height() + 5;
    }
}
