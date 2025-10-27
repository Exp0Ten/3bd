use toml;
use toml::value;

use iced::window::Settings;

use crate::ui;


struct Config {
    window: Settings,
    palletes: Vec<Pallete>,
    layouts: Vec<Layout>,
    data: Vec<DataCategory>,
    //keyboard: Vec<Keybind>
}

struct Pallete {

}

struct Layout {

}

struct DataCategory {
    name: String,
    datatypes: Vec<DataType>
}

struct DataType {
    name: String,
    structure: value::Array,
    byte_length: usize
}