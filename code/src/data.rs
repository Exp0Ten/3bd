use std::{
    sync::{Mutex, MutexGuard},
    path,
    fs,
};

// internal import
use crate::{
    config,
    dwarf,
    trace,
    ui
};

use rust_embed::Embed; // Embeding assets into the binary to produce a single relocatable binary file


/// FILE: data.rs - Embeding the Assets and Handling the Global Variables

#[derive(Embed)]
#[folder = "assets/"]
#[exclude = "*.md"]
#[exclude = "*/LICENSE"]
pub struct Asset;

// Global Data Handle

pub static mut DATA: Vec<u8> = Vec::new(); // The executable file contents
// we use an unsafe method of storing the file data in order to produce references of 'static lifetime
// this is important for storing the other global variables

// this is a safe and controlled method, using Mutex (which prevents from multiple references to exist at once and therefore data racing between threads)
// Option<T> lets us define the globals as empty on startup
type Global<T> = Mutex<Option<T>>;

pub static CONFIG: Global<config::Config> = empty();
pub static FILE: Global<std::path::PathBuf> = empty();
pub static STDIO: Global<std::os::fd::OwnedFd> = empty();
pub static PID: Global<nix::unistd::Pid> = empty();
pub static PROC_PATH: Global<path::PathBuf> = empty();
pub static MAPS: Global<Vec<trace::MemoryMap>> = empty();
pub static EXEC_SHIFT: Global<u64> = empty();
pub static MEMORY: Global<fs::File> = empty();
pub static DWARF: Global<dwarf::DwarfSections> = empty();
pub static EHFRAME: Global<dwarf::EhFrame> = empty();
pub static ENDIAN: Global<dwarf::Endian> = empty();
pub static SOURCE: Global<dwarf::SourceMap> = empty();
pub static LINES: Global<dwarf::LineAddresses> = empty();
pub static FUNCTIONS: Global<dwarf::FunctionIndex> = empty();
pub static BREAKPOINTS: Global<trace::Breakpoints> = empty();
pub static REGISTERS: Global<nix::libc::user_regs_struct> = empty();

pub static SAVED_STATE: Global<SavedState> = empty(); // used for storing the sidebar panes

#[derive(Clone)]
pub struct SavedState {
    pub left_sidebar: (iced::widget::pane_grid::Configuration<ui::Pane>, f32),
    pub right_sidebar: (iced::widget::pane_grid::Configuration<ui::Pane>, f32),
    pub panel: (iced::widget::pane_grid::Configuration<ui::Pane>, f32),
    pub main: Option<iced::widget::pane_grid::Configuration<ui::Pane>>
}

// init function for Global
const fn empty<T>() -> Global<T> {Mutex::new(None)}

pub trait ImplGlobal<T> {
    fn access(&self) -> MutexGuard<'_, Option<T>>;
    fn sets(&self, new: T);
    fn none(&self);
}

impl <T>ImplGlobal<T> for Global<T> {
    // accesing the Global
    fn access(&self) -> MutexGuard<'_, Option<T>> {
        self.lock().unwrap()
    }
    // overwriting the Global
    fn sets(&self, new: T) {
        *self.access() = Some(new);
    }
    // reseting the Global
    fn none(&self) {
        *self.access() = None;
    }
}