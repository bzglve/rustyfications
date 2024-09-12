use std::{
    collections::HashMap,
    fs,
    sync::{LazyLock, Mutex},
};

use gtk::glib;
use layer::Layer;
use log::LevelFilter as LogLevelFilter;
use serde::{Deserialize, Serialize};

mod layer {
    use super::*;

    use gtk_layer_shell::Layer as GtkLayer;

    #[derive(Debug, Deserialize, Serialize, Clone, Copy)]
    pub enum Layer {
        Background,
        Bottom,
        Top,
        Overlay,
    }

    impl Default for Layer {
        fn default() -> Self {
            Self::Top
        }
    }

    impl From<Layer> for GtkLayer {
        fn from(value: Layer) -> Self {
            match value {
                Layer::Background => Self::Background,
                Layer::Bottom => Self::Bottom,
                Layer::Top => Self::Top,
                Layer::Overlay => Self::Overlay,
            }
        }
    }

    impl From<GtkLayer> for Layer {
        fn from(value: GtkLayer) -> Self {
            match value {
                GtkLayer::Background => Self::Background,
                GtkLayer::Bottom => Self::Bottom,
                GtkLayer::Top => Self::Top,
                GtkLayer::Overlay => Self::Overlay,
                _ => unreachable!(),
            }
        }
    }
}

mod level_filter {
    use super::*;

    #[derive(Debug, Deserialize, Serialize, Clone, Copy)]
    pub enum LevelFilter {
        Off,
        Error,
        Warn,
        Info,
        Debug,
        Trace,
    }

    impl Default for LevelFilter {
        fn default() -> Self {
            Self::Info
        }
    }

    impl From<LevelFilter> for LogLevelFilter {
        fn from(value: LevelFilter) -> Self {
            match value {
                LevelFilter::Off => Self::Off,
                LevelFilter::Error => Self::Error,
                LevelFilter::Warn => Self::Warn,
                LevelFilter::Info => Self::Info,
                LevelFilter::Debug => Self::Debug,
                LevelFilter::Trace => Self::Trace,
            }
        }
    }
}

pub mod edge {
    use gtk_layer_shell::Edge as GtkEdge;
    use serde::{Deserialize, Serialize};

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum Edge {
        Left,
        Right,
        Top,
        Bottom,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct EdgeInfo {
        #[serde(default)]
        pub margin: i32,
        #[serde(default)]
        pub padding: i32,
    }

    impl EdgeInfo {
        pub fn total_margin(&self) -> i32 {
            self.margin + self.padding
        }
    }

    impl From<Edge> for GtkEdge {
        fn from(value: Edge) -> Self {
            match value {
                Edge::Left => Self::Left,
                Edge::Right => Self::Right,
                Edge::Top => Self::Top,
                Edge::Bottom => Self::Bottom,
            }
        }
    }

    impl From<GtkEdge> for Edge {
        fn from(value: GtkEdge) -> Self {
            match value {
                GtkEdge::Left => Self::Left,
                GtkEdge::Right => Self::Right,
                GtkEdge::Top => Self::Top,
                GtkEdge::Bottom => Self::Bottom,
                _ => unreachable!(),
            }
        }
    }
}

mod defaults {
    use std::collections::HashMap;

    use super::{
        edge::{Edge, EdgeInfo},
        layer::Layer,
        level_filter::LevelFilter,
    };

    pub fn layer() -> Layer {
        Layer::default()
    }

    pub fn ignore_exclusive_zones() -> bool {
        bool::default()
    }

    pub fn expire_timeout() -> u64 {
        5000
    }

    pub fn reverse() -> bool {
        bool::default()
    }

    pub fn icon_size() -> i32 {
        72
    }

    pub fn log_level() -> LevelFilter {
        LevelFilter::default()
    }

    pub fn window_close_icon() -> String {
        "window-close".to_owned()
    }

    pub fn show_app_name() -> bool {
        bool::default()
    }

    pub fn stay_on_action() -> bool {
        bool::default()
    }

    pub fn window_size() -> (i32, i32) {
        (410, 30)
    }

    pub fn icon_redefines() -> HashMap<String, String> {
        [
            ("inline-reply".to_owned(), "mail-reply".to_owned()),
            ("dismiss".to_owned(), "window-close".to_owned()),
        ]
        .into()
    }

    pub fn edges() -> HashMap<Edge, EdgeInfo> {
        let mut val = HashMap::new();
        val.insert(
            Edge::Top,
            EdgeInfo {
                margin: 5,
                padding: 5,
            },
        );
        val.insert(
            Edge::Right,
            EdgeInfo {
                margin: 5,
                padding: 0,
            },
        );
        val
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default = "defaults::layer")]
    pub layer: Layer,
    #[serde(default = "defaults::ignore_exclusive_zones")]
    pub ignore_exclusive_zones: bool,
    #[serde(default = "defaults::expire_timeout")]
    pub expire_timeout: u64,
    #[serde(default = "defaults::icon_size")]
    pub icon_size: i32,
    #[serde(default = "defaults::log_level")]
    pub log_level: level_filter::LevelFilter,
    #[serde(default = "defaults::show_app_name")]
    pub show_app_name: bool,
    #[serde(default = "defaults::window_close_icon")]
    pub window_close_icon: String,
    #[serde(default = "defaults::icon_redefines")]
    pub icons_alias: HashMap<String, String>,
    #[serde(default = "defaults::reverse")]
    pub reverse: bool,
    #[serde(default = "defaults::stay_on_action")]
    pub stay_on_action: bool,
    #[serde(default = "defaults::window_size")]
    pub window_size: (i32, i32),
    #[serde(default = "defaults::edges")]
    pub edges: HashMap<edge::Edge, edge::EdgeInfo>,
}

impl Config {
    pub fn new() -> Option<Self> {
        let config_path = Self::find_config_path()?;
        println!("Found config file: {:?}", config_path);

        let config_string = fs::read_to_string(&config_path).ok()?;
        let config = ron::from_str::<Self>(&config_string);
        if let Err(e) = config {
            eprintln!("{}", e);
            return None;
        }
        let config = config.unwrap();

        if config.validate() {
            Some(config)
        } else {
            None
        }
    }

    fn find_config_path() -> Option<std::path::PathBuf> {
        let user_config = glib::user_config_dir().join("rustyfications/config.ron");
        if user_config.exists() {
            return Some(user_config);
        }

        glib::system_config_dirs().iter().find_map(|dir| {
            let system_config = dir.join("rustyfications/config.ron");
            system_config.exists().then_some(system_config)
        })
    }

    fn validate(&self) -> bool {
        if self.edges.contains_key(&edge::Edge::Left) && self.edges.contains_key(&edge::Edge::Right)
            || self.edges.contains_key(&edge::Edge::Top)
                && self.edges.contains_key(&edge::Edge::Bottom)
        {
            eprintln!("Using two opposite edges is not allowed");
            false
        } else {
            true
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layer: defaults::layer(),
            ignore_exclusive_zones: defaults::ignore_exclusive_zones(),
            expire_timeout: defaults::expire_timeout(),
            reverse: defaults::reverse(),
            icon_size: defaults::icon_size(),
            log_level: defaults::log_level(),
            window_close_icon: defaults::window_close_icon(),
            show_app_name: defaults::show_app_name(),
            stay_on_action: defaults::stay_on_action(),
            window_size: defaults::window_size(),
            icons_alias: defaults::icon_redefines(),
            edges: defaults::edges(),
        }
    }
}

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    Mutex::new(Config::new().unwrap_or_else(|| {
        println!("Using default configuration");
        Config::default()
    }))
});
