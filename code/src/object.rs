use std::path::Path;
use std::os::fd::RawFd;
use std::os::fd::FromRawFd;
use std::process;
use std::process::Stdio;
use std::os::unix::{process::CommandExt, fs::MetadataExt};

use nix::sys::ptrace;
use nix::unistd::{fork, ForkResult};

use crate::window::Dialog;
use crate::data::*;


pub fn run_tracee(file: &Path, args: Vec<String>) -> Result<i32, i32> {

    let stdio_tup: (RawFd, RawFd) = INTERNAL.get().tracee_stdio.unwrap();

    let stdio = unsafe {(
        Stdio::from_raw_fd(stdio_tup.0),
        Stdio::from_raw_fd(stdio_tup.1),
    )};

    match unsafe {fork()} {
        Ok(ForkResult::Parent { child }) => Ok(child.into()),
        Ok(ForkResult::Child) => {tracee_program(file, args, stdio); Ok(0)}, // it will never return, as tracee_program exits after finishing the file execution
        Err(_) => Err(-1) // Fork Failed, TODO display Error
    }
}


fn tracee_program(file: &Path, args: Vec<String>, stdio: (Stdio, Stdio)) {
    ptrace::traceme().unwrap_or_else(|err| {
        Dialog::error(&format!("Failed to execute ptrace on the tracee: {}", err), Some("Traceme error"));
        process::exit(-1)
    });

    let _ = process::Command::new(file.as_os_str())
    .stdin(stdio.0)// TODO
    .stdout(stdio.1)
    .args(args)
    .exec();
}

fn test_file(file: &Path) -> Result<(), ()> { // True if it has DWARF, False if it doesnt, Err if invalid file
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