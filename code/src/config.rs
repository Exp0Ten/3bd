use toml;

use serde::Deserialize;

use crate::window;
use crate::data::*;


#[derive(Deserialize, Debug)]
pub struct Config {
    layout: Option<Layout>
}

#[derive(Deserialize, Debug)]
struct Layout {
    status_bar: Option<bool>,
    sidebar_left: Option<bool>,
    sidebar_right: Option<bool>,
    panel: Option<bool>,
    panel_mode: Option<PanelMode>,
    panes: Option<Panes>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug)]
enum PanelMode {
    middle,
    left,
    right,
    full
}

#[derive(Deserialize, Debug)]
struct Panes {
    main: Vec<Pane>,
    left: Vec<Pane>,
    right: Vec<Pane>,
    panel: Vec<Pane>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug)]
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
    pub fn test() -> Self {
        let config: Self = toml::from_slice(&Asset::get("test.toml").unwrap().data).unwrap();
        config
    }

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