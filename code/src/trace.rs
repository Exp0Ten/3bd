use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::ffi::c_void;

use std::collections::HashMap;


use nix::sys::ptrace;
use nix::sys::signal::{self, Signal,};
use nix::unistd::Pid;
use nix::sys::wait;

use nix::libc::user_regs_struct;

use ::object as object_foreign;

use object_foreign::Object;


use crate::data::*;
use crate::window::Dialog;
use crate::dwarf::*;
use crate::object;
use crate::window;
use crate::ui;

pub type Breakpoints = HashMap<u64, u8>;

pub trait ImplBreakpoints {
    fn add(&mut self, address: u64, byte: u8);
    fn add_future(&mut self, address: u64);
    fn rem(&mut self, address: u64) -> u8;
    fn is_active(&self, address: u64) -> bool;
    fn disable_all(&self) -> Result<(), ()>;
    fn enable_all(&mut self) -> Result<(), ()>;
}

impl ImplBreakpoints for Breakpoints {
    fn add(&mut self, address: u64, byte: u8) {
        self.insert(address, byte);
    }

    fn add_future(&mut self, address: u64) { // we save the location
        self.add(address, 0);
    }

    fn rem(&mut self, address: u64) -> u8 {
        self.remove(&address).unwrap()
    }

    fn is_active(&self, address: u64) -> bool {
        self.contains_key(&address)
    }

    fn disable_all(&self) -> Result<(), ()> { // This doesnt actually remove the saved breakpoints, it just removes them out of the tracee's code, good for single stepping and such
        let keys = self.keys();
        for key in keys {
            let byte = self.get(key).unwrap();
            remove_breakpoint(PID.access().unwrap(), anti_normal(*key), *byte)?;
        };
        Ok(())
    }

    fn enable_all(&mut self) -> Result<(), ()> {
        let copy = self.clone();
        let keys = copy.keys();
        for key in keys {
            let byte = insert_breakpoint(PID.access().unwrap(), anti_normal(*key))?;
            *self.get_mut(key).unwrap() = byte;
        };
        Ok(())
    }
}

#[derive(Debug)]
pub struct MapBits {
    pub r: bool,
    pub w: bool,
    pub x: bool,
    p: bool
}

#[derive(Debug)]
pub struct MemoryMap {
    pub name: String,
    pub range: std::ops::Range<u64>,
    pub permissions: MapBits,      // rwxp (read, write, execute, private)
    pub offset: u64,               // into file
}

pub enum Register {
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
    RIP(u64)
}

#[derive(Debug, Clone)]
pub enum Operation {
    LoadFile,
    ReloadFile,
    RunTracee,
    StopTracee,
    Step,
    SourceStep,
    Pause,
    Continue,
    Kill,
    Signal,
    SignalSelect(nix::sys::signal::Signal),
    BreakpointAdd(u64),
    BreakpointRemove(u64),
    HandleSignal(wait::WaitStatus),
    Reset,
    ResetFile
    //fill as needed
}


// Inner Tracing Logic

