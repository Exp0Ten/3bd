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

//Internal Data
//#[derive(Debug)]
//#![allow(unused)] // if needed
pub struct Internal<'a> {
    pub config: Option<config::Config>,
    pub file: Option<Box<std::path::Path>>,
    pub tracee_stdio: Option<(PipeWriter, PipeReader)>, // stdin writer, and stdout reader
    pub pid: Option<Pid>,
    pub proc_path: Option<path::PathBuf>,
    pub dynamic_exec_shift: Option<u64>, // dynamic executables ave shifted RIP
    pub memory_file: Option<fs::File>,
    pub dwarf: Option<dwarf::DwarfSections<'a>>,
    pub eh_frame: Option<dwarf::EhFrame<'a>>,
    pub source_files: Option<dwarf::SourceMap>,
    pub line_addresses: Option<dwarf::LineAddresses>, //dont forget to drop this reference when changing tracee
    pub function_index: Option<dwarf::FunctionIndex<'a>>,
    pub breakpoints: Option<trace::Breakpoints>,
    pub registers: Option<nix::libc::user_regs_struct> // make custom struct later??
}

// TODO - ORGANISE INTO MORE VARIBLES!!!;

// Public Handle

pub static mut DATA: Vec<u8> = Vec::new(); //The file contents

type Global<T> = Mutex<Option<T>>;

pub static INTERNAL: Mutex<Internal> = Mutex::new(Internal::empty());
pub static CONFIG: Global<config::Config> = empty();
pub static FILE: Global<Box<std::path::Path>> = empty();
pub static STDIO: Global<(PipeWriter, PipeReader)> = empty();
pub static PID: Global<Pid> = empty();
pub static PROC_PATH: Global<path::PathBuf> = empty();
pub static EXEC_SHIFT: Global<u64> = empty();
pub static DWARF: Global<dwarf::DwarfSections> = empty();
pub static EHFRAME: Global<dwarf::EhFrame> = empty();
pub static SOURCE: Global<dwarf::SourceMap> = empty();
pub static LINES: Global<dwarf::LineAddresses> = empty();
pub static FUNCTIONS: Global<dwarf::FunctionIndex> = empty();
pub static BREAKPOINTS: Global<trace::Breakpoints> = empty();
pub static REGISTERS: Global<nix::libc::user_regs_struct> = empty();

const fn empty<T>() -> Global<T> {Mutex::new(None)}

impl <'a>Internal<'a> {
    const fn empty() -> Self {
        Internal {
            config: None,
            file: None,
            tracee_stdio: None,
            pid: None,
            proc_path: None,
            dynamic_exec_shift: None,
            memory_file: None,
            dwarf: None,
            eh_frame: None,
            source_files: None,
            line_addresses: None,
            function_index: None,
            breakpoints: None,
            registers: None

        }
    }
}

impl <'a>Default for Internal<'a> {
    fn default() -> Self {
        Internal {
            config: Some(config::load_config()),
            file: None,
            tracee_stdio: None,
            pid: None,
            proc_path: None,
            dynamic_exec_shift: None,
            dwarf: None,
            eh_frame: None,
            memory_file: None,
            source_files: None,
            line_addresses: None,
            function_index: None,
            breakpoints: None,
            registers: None
        }
    }
}

pub trait Glob<'a> {
    fn access(&'a self) -> MutexGuard<'a, Internal<'a>>;
//    fn get(&'a self) -> Internal;
    fn set(&'a self, internal: Internal<'a>);
    //  add more as needed
}

impl <'a> Glob<'a> for Mutex<Internal<'a>> {
    fn access(&'a self) -> MutexGuard<'a, Internal<'a>> {
        self.lock().unwrap()
    }

//    fn get(&'a self) -> Internal {
//        self.access().clone()
//    }

    fn set(&'a self, internal: Internal<'a>) {
        *self.access() = internal;
    }
}

fn load_internal() { //TODO
    Glob::set(&INTERNAL,Internal::default());
}

pub trait ImplGlobal<T> {
    fn access(&self) -> MutexGuard<'_, Option<T>>;
    fn sets(&self, new: T);
}

impl <T>ImplGlobal<T> for Global<T> {
    fn access(&self) -> MutexGuard<'_, Option<T>> {
        self.lock().unwrap()
    }
    fn sets(&self, new: T) {
        *self.access() = Some(new);
    }
}





// SAVE FOR LATER
fn hi() {
    unsafe {
        DATA = vec![0,1,2]
    }
    #[allow(static_mut_refs)]
    let a = unsafe {
        &DATA
    };
}