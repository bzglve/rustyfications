use gtk::prelude::WidgetExt;
use gtk_layer_shell::{Edge, LayerShell};

use crate::{
    config::{edge::Edge as ConfigEdge, CONFIG},
    types::RuntimeData,
};

use super::window::Window;

pub fn init_layer_shell(window: &impl LayerShell) {
    window.init_layer_shell();

    let edges = CONFIG.lock().unwrap().edges.clone();
    let margins = CONFIG.lock().unwrap().margins.clone();
    let paddings = CONFIG.lock().unwrap().paddings.clone();

    window.set_anchor(Edge::Left, edges.contains(&ConfigEdge::Left));
    window.set_anchor(Edge::Right, edges.contains(&ConfigEdge::Right));
    window.set_anchor(Edge::Top, edges.contains(&ConfigEdge::Top));
    window.set_anchor(Edge::Bottom, edges.contains(&ConfigEdge::Bottom));

    for (edge, margin) in edges
        .iter()
        .zip(margins.iter().zip(paddings.iter()).map(|(m, p)| m + p))
    {
        window.set_margin((*edge).into(), margin);
    }
}

pub fn margins_update(runtime_data: RuntimeData) {
    let edges = CONFIG.lock().unwrap().edges.clone();
    let margins = CONFIG.lock().unwrap().margins.clone();
    let paddings = CONFIG.lock().unwrap().paddings.clone();

    let runtime_data = runtime_data.borrow();
    let windows = runtime_data.windows.iter();

    let new_on_top = CONFIG.lock().unwrap().new_on_top;
    let iter: Box<dyn Iterator<Item = (&u32, &Window)>> = if new_on_top {
        Box::new(windows.rev())
    } else {
        Box::new(windows)
    };

    let mut indent = paddings
        .iter()
        .zip(edges.iter())
        .find(|(_, e)| **e == ConfigEdge::Top || **e == ConfigEdge::Bottom)
        .map(|(p, _)| p)
        .cloned()
        .unwrap_or_default();
    for (_, window) in iter {
        if edges.contains(&ConfigEdge::Top) {
            window.inner.set_margin(Edge::Top, indent);
        } else if edges.contains(&ConfigEdge::Bottom) {
            window.inner.set_margin(Edge::Bottom, indent);
        }
        indent += window.inner.height()
            + margins
                .iter()
                .zip(edges.iter())
                .find(|(_, e)| **e == ConfigEdge::Left || **e == ConfigEdge::Right)
                .map(|(p, _)| p)
                .cloned()
                .unwrap_or_default();
    }
}

pub mod pixbuf {
    use std::path::PathBuf;

    use gtk::{gdk, gdk_pixbuf::Pixbuf, prelude::FileExt, IconLookupFlags, TextDirection};

    use crate::config::CONFIG;

    pub fn new_from_str(value: &str) -> Option<Pixbuf> {
        if PathBuf::from(value).is_absolute() {
            return Pixbuf::from_file(value).ok();
        } else {
            let itheme = gtk::IconTheme::for_display(&gdk::Display::default().unwrap());
            if itheme.has_icon(value) {
                let ipaint = itheme.lookup_icon(
                    value,
                    &["image-missing"],
                    CONFIG.lock().unwrap().icon_size,
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
