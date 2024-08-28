use std::rc::Rc;

pub use css::load_css;

use crate::{
    dbus::{IFace, IFaceRef, Reason},
    margins_update,
    types::RuntimeData,
};

mod css {
    use std::collections::HashMap;

    use gtk::{gdk, CssProvider};

    pub fn load_css() {
        let provider = CssProvider::new();

        let text = css_glob_export_string();
        let theme_colors = css_glob_export_colors(&text);

        let borders = theme_colors.get("borders").unwrap_or(&"gray");
        let theme_base_color = theme_colors.get("theme_base_color").unwrap_or(&"gray");

        provider.load_from_data(&format!(
            "
    #notification {{
      border: 1pt solid {borders};
      border-radius: 5pt;
    }}
    
    #notification.hover {{
      background-color: {theme_base_color};
    }}",
        ));

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    pub fn css_glob_export_string() -> String {
        let settings = gtk::Settings::default().unwrap();
        let theme_name = settings.gtk_theme_name().unwrap();
        let pref_dark = settings.is_gtk_application_prefer_dark_theme();
        let css_provider = gtk::CssProvider::new();
        css_provider.load_named(&theme_name, if pref_dark { Some("dark") } else { None });

        css_provider.to_string()
    }

    pub fn css_glob_export_colors(text: &str) -> HashMap<&str, &str> {
        text.lines()
            .map(|line| line.trim().trim_end_matches(";"))
            .filter(|line| line.starts_with("@define-color"))
            .filter_map(|line| {
                line.trim_start_matches("@define-color")
                    .trim()
                    .split_once(" ")
            })
            .collect()
    }
}

pub async fn close_hook(
    id: u32,
    died_from: Reason,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) {
    IFace::notification_closed(iface.signal_context(), id, died_from)
        .await
        .unwrap();

    runtime_data.borrow_mut().windows.remove(&id);
    margins_update(runtime_data.clone());
}
