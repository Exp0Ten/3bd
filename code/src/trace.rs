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

const SI_KERNEL: i32 = 0x80;
const TRAP_BRKPT: i32 = 1;

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
            let byte = insert_breakpoint(PID.access().unwrap(), anti_normal(*key));
            match byte {
                Ok(byte) => *self.get_mut(key).unwrap() = byte,
                Err(()) => {self.remove(key);}
            };
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
    Signal(Signal),
    BreakpointAdd(u64),
    BreakpointRemove(u64),
    HandleSignal(Result<wait::WaitStatus, nix::errno::Errno>),
    Reset,
    ResetFile,
    Read(Result<(Vec<u8>, usize), ()>),
    Stack(Result<Vec<(usize, String)>, ()>)
}

//TASK DEFINITION

fn task_wait() -> iced::Task<window::Message> {
    iced::Task::perform(wait_async(PID.access().unwrap()), |result| window::Message::Operation(Operation::HandleSignal(result)))
}

fn task_reset() -> iced::Task<window::Message> {
    iced::Task::done(window::Message::Operation(Operation::Reset))
}

fn task_read() -> iced::Task<window::Message> {
    iced::Task::perform(object::read_stdout(), |result| window::Message::Operation(Operation::Read(result)))
}

fn task_stack() -> iced::Task<window::Message> {
    iced::Task::perform(async {ui::stack_lines(call_stack())}, |result| window::Message::Operation(Operation::Stack(result)))
}

// Inner Tracing Logic

