use std::{
    collections::HashMap,
    fs,
    sync::{LazyLock, Mutex},
};

use defaults::*;
use edge::Edge;
use gtk::glib;
use level_filter::LevelFilter;
use serde::{Deserialize, Serialize};

pub mod level_filter {
    use super::*;

    use log::LevelFilter as LogLevelFilter;

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
    #[serde(default = "expire_timeout")]
    pub expire_timeout: u64,

    #[serde(default = "new_on_top")]
    pub new_on_top: bool,

    #[serde(default = "icon_size")]
    pub icon_size: i32,

    #[serde(default = "log_level")]
    pub log_level: LevelFilter,

    #[serde(default = "window_close_icon")]
    pub window_close_icon: String,

    #[serde(default = "show_app_name")]
    pub show_app_name: bool,

    #[serde(default = "window_size")]
    pub window_size: (i32, i32),

    #[serde(default = "icon_redefines")]
    pub icon_redefines: HashMap<String, String>,

    #[serde(default = "edges")]
    pub edges: Vec<Edge>,

    #[serde(default = "margins")]
    pub margins: Vec<i32>,

    #[serde(default = "paddings")]
    pub paddings: Vec<i32>,
}

impl Config {
    pub fn new() -> Option<Config> {
        let path;

        let user_config = glib::user_config_dir()
            .join("rustyfications")
            .join("config.ron");

        if user_config.exists() {
            path = user_config;
        } else if let Some(system_config) = glib::system_config_dirs().first() {
            let system_config = system_config
                .to_path_buf()
                .join("rustyfications")
                .join("config.ron");
            if system_config.exists() {
                path = system_config;
            } else {
                return None;
            }
        } else {
            return None;
        }

        println!("Found config file {:?}", path);
        match ron::from_str::<Self>(&fs::read_to_string(path.clone()).unwrap()) {
            Ok(r) => {
                if r.edges.contains(&Edge::Left) && r.edges.contains(&Edge::Right)
                    || r.edges.contains(&Edge::Top) && r.edges.contains(&Edge::Bottom)
                {
                    eprintln!("Using two opposite edges is not allowed");
                    println!("Using default configuration");
                    return None;
                }
                Some(r)
            }
            Err(e) => {
                eprintln!("{}", e);
                println!("Using default configuration");
                None
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            expire_timeout: expire_timeout(),
            new_on_top: new_on_top(),
            icon_size: icon_size(),
            log_level: log_level(),
            window_close_icon: window_close_icon(),
            show_app_name: show_app_name(),
            window_size: window_size(),
            icon_redefines: icon_redefines(),
            edges: edges(),
            margins: margins(),
            paddings: paddings(),
        }
    }
}

pub static CONFIG: LazyLock<Mutex<Config>> =
    LazyLock::new(|| Mutex::new(Config::new().unwrap_or_default()));
