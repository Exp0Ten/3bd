use std::fs;
use toml;
use serde::Deserialize;

use crate::data::*;


#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    layout: Option<Layout>
}

#[derive(Deserialize, Debug, Clone)]
struct Layout {
    status_bar: Option<bool>,
    sidebar_left: Option<bool>,
    sidebar_right: Option<bool>,
    panel: Option<bool>,
    panel_mode: Option<PanelMode>,
    panes: Option<Panes>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone)]
enum PanelMode {
    middle,
    left,
    right,
    full
}

#[derive(Deserialize, Debug, Clone)]
struct Panes {
    main: Vec<Pane>,
    left: Vec<Pane>,
    right: Vec<Pane>,
    panel: Vec<Pane>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone)]
enum Pane {
    memory,
    stack,
    code,
    assembly,
    registers,
    variables,
    info,
    control,
    terminal
}

impl Default for Config {
    fn default() -> Self {
        let config: Self = toml::from_slice(&Asset::get("config.toml").unwrap().data).unwrap();
        config
    }
}

impl Config {
    pub fn merge(&mut self, default: Self) {
        // UPDATE FOR EVERY FIELD

        match &mut self.layout {
            None => self.layout = default.layout,
            Some(layout) => {
                let default = default.layout.unwrap();
                match layout.status_bar {
                    None => layout.status_bar = default.status_bar,
                    Some(_) => ()
                }
                
                match layout.sidebar_left {
                    None => layout.sidebar_left = default.sidebar_left,
                    Some(_) => ()
                }
                
                match layout.sidebar_right {
                    None => layout.sidebar_right = default.sidebar_right,
                    Some(_) => ()
                }
                
                match layout.panel {
                    None => layout.panel = default.panel,
                    Some(_) => ()
                }

                match layout.panel_mode {
                    None => layout.panel_mode = default.panel_mode,
                    Some(_) => ()
                }

                match layout.panes {
                    None => layout.panes = default.panes,
                    Some(_) => ()
                }
            },
        };

        //self.clone()  //derive Clone for this if needed
    }
}

pub fn load_config() -> Config { // TODO reprogram to "if let" statements instead of match
    let path = "$HOME/.config/tbd/config.toml";
    match fs::read(path) {
        Ok(file) => {
            let config: Result<Config, toml::de::Error> = toml::from_slice(&file);
            match config {
                Ok(mut config) => {config.merge(Config::default()); config},
                Err(_) => Config::default()
            }
        }
        Err(_) => Config::default()
    }
}