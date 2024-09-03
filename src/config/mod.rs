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

    #[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum Edge {
        Left,
        Right,
        Top,
        Bottom,
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

    use super::{edge::Edge, level_filter::LevelFilter};

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

    pub fn edges() -> Vec<Edge> {
        vec![Edge::Top, Edge::Right]
    }

    pub fn margins() -> Vec<i32> {
        vec![5, 5]
    }

    pub fn paddings() -> Vec<i32> {
        vec![5, 0]
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default = "defaults::expire_timeout")]
    pub expire_timeout: u64,
    #[serde(default = "defaults::new_on_top")]
    pub new_on_top: bool,
    #[serde(default = "defaults::icon_size")]
    pub icon_size: i32,
    #[serde(default = "defaults::log_level")]
    pub log_level: level_filter::LevelFilter,
    #[serde(default = "defaults::window_close_icon")]
    pub window_close_icon: String,
    #[serde(default = "defaults::show_app_name")]
    pub show_app_name: bool,
    #[serde(default = "defaults::window_size")]
    pub window_size: (i32, i32),
    #[serde(default = "defaults::icon_redefines")]
    pub icon_redefines: HashMap<String, String>,
    #[serde(default = "defaults::edges")]
    pub edges: Vec<edge::Edge>,
    #[serde(default = "defaults::margins")]
    pub margins: Vec<i32>,
    #[serde(default = "defaults::paddings")]
    pub paddings: Vec<i32>,
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
        if config.edges.contains(&edge::Edge::Left) && config.edges.contains(&edge::Edge::Right)
            || config.edges.contains(&edge::Edge::Top) && config.edges.contains(&edge::Edge::Bottom)
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
            icon_redefines: defaults::icon_redefines(),
            edges: defaults::edges(),
            margins: defaults::margins(),
            paddings: defaults::paddings(),
        }
    }
}

pub static CONFIG: LazyLock<Mutex<Config>> =
    LazyLock::new(|| Mutex::new(Config::new().unwrap_or_default()));
