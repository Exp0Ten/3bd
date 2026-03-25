use std::{
    fs,
    path::Path,
    process::{
        self,
        Stdio
    },
    os::unix::{
        process::CommandExt,
        fs::MetadataExt
    },
    io::Read
};

use nix::{
    sys::ptrace,
    pty,
    unistd::{
        fork,
        ForkResult
    }
};

// internal import
use crate::{
    data::*,
    window::Dialog
};


/// FILE: object.rs - Managing communication with the filesystem

pub fn run_tracee(file: &Path, args: Vec<String>, slave: Option<std::os::fd::OwnedFd>) -> Result<i32, ()> {
    let stdio= match slave { // creating Stdio from the internal terminal or the external ones
        Some(slave) =>  ( //AI
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

    match unsafe {fork()} { // forking the program to create the child that will run PTRACE_TRACEME (so we can attach without su permissions)
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

    process::Command::new(file) // .exec() will run execve syscall, replacing the entire memory of the child with the new executable (program we want to debug)
    .stdin(stdio.0)
    .stdout(stdio.1)
    .stderr(stdio.2)
    .args(args)
    .exec()
}

pub fn test_file(file: &Path) -> Result<(), ()> { // for testing if we have a correct file selected
    if !file.exists() || file.is_dir() {
        Dialog::error("File does not exist.", Some("Executable Error"));
        return Err(())
    };

    if let Ok(metadata) = file.metadata() {
        if (metadata.mode() & 0o111) == 0 {
            Dialog::error("File does not have execute permissions.", Some("Executable Error"));
            return Err(());
        }
    } else {
        Dialog::error("Could not read metadata of the file.", Some("Executable Error"));
        return Err(());
    };

    Ok(())
}

pub fn open_child_stdio() -> Result<std::os::fd::OwnedFd, ()> { // retuns the FD of the slave, and sets the Global with the master FD
    let pty = match pty::openpty(None, None) { //AI
        Ok(pty) => pty,
        Err(_) => {Dialog::error("Could not open child stdio.", Some("Trace Error")); return Err(());},
    };
    STDIO.sets(pty.master);

    Ok(pty.slave)
}

pub fn close_child_stdio() -> Result<(), ()> { // discarding the master FD (which by Rust lifetime rules SHOULD close the pty)
    if STDIO.access().is_none() {return Err(());}

    STDIO.none();

    Ok(())
}

pub fn read_file(file: &Path) -> Vec<u8> {
    fs::read(file).unwrap()
}

pub fn read_source(file: &Path) -> Result<String, ()> {
    fs::read_to_string(file).map_err(|_| ())
}

pub async fn read_stdout() -> Result<(Vec<u8>, usize), ()> { // async read of the PTY stdout
    let mut buf = vec![0;256]; // 256 bytes per read
    let amount = stdio()?.read(&mut buf).map_err(|_| ())?;
    Ok((buf, amount))
}

pub fn stdio() -> Result<pty::PtyMaster, ()> { // clones the FD and creates the Master interface for the PTY
    unsafe {
        Ok(pty::PtyMaster::from_owned_fd(STDIO.access().as_ref().ok_or(())?.try_clone().unwrap()))
    }
}