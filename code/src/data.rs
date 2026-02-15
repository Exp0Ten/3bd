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
pub struct Internal {
    pub config: Option<config::Config>,
    pub file: Option<Box<path::Path>>,
    pub tracee_stdio: Option<(PipeWriter, PipeReader)>, // stdin writer, and stdout reader
    pub pid: Option<Pid>,
    pub proc_path: Option<path::PathBuf>,
    pub memory_file: Option<fs::File>,
    pub dwarf: Option<dwarf::DwarfSections<'static>>,
    pub source_files: Option<dwarf::SourceMap>,
    pub line_addresses: Option<dwarf::LineAddresses>, //dont forget to drop this reference when changing tracee
    pub breakpoints: Option<trace::Breakpoints>
}

// Public Handle

pub static INTERNAL: Mutex<Internal> = Mutex::new(Internal::empty());

impl Internal {
    const fn empty() -> Self {
        Internal {
            config: None,
            file: None,
            tracee_stdio: None,
            pid: None,
            proc_path: None,
            memory_file: None,
            dwarf: None,
            source_files: None,
            line_addresses: None,
            breakpoints: None

        }
    }
}

impl Default for Internal {
    fn default() -> Self {
        Internal {
            config: Some(config::load_config()),
            file: None,
            tracee_stdio: None,
            pid: None,
            proc_path: None,
            dwarf: None,
            memory_file: None,
            source_files: None,
            line_addresses: None,
            breakpoints: Some(trace::Breakpoints::new())
        }
    }
}

pub trait Glob<'a> {
    fn access(&'a self) -> MutexGuard<'a, Internal>;
//    fn get(&'a self) -> Internal;
    fn set(&'a self, internal: Internal);
    //  add more as needed
}

impl <'a> Glob<'a> for Mutex<Internal> {
    fn access(&'a self) -> MutexGuard<'a, Internal> {
        self.lock().unwrap()
    }

//    fn get(&'a self) -> Internal {
//        self.access().clone()
//    }

    fn set(&'a self, internal: Internal) {
        *self.access() = internal;
    }

}

fn load_internal() { //TODO
    Glob::set(&INTERNAL,Internal::default());
}