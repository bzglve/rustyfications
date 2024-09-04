use std::{
    collections::HashMap,
    fs,
    sync::{LazyLock, Mutex},
};

use gtk::glib;
use log::LevelFilter as LogLevelFilter;
use serde::{Deserialize, Serialize};

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

    impl From<Edge> for gtk_layer_shell::Edge {
        fn from(value: Edge) -> Self {
            match value {
                Edge::Left => gtk_layer_shell::Edge::Left,
                Edge::Right => gtk_layer_shell::Edge::Right,
                Edge::Top => gtk_layer_shell::Edge::Top,
                Edge::Bottom => gtk_layer_shell::Edge::Bottom,
            }
        }
    }
}

mod defaults {
    use std::collections::HashMap;

    use super::{
        edge::{Edge, EdgeInfo},
        level_filter::LevelFilter,
    };

    pub fn expire_timeout() -> u64 {
        5000
    }

    pub fn new_on_top() -> bool {
        true
    }

    pub fn icon_size() -> i32 {
        72
    }

    pub fn log_level() -> LevelFilter {
        LevelFilter::Info
    }

    pub fn window_close_icon() -> String {
        "window-close".to_owned()
    }

    pub fn show_app_name() -> bool {
        false
    }

    pub fn window_size() -> (i32, i32) {
        (410, 30)
    }

    pub fn icon_redefines() -> HashMap<String, String> {
        HashMap::new()
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
    #[serde(default = "defaults::new_on_top")]
    pub new_on_top: bool,
    #[serde(default = "defaults::window_size")]
    pub window_size: (i32, i32),
    #[serde(default = "defaults::edges")]
    pub edges: HashMap<edge::Edge, edge::EdgeInfo>,
}

impl Config {
    pub fn new() -> Option<Self> {
        let config_path = Self::find_config_path()?;
        println!("Found config file {:?}", config_path);

        match ron::from_str::<Self>(&fs::read_to_string(&config_path).unwrap()) {
            Ok(config) if Self::validate_config(&config) => Some(config),
            Ok(_) => None,
            Err(e) => {
                eprintln!("{}", e);
                None
            }
        }
    }

    fn find_config_path() -> Option<std::path::PathBuf> {
        let user_config = glib::user_config_dir().join("rustyfications/config.ron");
        if user_config.exists() {
            return Some(user_config);
        }

        glib::system_config_dirs().iter().find_map(|dir| {
            let system_config = dir.join("rustyfications/config.ron");
            if system_config.exists() {
                Some(system_config)
            } else {
                None
            }
        })
    }

    fn validate_config(config: &Self) -> bool {
        if config.edges.contains_key(&edge::Edge::Left)
            && config.edges.contains_key(&edge::Edge::Right)
            || config.edges.contains_key(&edge::Edge::Top)
                && config.edges.contains_key(&edge::Edge::Bottom)
        {
            eprintln!("Using two opposite edges is not allowed");
            return false;
        }

        true
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            expire_timeout: defaults::expire_timeout(),
            new_on_top: defaults::new_on_top(),
            icon_size: defaults::icon_size(),
            log_level: defaults::log_level(),
            window_close_icon: defaults::window_close_icon(),
            show_app_name: defaults::show_app_name(),
            window_size: defaults::window_size(),
            icons_alias: defaults::icon_redefines(),
            edges: defaults::edges(),
        }
    }
}

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    Mutex::new({
        match Config::new() {
            Some(c) => c,
            None => {
                println!("Using default configuration");
                Config::default()
            }
        }
    })
});
