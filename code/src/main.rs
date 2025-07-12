/*
    This is the main file of the entire program
    It's responsible for all modules to be working together correctly
    It handles errors, logging as feedback to development and the user, also is used as specification for all of the modules
    It handles the runtime, window, and debugging routine
*/

use std::fs;
use nix;
use iced;

mod test;       // for testing and debbuging

// the following files go in sequence based on what they depend on (tracing need objdump, data needs trace, ui needs data ...)
mod objdump;    // ELF handling, reading, preparing, AND TRACING THE PROGRAM (trace.rs merily debugs it)
mod trace;      // debugging programs
mod data;       // data manipulation, reading
mod ui;         // user interface - communication with user, preferences, config ..., file selection, options
mod window;     // window and graphics handling

fn main() {
    test::test();
    println!("Hello, world!");
}
