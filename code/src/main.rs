/*
    This is the main file of the entire program
    It's responsible for all modules to be working together correctly
    It handles errors, logging as feedback to development and the user, also is used as specification for all of the modules
    It handles the runtime, window, and debugging routine
*/

mod test;       // for testing and debbuging

// the following files go in sequence based on what they depend on (tracing need objdump, data needs trace, ui needs data ...)
mod object;     // file handling, reading, preparing (+ from dwarf, import source files)
mod dwarf;      // local variables, call stack, background line tracking
mod trace;      // debugging programs (eg. backend for the ui and debug functions)
mod data;       // data manipulation, reading (formatting, )
mod config;     // handling config and setting files located in ~/.config/tbd/
mod ui;         // user interface - communicating with user and graphics
mod style;      // styling functions
mod window;     // window handle
// mod keyboard;        // keyboard shortcuts handling (in use with graphics)

// CONSTANT FLAGS

//TODO - EXECUTABLE FLAGS AND CMD LINE HANDLING


// MAIN

fn main() {
    println!("hii");
    test::test();
    //window::run_app().expect("Not working");
}