pub fn operation_message(state: &mut window::State, operation: Operation, task: &mut Option<iced::Task<window::Message>>) {
    match operation {
        Operation::LoadFile => {
            let reset = FILE.access().is_some();

            let file = match Dialog::file(None, None) {
                Some(file) => file,
                None => return
            };
            if reset {
                if reset_file(state).is_err() {
                    return;
                };
            }

            match object::test_file(&file) {
                Ok(_) => (),
                Err(()) => return
            };

            let data = object::read_file(&file);
            unsafe {
                DATA = data
            }
            FILE.sets(file.clone());

            let no_debug = dwarf_set().is_err(); // preloading all dwarf related data
            state.internal.no_debug = no_debug;
            if no_debug {
                Dialog::warning(&format!("This file ({}) does not contain debbuging data.", file.file_name().unwrap().to_str().unwrap()), None);
                BREAKPOINTS.sets(Breakpoints::new());
                return;
            }
            panes_preload(state, task);
        },

        Operation::RunTracee => {
            let stdio = match object::open_child_stdio() {
                Ok(stdio) => stdio,
                Err(()) => return
            };

            let pid = match object::run_tracee(FILE.access().as_ref().unwrap(), Vec::new(), Some(stdio)) {
                Err(_) => return,
                Ok(pid) => {PID.sets(Pid::from_raw(pid)); Pid::from_raw(pid)},
            };
            tracee_setup(state, pid, task);

            *task = Some(task_read());
        },
        Operation::StopTracee => {
            if PID.access().is_none() {
                return;
            };
            match kill_tracee(PID.access().unwrap()) {
                Ok(_) => (),
                Err(()) => return
            };

            let _ = object::close_child_stdio();

            *task = Some(task_reset());
        },

        Operation::Step => {
            if step(PID.access().unwrap(), state.last_signal).is_err() {return;};
            state_cont(state);
            if !state.internal.manual {
                *task = Some(task_wait())
            }
        },
        Operation::SourceStep => {
            let pid = PID.access().unwrap();
            if !state.internal.manual {
                if step(pid, state.last_signal).is_err() {return;};
                state.last_signal = None;
                let _ = wait(pid);
            }
            let mut breakpoints = Breakpoints::new();
            let bind = LINES.access();
            for (address, source) in bind.as_ref().unwrap().iter() { //we breakpoint every line for a single wait call, whatever the program stops at, we disable them again
                if source.hash_path != *state.internal.comp_dir.as_ref().unwrap() {
                    //continue;
                }
                let byte = insert_breakpoint(pid, anti_normal(*address)).unwrap();
                breakpoints.add(*address, byte);
            }
            if restart_tracee(pid, None).is_err() {
                breakpoints.disable_all().unwrap();
                return;
            };
            state.internal.source_step = Some(breakpoints);
            state.internal.stopped = false;
            state_cont(state);
            if !state.internal.manual {
                *task = Some(task_wait())
            }
        },
        Operation::Pause => {
            if send_signal(PID.access().unwrap(), Signal::SIGTRAP).is_err() {
                return;
            };
            state.internal.manual = true;
        },
        Operation::Continue => {
            let pid = PID.access().unwrap();
            if !state.internal.manual {
                if step(pid, None).is_err() {return;};
                let _ = wait(pid);
            }
            if BREAKPOINTS.access().as_mut().unwrap().enable_all().is_err() {
                *task = Some(task_reset());
                return;
            };
            if restart_tracee(pid, state.last_signal).is_err() {
                if BREAKPOINTS.access().as_mut().unwrap().disable_all().is_err() {
                    *task = Some(task_reset())
                };
                return;
            };
            if !state.internal.manual {
                *task = Some(task_wait())
            }
            state_cont(state);
            state.internal.stopped = false;
            state.internal.manual = false;
        },
        Operation::Kill => {
            if state.internal.stopped {
                state.last_signal = Some(Signal::SIGKILL);
                return;
            }
            if send_signal(PID.access().unwrap(), Signal::SIGKILL).is_err() {
                return;
            };
        },
        Operation::Signal(sig) => {
            if state.internal.stopped {
                state.last_signal = Some(sig);
                return;
            }

            let pid = PID.access().unwrap();
            if send_signal(pid, Signal::SIGSTOP).is_err() {
                return;
            };
            let _ = wait(pid);
            //now we can signal the tracee (all ptrace functions (apart from some) can be done only when the tracee is stopped)
            let _ = signal_tracee(pid, sig);
        },

        Operation::BreakpointAdd(addr) => BREAKPOINTS.access().as_mut().unwrap().add_future(addr),
        Operation::BreakpointRemove(addr) => {BREAKPOINTS.access().as_mut().unwrap().rem(addr);},

        Operation::HandleSignal(Ok(status)) => handle(state, status, task),
        Operation::HandleSignal(Err(err)) => *task = match Dialog::warning_choice(&format!("Encountered an error while waiting for the tracee program: {}\nDo you wish to try again? (selecting no will kill the tracee)", err), Some("Trace Error")) {
            rfd::MessageDialogResult::Yes => Some(iced::Task::perform(wait_async(PID.access().unwrap()), |result| window::Message::Operation(Operation::HandleSignal(result)))),
            _ => Some(iced::Task::done(window::Message::Operation(Operation::StopTracee)))
        },

        Operation::Reset => {
            state.internal.stopped = false;
            state.internal.source_step = None;
            state.internal.manual = false;
            state.internal.breakpoint = false;
            state.internal.file = None;
            state.internal.output.clear();
            state.last_signal = None;
            reset();
        },
        Operation::ResetFile => {let _ = reset_file(state);},

        Operation::Read(result) => {
            if result.is_err() {
                return;
            }

            *task = Some(task_read());

            let data = result.unwrap();
            if data.1 == 0 {
                return;
            }

            let mut text: String = data.0[..data.1].iter().map(|byte| *byte as char).collect();

            ui::process_string(&mut text);

            state.internal.output.push_str(&text);
        },
        Operation::Stack(result) => {
            match result {
                Ok(stack) => state.internal.stack = Some(stack),
                Err(()) => state.internal.stack = None
            }
        },
        _ => ()
    };
}

fn dwarf_set() -> Result<(), ()> {
    #[allow(static_mut_refs)]
    let data = unsafe {
        &DATA
    };

    let (dwarf, object) = match load_dwarf(data) {
        Ok(res) => res,
        Err(_) => return Err(())
    };

    let endian = match object.endianness() {
        object_foreign::Endianness::Little => Endian::Little,
        object_foreign::Endianness::Big => Endian::Big
    };
    ENDIAN.sets(endian);
    EHFRAME.sets(EhFrame::new(object));

    load_source(dwarf.dwarf(endian));
    parse_functions(dwarf.dwarf(endian));

    DWARF.sets(dwarf);
    Ok(())
}

fn panes_preload(state: &mut window::State, task: &mut Option<iced::Task<window::Message>>) {
    BREAKPOINTS.sets(Breakpoints::new());
    let panes = &mut state.layout.panes;

    let (comp_dir, main_file) = get_main_file();
    state.internal.comp_dir = Some(PathBuf::from(comp_dir.clone()));
    let mut tasks = Vec::new();

    for (id, pane) in panes.iter_mut() {
        match pane {
            ui::Pane::Code(inner) => {
                inner.dir = Some(comp_dir.clone());
                tasks.push(iced::Task::done(window::Message::Pane(ui::PaneMessage::CodeSelectFile(*id, main_file.clone()))));
            },
            _ => ()
        }
    }
    *task = Some(iced::Task::batch(tasks));
}

