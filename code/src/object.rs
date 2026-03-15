use std::path::Path;
use std::process;
use std::process::Stdio;
use std::os::unix::{process::CommandExt, fs::MetadataExt};


use std::fs;

use std::io::{Read};

use nix::sys::ptrace;
use nix::pty;
use nix::unistd::{fork, ForkResult};

use crate::window::Dialog;
use crate::data::*;


pub fn run_tracee(file: &Path, args: Vec<String>, slave: Option<std::os::fd::OwnedFd>) -> Result<i32, ()> {

    let stdio =match slave {
        Some(slave) =>  (
            Stdio::from(slave.try_clone().map_err(|_| ())?),
            Stdio::from(slave.try_clone().map_err(|_| ())?),
            Stdio::from(slave),
        ),
        None => (
            Stdio::inherit(),
            Stdio::inherit(),
            Stdio::inherit(),
        )
    };

    match unsafe {fork()} {
        Ok(ForkResult::Parent { child }) => Ok(child.into()),
        Ok(ForkResult::Child) => {
            let error = tracee_program(file, args, stdio);
            process::exit(error.raw_os_error().unwrap());
        },
        Err(_) => Err(())
    }
}

fn tracee_program(file: &Path, args: Vec<String>, stdio: (Stdio, Stdio, Stdio)) -> std::io::Error {
    ptrace::traceme().unwrap_or_else(|err| {
        Dialog::error(&format!("Failed to execute ptrace on the tracee: {}", err), Some("Traceme error"));
        process::exit(-1)
    });

    //let a= process::Command::new(file.as_os_str())
    process::Command::new(file)
    .stdin(stdio.0)// TODO
    .stdout(stdio.1)
    .stderr(stdio.2)
    .args(args)
    .exec()
}

pub fn test_file(file: &Path) -> Result<(), ()> { // True if it has DWARF, False if it doesnt, Err if invalid file
    let title = Some("Executable Error");

    if !file.exists() || file.is_dir() {
        Dialog::error("File does not exist.", title);
        return Err(())
    };

    if let Ok(metadata) = file.metadata() {
        if (metadata.mode() & 0o111) == 0 {
            Dialog::error("File does not have execute permissions.", title);
            return Err(());
        }
    } else {
        Dialog::error("Could not read metadata of the file.", title);
        return Err(());
    };

    Ok(())
}

pub fn open_child_stdio() -> Result<std::os::fd::OwnedFd, ()> { //returns the stdio pipe reader and writer for the TRACEE, and sets the Internal global stdio for the TRACER
    let pty = match pty::openpty(None, None) {
        Ok(pty) => pty,
        Err(_) => {Dialog::error("Could not open child stdio.", Some("Trace Error")); return Err(());},
    };
    STDIO.sets(pty.master);

    Ok(pty.slave)
}

pub fn close_child_stdio() -> Result<(), ()> {
    if STDIO.access().is_none() {return Err(());}

    STDIO.none();

    Ok(())
}

pub fn read_file(file: &Path) -> Vec<u8> { // TODOMAYBE create mmap instead
    fs::read(file).unwrap()
}

pub fn read_source(file: &Path) -> Result<String, ()> {
    fs::read_to_string(file).map_err(|_| ())
}

pub async fn read_stdout() -> Result<(Vec<u8>, usize), ()> {
    let mut buf = vec![0;256]; // 256 bytes per read
    let amount = stdio().read(&mut buf).map_err(|_| ())?;
    Ok((buf, amount))
}

pub fn stdio() -> pty::PtyMaster {
    unsafe {
        pty::PtyMaster::from_owned_fd(STDIO.access().as_ref().unwrap().try_clone().unwrap())
    }
}