pub fn operation_message(state: &mut window::State, operation: Operation, task: &mut Option<iced::Task<window::Message>>) {
    match operation {
        Operation::LoadFile => {
            let file = match Dialog::file(None, None) {
                Some(file) => file,
                None => return
            };
            match object::test_file(&file) {
                Ok(_) => (),
                Err(()) => return
            };

            let data = object::read_file(&file);


            unsafe {
                DATA = data
            }
            FILE.sets(file);

            dwarf_set(); // preloading all dwarf related data
            panes_preload(state);
        },
        Operation::RunTracee => {
            let pid = match object::run_tracee(FILE.access().as_ref().unwrap(), Vec::new(), None) {
                Err(_) => return,
                Ok(pid) => {PID.sets(Pid::from_raw(pid)); Pid::from_raw(pid)},
            };
            tracee_setup(state, pid);
        },
        Operation::StopTracee => {
            match kill_tracee(PID.access().unwrap()) {
                Ok(_) => (),
                Err(()) => return
            };
            *task = Some(iced::Task::done(window::Message::Operation(Operation::Reset)));
        },
        Operation::Step => {
            let pid = PID.access().unwrap();
            if step(pid, None).is_err() {return;};
        },
        Operation::SourceStep => {
            let pid = PID.access().unwrap();
            if source_step(pid, LINES.access().as_ref().unwrap()).is_err() {
                return;
            };
        },
        Operation::Pause => {
            if signal(PID.access().unwrap(), Signal::SIGTRAP).is_err() {
                return;
            };
        },
        Operation::Continue => {
            BREAKPOINTS.access().as_mut().unwrap().enable_all().unwrap();
            let pid = PID.access().unwrap();

            if continue_tracee(pid).is_err() {
                return;
            };
        },
        Operation::Kill => {
            if !state.internal.stopped {

            };
            if signal(PID.access().unwrap(), Signal::SIGKILL).is_err() {
                return;
            };
        },
        Operation::Signal => {
            let pid = PID.access().unwrap();
            if signal(pid, Signal::SIGSTOP).is_err() {
                return;
            };
            let _ = wait(pid);
            //now we can signal the tracee (all ptrace functions (apart from some) can be done only when the tracee is stopped)
            let _ = signal_tracee(pid, state.internal.selected_signal.unwrap());
        },
        Operation::SignalSelect(signal) => {state.internal.selected_signal = Some(signal)},
        Operation::BreakpointAdd(addr) => BREAKPOINTS.access().as_mut().unwrap().add_future(addr),
        Operation::BreakpointRemove(addr) => {BREAKPOINTS.access().as_mut().unwrap().rem(addr);},

        Operation::HandleSignal(status) => handle(state, status, task),
        Operation::Reset => {
            state.internal.stopped = false;
            reset();
        },
        Operation::ResetFile => {
            state.internal.stopped = false;
            if PID.access().is_some() {reset();}
            FILE.none();
            DWARF.none();
            EHFRAME.none();
            BREAKPOINTS.none();
            SOURCE.none();
            LINES.none();
            FUNCTIONS.none();
            unsafe {
                DATA = Vec::new()
            };
        },
        _ => ()
    };
}

fn dwarf_set() {
    #[allow(static_mut_refs)]
    let data = unsafe {
        &DATA
    };

    let (dwarf, object) = load_dwarf(data);

    let endian = match object.endianness() {
        object_foreign::Endianness::Little => Endian::Little,
        object_foreign::Endianness::Big => Endian::Big
    };
    ENDIAN.sets(endian);
    EHFRAME.sets(EhFrame::new(object));

    load_source(dwarf.dwarf(endian));
    parse_functions(dwarf.dwarf(endian));

    DWARF.sets(dwarf);
}

fn panes_preload(state: &mut window::State) {
    BREAKPOINTS.sets(Breakpoints::new());
    let panes = &mut state.layout.panes;

    let (comp_dir, main_file) = get_main_file();

    for (_id, pane) in panes.iter_mut() {
        match pane {
            ui::Pane::Code(inner) => {
                inner.dir = Some(comp_dir.clone());
                inner.file = Some(main_file.clone());
            },
            _ => ()
        }
    }
}

fn tracee_setup(state: &mut window::State, pid: Pid) {
    let proc_path = PathBuf::from(format!("/proc/{pid}/"));
    state.status = Some(wait(pid).unwrap());
    let path = get_tracee_path(&proc_path).unwrap();
    FILE.sets(path.clone());

    let maps = get_process_maps(&proc_path).unwrap();

    for map in maps {
        if map.name != path.to_str().unwrap() {
            continue;
        }
        if map.permissions.x {
            EXEC_SHIFT.sets(map.range.start - map.offset);
            break;
        }
    };

    MEMORY.sets(open_memory(&proc_path).unwrap());
    PROC_PATH.sets(proc_path);
    state.internal.stopped = true;

    REGISTERS.sets(get_registers(pid).unwrap());

    let panes = &mut state.layout.panes;

    for (id, pane) in panes.iter_mut() {
        match pane {
            ui::Pane::Memory(inner) => {
                inner.address = anti_normal(0);
                inner.field = ui::Base::form(&ui::Base::Hex, anti_normal(0));
                ui::update_memory(inner);
            },
            _ => ()
        }
    };
}

