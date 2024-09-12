use gtk::prelude::WidgetExt;
use gtk_layer_shell::{Edge, LayerShell};

use crate::{
    config::{
        edge::{Edge as ConfigEdge, EdgeInfo},
        CONFIG,
    },
    types::RuntimeData,
};

use super::window::Window;

pub fn init_layer_shell(window: &impl LayerShell) {
    window.init_layer_shell();

    window.set_layer(CONFIG.lock().unwrap().layer.into());
    if CONFIG.lock().unwrap().ignore_exclusive_zones {
        window.set_exclusive_zone(-1);
    }

    let edges = CONFIG.lock().unwrap().edges.clone();

    for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
        window.set_anchor(edge, edges.contains_key(&ConfigEdge::from(edge)));
    }

    edges.iter().for_each(|(config_edge, edge_info)| {
        window.set_margin((*config_edge).into(), edge_info.total_margin());
    });
}

pub fn margins_update(runtime_data: RuntimeData) {
    let edges = CONFIG.lock().unwrap().edges.clone();

    let runtime_data = runtime_data.borrow();
    let windows_iter: Box<dyn Iterator<Item = (&u32, &Window)>> = if !CONFIG.lock().unwrap().reverse
    {
        Box::new(runtime_data.windows.iter().rev())
    } else {
        Box::new(runtime_data.windows.iter())
    };

    let mut top_bottom_indent = edges
        .get(&ConfigEdge::Top)
        .or_else(|| edges.get(&ConfigEdge::Bottom))
        .map_or(0, |edge_info| edge_info.padding);

    for (_, window) in windows_iter {
        if edges.contains_key(&ConfigEdge::Top) {
            window.inner.set_margin(Edge::Top, top_bottom_indent);
        } else if edges.contains_key(&ConfigEdge::Bottom) {
            window.inner.set_margin(Edge::Bottom, top_bottom_indent);
        }

        top_bottom_indent += window.inner.height()
            + edges
                .get(&ConfigEdge::Left)
                .or_else(|| edges.get(&ConfigEdge::Right))
                .unwrap_or(&EdgeInfo::default())
                .margin;
    }
}

pub mod pixbuf {
    use std::path::PathBuf;

    use gtk::{gdk, gdk_pixbuf::Pixbuf, prelude::FileExt, IconLookupFlags, TextDirection};

    use crate::config::CONFIG;

    pub fn new_from_str(value: &str) -> Option<Pixbuf> {
        if PathBuf::from(value).is_absolute() {
            return Pixbuf::from_file(value).ok();
        }

        let icon_theme = gtk::IconTheme::for_display(&gdk::Display::default()?);

        if icon_theme.has_icon(value) {
            let icon_info = icon_theme.lookup_icon(
                value,
                &["image-missing"],
                CONFIG.lock().unwrap().icon_size,
                1,
                TextDirection::None,
                IconLookupFlags::empty(),
            );

            if let Some(image_path) = icon_info.file().and_then(|file| file.path()) {
                return Pixbuf::from_file(image_path.to_string_lossy().as_ref()).ok();
            }
        }

        None
    }

    pub fn crop_square(pixbuf: &Pixbuf) -> Pixbuf {
        let side = pixbuf.height().min(pixbuf.width());
        pixbuf.new_subpixbuf(
            (pixbuf.width() - side) / 2,
            (pixbuf.height() - side) / 2,
            side,
            side,
        )
    }
}
