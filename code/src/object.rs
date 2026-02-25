use std::path::Path;
use std::process;
use std::process::Stdio;
use std::os::unix::{process::CommandExt, fs::MetadataExt};


use std::fs;

use std::io::pipe;
use std::io::{PipeReader, PipeWriter, Read};

use nix::sys::ptrace;
use nix::unistd::{fork, ForkResult};

use crate::window::Dialog;
use crate::data::*;


pub fn run_tracee(file: &Path, args: Vec<String>, pipe_stdio: (PipeReader, PipeWriter)) -> Result<i32, i32> {

    let stdio = (
        Stdio::from(pipe_stdio.0),
        Stdio::from(pipe_stdio.1),
    );

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

pub fn open_child_stdio() -> Result<(PipeReader, PipeWriter), ()> { //returns the stdio pipe reader and writer for the TRACEE, and sets the Internal global stdio for the TRACER
    let stdin_pipe = match pipe() {
        Ok(pipe) => pipe,
        Err(err) => {Dialog::error(&format!("Could not open pipe: {}", err), Some("Trace error")); return Err(());}
    };
    let stdout_pipe = match pipe() {
        Ok(pipe) => pipe,
        Err(err) => {Dialog::error(&format!("Could not open pipe: {}", err), Some("Trace error")); return Err(());}
    };

    INTERNAL.access().tracee_stdio = Some((stdin_pipe.1, stdout_pipe.0));

    Ok((stdin_pipe.0, stdout_pipe.1))
}

pub fn close_child_stdio() -> Option<String> { // returns what was left in the pipes, should be empty, errors on none empty
    let mut internal = INTERNAL.access();

    let (stdin, stdout) = internal.tracee_stdio.as_mut().unwrap();
    let mut text = String::new();

    let _ = stdout.read_to_string(&mut text);
    internal.tracee_stdio = None;

    if text == "" {
        None
    } else {
        Some(text)
    }
    // TODO, CHECK IF PIPES GET CLOSED !!!!!!!!!!!!!!!!!
}

pub fn read_file(file: &Path) -> Vec<u8> { // TODOMAYBE create mmap instead
    fs::read(file).unwrap()
}

