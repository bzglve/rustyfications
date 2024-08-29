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

pub mod pixbuf {
    use std::path::PathBuf;

    use gtk::{gdk, gdk_pixbuf::Pixbuf, prelude::FileExt, IconLookupFlags, TextDirection};

    use crate::ICON_SIZE;

    pub fn new_from_str(value: &str) -> Option<Pixbuf> {
        if PathBuf::from(value).is_absolute() {
            return Pixbuf::from_file(value).ok();
        } else {
            let itheme = gtk::IconTheme::for_display(&gdk::Display::default().unwrap());
            if itheme.has_icon(value) {
                let ipaint = itheme.lookup_icon(
                    value,
                    &["image-missing"],
                    ICON_SIZE,
                    1,
                    TextDirection::None,
                    IconLookupFlags::empty(),
                );
                let image_path = ipaint
                    .file()
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                return Pixbuf::from_file(image_path).ok();
            }
        }
        None
    }

    pub fn crop_square(value: &Pixbuf) -> Pixbuf {
        let height = value.height();
        let width = value.width();
        let side = height.min(width);

        value.new_subpixbuf((width - side) / 2, (height - side) / 2, side, side)
    }
}