fn tracee_setup(state: &mut window::State, pid: Pid, task: &mut Option<iced::Task<window::Message>>) {
    let proc_path = PathBuf::from(format!("/proc/{pid}/"));
    state.status = Some(wait(pid).unwrap());
    let path = get_tracee_path(&proc_path).unwrap();
    FILE.sets(path.clone());

    let maps = get_process_maps(&proc_path).unwrap();

    for map in &maps {
        if map.name != path.to_str().unwrap() {
            continue;
        }
        if map.offset == 0 {
            EXEC_SHIFT.sets(map.range.start);
            break;
        }
    };

    MAPS.sets(maps);
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

    if ui::check_for_assembly(state) {
        *task = assembly_update(state, REGISTERS.access().unwrap().rip);
    }
}


fn handle(state: &mut window::State, status: wait::WaitStatus, task: &mut Option<iced::Task<window::Message>>) {
    state.status = Some(status);
    match status {
        wait::WaitStatus::Exited(pid, exit) => {
            *task = Some(iced::Task::done(window::Message::Operation(Operation::Reset)));
            Dialog::warning(&format!("Program exited with the code: {:-}.\nPid: {}", exit, pid), Some("Program exited"));
            return;
        },
        wait::WaitStatus::Signaled(pid, signal, _) => {
            if !test_pid(pid) {
                Dialog::warning(&format!("Program with pid {} was terminated by signal: {}", pid, signal), Some("Program ended"));
                *task = Some(task_reset());
                return;
            }
            if !state.internal.manual {
                state.last_signal = Some(signal)
            }
        },
        _ => ()
    };

    let pid = PID.access().unwrap();

    let info = match get_sig_info(pid) {
        Ok(info) => info,
        Err(_) => return
    };

    match info.si_code {
        TRAP_BRKPT|SI_KERNEL => {
            state.internal.breakpoint = true;
            state.last_signal = None
        },
        _ => state.internal.breakpoint = false
    };

    let mut regs = match get_registers(pid) {
        Ok(regs) => regs,
        Err(_) => return
    };
    if state.internal.breakpoint {
        regs.rip = regs.rip -1;
        let _ = set_registers(PID.access().unwrap(), regs);
    }
    REGISTERS.sets(regs);

    MAPS.sets(get_process_maps(PROC_PATH.access().as_ref().unwrap()).unwrap());

    let _ = match &state.internal.source_step {
        Some(breakpoints) => {
            let res = breakpoints.disable_all();
            state.internal.source_step = None;
            res
        },
        None => if !state.internal.stopped {BREAKPOINTS.access().as_mut().unwrap().disable_all()} else {Ok(())}
    };
    state.internal.stopped = true;


    let assembly_task = if ui::check_for_assembly(state) { // performance reasons
        assembly_update(state, regs.rip)
    } else {
        None
    };

    if state.internal.no_debug {
        *task = assembly_task;
        return;
    }
    let mut pane_task = None;

    let bind = LINES.access();
    let file = bind.as_ref().unwrap().get_line(regs.rip);
    state.internal.file = file.map(|index| index.clone());
    drop(bind);
    let code_task = ui::code_panes_update(state, &mut pane_task);
    match code_task {
        Some(mut inner) => {
            match assembly_task {
                Some(asm_inner) => inner.push(asm_inner),
                None => ()
            };
            pane_task = Some(iced::Task::batch(inner))
        },
        None => pane_task = assembly_task
    }
    match &state.internal.file {
        None => *task = pane_task,
        Some(file) => if Some(file.hash_path.clone()) != state.internal.comp_dir {
            *task = pane_task
        } else {
            match pane_task {
                Some(pane) => *task = Some(pane.chain(task_stack())),
                None => *task = Some(task_stack())
            };
        }
    }
}

fn reset() {
    STDIO.none();
    PID.none();
    PROC_PATH.none();
    EXEC_SHIFT.none();
    MEMORY.none();
    REGISTERS.none();
    MAPS.none();
}

fn reset_file(state: &mut window::State) -> Result<(), ()> {
    if PID.access().is_some() {
        match Dialog::warning_choice("The program is still running. Are you sure you want to stop the process and discard of the file?", None) {
            rfd::MessageDialogResult::No => return Err(()),
            _ => ()
        }
        reset();
    }
    state.internal.comp_dir = None;
    state.internal.file = None;
    state.internal.stopped = false;
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
    Ok(())
}

