/*
    This is the main file of the entire program
    It loads the config and runs the application
*/

mod data;       // Globals Definition and Handling
mod object;     // file handling, reading, preparing, (also responsible for terminal setup and running the Tracee)
mod dwarf;      // local variables, call stack, background line tracking
mod trace;      // debugging programs (eg. backend for the ui and debug functions)
mod config;     // handling config and setting files located in ~/.config/tbd/
mod ui;         // user interface - communicating with user and graphics
mod style;      // styling functions
mod window;     // window handle

// MAIN

use crate::data::*;

fn main() {
    CONFIG.sets(config::load_config());
    window::run_app().expect("Not working");
}