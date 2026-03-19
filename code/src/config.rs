use std::fs;
use toml;
use serde::Deserialize;

use crate::data::*;


#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub layout: Option<Layout>,
    pub window: Option<Window>,
    pub feature: Option<Feature>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Feature {
    pub exp_rust_unwind: Option<bool>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Window {
    pub size: Option<(u16, u16)>,
    pub position: Option<(u16, u16)>,
    pub theme: Option<Theme>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone)]
pub enum Theme {
    light,
    dark,
    dracula,
    nord,
    solarized_light,
    Solarized_Dark,
    gruvbox_light,
    gruvbox_dark,
    catppuccin_latte,
    catppuccin_frappe,
    catppuccin_macchiato,
    catppuccin_mocha,
    tokyonight,
    tokyonight_storm,
    tokyonight_light,
    kanagawa_wave,
    kanagawa_dragon,
    kanagawa_lotus,
    moonfly,
    nightfly,
    oxocarbon,
    ferra
}

impl Theme {
    pub fn to_iced_theme(&self) -> iced::Theme{
        match self {
            Self::light => iced::Theme::Light,
            Self::dark => iced::Theme::Dark,
            Self::dracula => iced::Theme::Dracula,
            Self::nord => iced::Theme::Nord,
            Self::solarized_light => iced::Theme::SolarizedLight,
            Self::Solarized_Dark => iced::Theme::SolarizedDark,
            Self::gruvbox_light => iced::Theme::GruvboxLight,
            Self::gruvbox_dark => iced::Theme::GruvboxDark,
            Self::catppuccin_latte => iced::Theme::CatppuccinLatte,
            Self::catppuccin_frappe => iced::Theme::CatppuccinFrappe,
            Self::catppuccin_macchiato => iced::Theme::CatppuccinMacchiato,
            Self::catppuccin_mocha => iced::Theme::CatppuccinMocha,
            Self::tokyonight => iced::Theme::TokyoNight,
            Self::tokyonight_storm => iced::Theme::TokyoNightStorm,
            Self::tokyonight_light => iced::Theme::TokyoNightLight,
            Self::kanagawa_wave => iced::Theme::KanagawaWave,
            Self::kanagawa_dragon => iced::Theme::KanagawaDragon,
            Self::kanagawa_lotus => iced::Theme::KanagawaLotus,
            Self::moonfly => iced::Theme::Moonfly,
            Self::nightfly => iced::Theme::Nightfly,
            Self::oxocarbon => iced::Theme::Oxocarbon,
            Self::ferra => iced::Theme::Ferra
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Layout {
    pub status_bar: Option<bool>,
    pub sidebar_left: Option<bool>,
    pub sidebar_right: Option<bool>,
    pub panel: Option<bool>,
    pub panel_mode: Option<PanelMode>,
    pub panes: Option<Panes>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum PanelMode {
    middle,
    left,
    right,
    full
}

#[derive(Deserialize, Debug, Clone)]
pub struct Panes {
    pub main: Vec<Pane>,
    pub left: Vec<Pane>,
    pub right: Vec<Pane>,
    pub panel: Vec<Pane>
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone)]
pub enum Pane {
    memory,
    stack,
    code,
    assembly,
    registers,
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
        // UPDATE AND CHECK EVERY FIELD

        match &mut self.feature {
            None => self.feature = default.feature,
            Some(feature) => {
                let default = default.feature.unwrap();
                match feature.exp_rust_unwind {
                    None => feature.exp_rust_unwind = default.exp_rust_unwind,
                    Some(_) => ()
                }
            }
        }

        match &mut self.window {
            None => self.window = default.window,
            Some(window) => {
                let default = default.window.unwrap();
                match window.size {
                    None => window.size = default.size,
                    Some(_) => ()
                }

                match window.position {
                    None => window.position = default.position,
                    Some(_) => ()
                }

                match window.theme {
                    None => window.theme = default.theme,
                    Some(_) => ()
                }
            }
        }

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
    }
}

pub fn load_config() -> Config {
    let path = format!("{}/.config/tbd/config.toml", std::env::home_dir().unwrap_or(std::path::PathBuf::new()).to_str().unwrap());
    match fs::read(path) {
        Ok(file) => {
            let config: Result<Config, toml::de::Error> = toml::from_slice(&file);
            match config {
                Ok(mut config) => {config.merge(Config::default()); config},
                Err(err) => {crate::window::Dialog::error(&format!("Config syntax error: {}", err), Some("Config Loading Error")); Config::default()}
            }
        }
        Err(_) => Config::default()
    }
}