fn state_cont(state: &mut window::State) {
    state.last_signal = None;
    state.internal.breakpoint = false;
    state.internal.file = None;
}

fn assembly_update(state: &mut window::State, rip: u64) -> Option<iced::Task<window::Message>> {
    let mut task = None;
    let range = match get_map_range(rip) {
        Some(range) => range,
        None => return None
    };
    let base = (rip - rip%8 - 512).max(range.start);
    let end = (base + 1024).min(range.end);
    let size = end - base;

    let bytes = match read_memory(base, size as usize) {
        Ok(data) => data,
        Err(()) => return None
    };

    let (pointer, line) = match align_pointer(base, rip, &bytes) {
        Ok(value) => value,
        Err(()) => return None
    };

    let assembly = match disassemble_code(pointer, &bytes[(pointer - base) as usize..]) {
        Ok(assembly) => assembly,
        Err(()) => return None
    };

    state.internal.assembly = Some(assembly);

    ui::assembly_scroll(state, line, &mut task);

    task
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
    let bind = MAPS.access();
    let maps = bind.as_ref().unwrap();
    for map in maps {
        if map.range.contains(&address) {
            return Some(map.range.clone());
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

//fn set_register_value(pid: Pid, register: Register) -> Result<(), ()> {
//    let mut regs = get_registers(pid)?;
//
//    match register {
//        Register::RAx(value) => regs.rax = value,
//        Register::RBx(value) => regs.rbx = value,
//        Register::RCx(value) => regs.rcx = value,
//        Register::RDx(value) => regs.rdx = value,
//        Register::RSi(value) => regs.rsi = value,
//        Register::RDi(value) => regs.rdi = value,
//        Register::RBp(value) => regs.rbp = value,
//        Register::RSp(value) => regs.rsp = value,
//        Register::R8(value) => regs.r8 = value,
//        Register::R9(value) => regs.r9 = value,
//        Register::R10(value) => regs.r10 = value,
//        Register::R11(value) => regs.r11 = value,
//        Register::R12(value) => regs.r12 = value,
//        Register::R13(value) => regs.r13 = value,
//        Register::R14(value) => regs.r14 = value,
//        Register::R15(value) => regs.r15 = value,
//        Register::RIP(value) => regs.rip = value
//    };
//
//    set_registers(pid, regs)?;
//    Ok(())
//}

fn kill_tracee(pid: Pid) -> Result<(), ()> {
    close_memory();
    match ptrace::kill(pid) {
        Ok(()) => (),
        Err(err) => {Dialog::error(&format!("Could not stop the tracee: {}", err), Some("Trace error")); return Err(());}
    };
    let _ = object::close_child_stdio();
    Ok(())
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

//pub fn continue_tracee(pid: Pid) -> Result<(), ()> { //WRAPPER
//    restart_tracee(pid, None)
//}

pub fn step(pid: Pid, signal: Option<Signal>) -> Result<(), ()> {
    match ptrace::step(pid, signal) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not step the tracee program: {}", err), Some("Trace error")); Err(())}
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

//fn write_memory(address: u64, buf: &[u8]) -> Result<(), ()> {
//    let mut bind = MEMORY.access();
//    let mut memory = bind.as_mut().unwrap();
//
//    seek_memory(address, &mut memory)?;
//
//    match memory.write_all(buf) {
//        Ok(_) => Ok(()),
//        Err(err) => {Dialog::error(&format!("Could not seek into the memory file: {}", err), Some("Memory error")); Err(())}
//    }
//}

pub fn wait(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> {
    wait::waitpid(pid, None)
}

pub async fn wait_async(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> { // async wrapper
    wait(pid)
}

//pub fn testwait(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> {
//    nix::sys::wait::waitpid(pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG))
//}

fn get_sig_info(pid: Pid) -> Result<nix::libc::siginfo_t, ()> {
    match ptrace::getsiginfo(pid) {
        Ok(info) => Ok(info),
        Err(err) => {Dialog::error(&format!("Could not get signal info of the tracee program: {}", err), Some("Trace error")); Err(())}
    }
}

fn send_signal(pid: Pid, signal: Signal) -> Result<(), ()> {
    signal::kill(pid, signal).map_err(|_| ())
}

fn test_pid(pid: Pid) -> bool {
    signal::kill(pid, None).is_ok()
}