fn handle(state: &mut window::State, status: wait::WaitStatus, task: &mut Option<iced::Task<window::Message>>) {
    match status {
        wait::WaitStatus::Exited(_, exit) => {
            state.info = window::Info::Exited(exit);
            *task = Some(iced::Task::done(window::Message::Operation(Operation::Reset)));
            return;
        },

        _ => return
    };


}

fn reset() {
    STDIO.none();
    PID.none();
    PROC_PATH.none();
    EXEC_SHIFT.none();
    MEMORY.none();
    REGISTERS.none();
}

// Tracing Interface

pub fn open_memory(proc_path: &PathBuf) -> Result<File, ()> {
    let mut path = proc_path.clone();
    path.push("mem");
    match File::open(path) {
        Ok(file) => Ok(file),
        Err(err) => {Dialog::error(&format!("Could not open memory of the tracee: {}", err), Some("Trace Error")); Err(())}
    }
}

fn close_memory() {
    *MEMORY.access() = None;
}

pub fn get_process_maps(proc_path: &PathBuf) -> Result<Vec<MemoryMap>, ()> {
    let mut path = proc_path.clone();
    path.push("maps");
    let mut file = match File::open(path) {
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
            if line == "" {continue;}

            let mut split = line.split_ascii_whitespace();

            let mut range_split: std::str::Split<'_, &str> = split.next().unwrap().split("-");
            let range: (u64, u64) = (u64::from_str_radix(range_split.next().unwrap(), 16).unwrap(), u64::from_str_radix(range_split.next().unwrap(), 16).unwrap());

            let range = range.0..range.1;

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

            if name == "" {continue;}

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

pub fn get_map_range(address: u64) -> Option<std::ops::Range<u64>> {
    let maps = get_process_maps(PROC_PATH.access().as_ref().unwrap()).unwrap();
    for map in maps {
        if map.range.contains(&address) {
            return Some(map.range);
        }
    };
    None
}

pub fn get_tracee_path(proc_path: &PathBuf) -> Result<PathBuf, ()> {
    let mut path = proc_path.clone();
    path.push("exe");
    match std::fs::read_link(path) {
        Ok(path) => Ok(path),
        Err(err) => {Dialog::error(&format!("Could not get tracee's path: {}", err), Some("Trace error")); Err(())}
    }
}

pub fn insert_breakpoint(pid: Pid, address: u64) -> Result<u8, ()> {
    let save = match ptrace::read(pid, address as *mut c_void) {
        Ok(long) => long as u64,
        Err(err) => {Dialog::error(&format!("Could not insert breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    };

    match ptrace::write(pid, address as *mut c_void, (0xcc | (save & 0xffffffffffffff00)) as i64) {
        Ok(()) => Ok((save & 0xff) as u8),
        Err(err) => {Dialog::error(&format!("Could not insert breakpoint at {}: {}", address, err), Some("Trace error")); Err(())}
    }
}

pub fn remove_breakpoint(pid: Pid, address: u64, byte: u8) -> Result<(), ()> {
    let save = match ptrace::read(pid, address as *mut c_void) {
        Ok(long) => long as u64,
        Err(err) => {Dialog::error(&format!("Could not remove breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    };

    match ptrace::write(pid, address as *mut c_void, (byte as u64 | (save & 0xffffffffffffff00)) as i64) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not remove breakpoint at {}: {}", address, err), Some("Trace error")); Err(())}
    }
}

pub fn get_registers(pid: Pid) -> Result<user_regs_struct, ()> {
    match ptrace::getregs(pid) {
        Ok(regs) => Ok(regs),
        Err(err) => {Dialog::error(&format!("Could not get register values: {}", err), Some("Trace error")); Err(())}
    }
}

fn set_registers(pid: Pid, regs: user_regs_struct) -> Result<(), ()> {
    match ptrace::setregs(pid, regs) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not set register values: {}", err), Some("Trace error")); Err(())}
    }
}

pub fn set_register_value(pid: Pid, register: Register) -> Result<(), ()> {
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
        Register::RIP(value) => regs.rip = value
    };

    set_registers(pid, regs)?;
    Ok(())
}

fn kill_tracee(pid: Pid) -> Result<Option<String>, ()> {
    close_memory();
    match ptrace::kill(pid) {
        Ok(()) => (),
        Err(err) => {Dialog::error(&format!("Could not stop the tracee: {}", err), Some("Trace error")); return Err(());}
    };

    Ok(object::close_child_stdio())
}

fn restart_tracee(pid: Pid, signal: Option<Signal>) -> Result<(), ()> { //also used for signaling the tracee
    match ptrace::cont(pid, signal) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not deliver the signal to the tracee: {}", err), Some("Trace error")); Err(())}
    }
}

fn signal_tracee(pid: Pid, signal: Signal) -> Result<(), ()> { //WRAPPER
    restart_tracee(pid, Some(signal))
}

pub fn continue_tracee(pid: Pid) -> Result<(), ()> { //WRAPPER
    restart_tracee(pid, None)
}

pub fn step(pid: Pid, signal: Option<Signal>) -> Result<(), ()> {
    match ptrace::step(pid, signal) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not step the tracee program: {}", err), Some("Trace error")); Err(())}
    }
}

pub fn step_over(pid: Pid, rip: u64, byte: u8) -> Result<(), ()> { // Step after a breakpoint, not over an instruction
    remove_breakpoint(pid, rip, byte)?;
    step(pid, None)?;
    let _ = wait(pid);
    insert_breakpoint(pid, rip)?;
    Ok(())
}

pub fn source_step<'a>(pid: Pid, lines: &'a LineAddresses) -> Result<(u64,  &'a SourceIndex), ()> { // new rip, line_number and File PathBuf
    loop {
        step(pid, None)?;
        let _ = wait(pid);
        let rip = get_registers(pid)?.rip;
        match lines.get_line(rip) {
            Some(line) => return Ok((rip, line)),
            None => ()
        };
    }
}

fn seek_memory(address: u64, memory_file: &mut File) -> Result<(), ()> {
    match memory_file.seek(std::io::SeekFrom::Start(address)) {
        Ok(_) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not seek into the memory file: {}", err), Some("Memory error")); Err(())}
    }
}

pub fn test_memory(address: u64) -> Result<(), bool> { // if Err(true) -> start, if Err(false) -> End, if Ok() then we are in valid memory space
    match get_map_range(address) {
        Some(_) => (),
        None => return Err(true)
    };
    match get_map_range(address + 2048) {
        Some(_) => (),
        None => return Err(false)
    };
    Ok(())
}

pub fn read_memory(address: u64, amount: usize) -> Result<Vec<u8>, ()> {
    let mut bind = MEMORY.access();
    let mut memory = bind.as_mut().unwrap();
    let mut buf: Vec<u8> = vec![0; amount];

    seek_memory(address, &mut memory)?;

    match memory.read_exact(&mut buf) {
        Ok(()) => (),
        Err(err) => {Dialog::error(&format!("Could not read from the memory file: {}", err), Some("Memory error")); return Err(());}
    };

    Ok(buf)
}

fn write_memory(address: u64, buf: &[u8]) -> Result<(), ()> {
    let mut bind = MEMORY.access();
    let mut memory = bind.as_mut().unwrap();

    seek_memory(address, &mut memory)?;

    match memory.write_all(buf) {
        Ok(_) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not seek into the memory file: {}", err), Some("Memory error")); Err(())}
    }
}

pub fn wait(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> {
   wait::waitpid(pid, None)
}

pub fn testwait(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> {
    nix::sys::wait::waitpid(pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG))
}

fn signal(pid: Pid, signal: Signal) -> Result<(), ()> {
    signal::kill(pid, signal).map_err(|_| ())
}