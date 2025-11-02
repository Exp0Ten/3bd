use toml;

use crate::window;
use crate::data::*;

pub fn get_app() -> Option<window::App> {
    //Here i read from global or call a generic function to read to toml config file to retrieve the default App state
    Some(window::App::new()) // for now (Im probably also gonna return only specific fields as well to limit imports)
}

/* # I have no idea how to use this but want to
trait BuiltIn {

}
*/
/*
pub struct Config {
    name: String,
    window: Settings,
    theme: Theme,
    layout: Layout,
    keybinds: Keybinds,
}

struct Layout {

}

struct Keybinds {

}

impl Config {
    fn new(name: &str, ) -> Self {

    }
}

impl Default for Config {
    fn default() -> Self {
        
    }
}


impl Default for Layout {
    fn default() -> Self {
        
    }
}

impl Default for Keybinds {
    fn default() -> Self {
        
    }
}
*/