use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::ffi::c_void;

use nix::sys::ptrace;
use nix::unistd::Pid;

use nix::libc::user_regs_struct;

use crate::data::INTERNAL;
use crate::window::Dialog;

struct MapBits {
    r: bool,
    w: bool,
    x: bool,
    p: bool
}

struct MemoryMap {
    name: String,
    range: (u64, u64),
    permissions: MapBits,      // rwxp (read, write, execute, private)
    offset: u64,               // into file
}

enum Register { // only general purpose for now
    RAx(u64),
    RBx(u64),
    RCx(u64),
    RDx(u64),
    RSi(u64),
    RDi(u64),
    RBp(u64),
    RSp(u64),
    R8(u64),
    R9(u64),
    R10(u64),
    R11(u64),
    R12(u64),
    R13(u64),
    R14(u64),
    R15(u64),
    RIp(u64)
}

#[derive(Debug, Clone)]
pub enum Operation {
    LoadFile,
    ReloadFile
    //fill as needed
}

pub fn operation_message(operation: Operation) {
    match operation {
        Operation::LoadFile => {Dialog::file(None, None);},
        _ => ()
    }
}

fn open_memory(proc_path: &mut PathBuf) -> Result<File, ()> {
    match File::open({proc_path.push("mem"); proc_path}) {
        Ok(file) => Ok(file),
        Err(err) => {Dialog::error(&format!("Could not open memory of the tracee: {}", err), Some("Trace Error")); Err(())}
    }
}

fn get_process_maps(proc_path: &mut PathBuf) -> Result<Vec<MemoryMap>, ()> {
    let mut file = match File::open({proc_path.push("maps"); proc_path}) {
        Ok(file) => file,
        Err(err) => {Dialog::error(&format!("Could not open memory map file of the tracee: {}", err), Some("Trace Error")); return Err(())}
    };

    let mut content = String::new();
    match file.read_to_string(&mut content) {
        Ok(_) => (),
        Err(err) => {Dialog::error(&format!("Could not open memory map file of the tracee: {}", err), Some("Trace Error")); return Err(())}
    };
    drop(file);

    let lines: Vec<&str> = content.split("\n").collect();
    let mut mmap_vector = Vec::with_capacity(lines.len());

    for line in lines {
        mmap_vector.push({
            let mut split = line.split_ascii_whitespace();

            let mut range_split: std::str::Split<'_, &str> = split.next().unwrap().split("-");
            let range: (u64, u64) = (u64::from_str_radix(range_split.next().unwrap(), 16).unwrap(), u64::from_str_radix(range_split.next().unwrap(), 16).unwrap());

            let permissions_split = split.next().unwrap();
            let permissions = MapBits {
                r: permissions_split[0..1] == *"r",
                w: permissions_split[1..2] == *"w",
                x: permissions_split[2..3] == *"x",
                p: permissions_split[3..4] == *"p",
            };

            let offset = u64::from_str_radix(split.next().unwrap(), 16).unwrap();

            split.next();
            split.next();

            let name = split.next().unwrap_or("").to_string();

            MemoryMap {
                name,
                range,
                permissions,
                offset
            }
        });
    };
    Ok(mmap_vector)
}

fn insert_breakpoint(pid: Pid, address: u64) -> Result<u8, ()> {
    let save = match ptrace::read(pid, address as *mut c_void) {
        Ok(long) => long as u16,
        Err(err) => {Dialog::error(&format!("Could not insert breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    };

    match ptrace::write(pid, address as *mut c_void, (0xcc | (save & 0xff00)).into()) {
        Ok(()) => Ok((save & 0xff) as u8),
        Err(err) => {Dialog::error(&format!("Could not insert breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    }
}

fn remove_breakpoint(pid: Pid, address: u64, byte: u8) -> Result<(), ()> {
    let save = match ptrace::read(pid, address as *mut c_void) {
        Ok(long) => long as u16,
        Err(err) => {Dialog::error(&format!("Could not remove breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    };

    match ptrace::write(pid, address as *mut c_void, (byte as u16 | (save & 0xff00)).into()) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not remove breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    }
}

fn get_registers(pid: Pid) -> Result<user_regs_struct, ()> {
    match ptrace::getregs(pid) {
        Ok(regs) => Ok(regs),
        Err(err) => {Dialog::error(&format!("Could not get register values: {}", err), Some("Trace error")); return Err(());}
    }
}

fn set_registers(pid: Pid, regs: user_regs_struct) -> Result<(), ()> {
    match ptrace::setregs(pid, regs) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not ser register values: {}", err), Some("Trace error")); return Err(());}
    }
}

fn set_register_value(pid: Pid, register: Register) -> Result<(), ()> {
    let mut regs = get_registers(pid)?;

    match register {
        Register::RAx(value) => regs.rax = value,
        Register::RBx(value) => regs.rbx = value,
        Register::RCx(value) => regs.rcx = value,
        Register::RDx(value) => regs.rdx = value,
        Register::RSi(value) => regs.rsi = value,
        Register::RDi(value) => regs.rdi = value,
        Register::RBp(value) => regs.rbp = value,
        Register::RSp(value) => regs.rsp = value,
        Register::R8(value) => regs.r8 = value,
        Register::R9(value) => regs.r9 = value,
        Register::R10(value) => regs.r10 = value,
        Register::R11(value) => regs.r11 = value,
        Register::R12(value) => regs.r12 = value,
        Register::R13(value) => regs.r13 = value,
        Register::R14(value) => regs.r14 = value,
        Register::R15(value) => regs.r15 = value,
        Register::RIp(value) => regs.rip = value
    };

    set_registers(pid, regs)?;
    Ok(())
}

//TODO implement INTERNAL