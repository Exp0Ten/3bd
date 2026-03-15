use std::sync::{Mutex, MutexGuard};
use std::path;
use std::fs;
use std::io::{PipeReader, PipeWriter};


use nix::unistd::Pid;

use crate::config;
use crate::dwarf;
use crate::trace;

use rust_embed::Embed; // to run as a single file binary without the dependecy on the file system

#[derive(Embed)]
#[folder = "assets/"]
#[exclude = "*.md"]
#[exclude = "*/LICENSE"]
pub struct Asset;

// Public Handle

pub static mut DATA: Vec<u8> = Vec::new(); //The file contents

type Global<T> = Mutex<Option<T>>;

pub static CONFIG: Global<config::Config> = empty();
pub static FILE: Global<std::path::PathBuf> = empty();
pub static STDIO: Global<std::os::fd::OwnedFd> = empty();
pub static PID: Global<Pid> = empty();
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

pub static SAVED_STATE: Global<SavedState> = empty();

#[derive(Clone)]
pub struct SavedState {
    pub left_sidebar: (iced::widget::pane_grid::Configuration<crate::ui::Pane>, f32),
    pub right_sidebar: (iced::widget::pane_grid::Configuration<crate::ui::Pane>, f32),
    pub panel: (iced::widget::pane_grid::Configuration<crate::ui::Pane>, f32),
    pub main: Option<iced::widget::pane_grid::Configuration<crate::ui::Pane>> // this is just for the parsing, not for the actual storing of the info
}

const fn empty<T>() -> Global<T> {Mutex::new(None)}

pub trait ImplGlobal<T> {
    fn access(&self) -> MutexGuard<'_, Option<T>>;
    fn sets(&self, new: T);
    fn none(&self);
}

impl <T>ImplGlobal<T> for Global<T> {
    fn access(&self) -> MutexGuard<'_, Option<T>> {
        self.lock().unwrap()
    }
    fn sets(&self, new: T) {
        *self.access() = Some(new);
    }
    fn none(&self) {
        *self.access() = None;
    }
}