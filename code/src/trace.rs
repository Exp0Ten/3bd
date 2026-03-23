use std::{
    fs::File,
    io::{Read, Seek},
    path::PathBuf,
    ffi::c_void,
    collections::HashMap
};

use nix::{
    sys::{
       ptrace,
       wait,
       signal::{self, Signal,}
    },
    unistd::Pid,
    libc::user_regs_struct
};


use ::object as object_foreign;

use object_foreign::Object;

// internal import
use crate::{
    data::*,
    dwarf::*,
    object,
    ui,
    window,
    window::Dialog
};


/// FILE: trace.rs - Tracing the program and kernel level debbuging

// Signal info constants for hitting a breakpoint
const SI_KERNEL: i32 = 0x80;
const TRAP_BRKPT: i32 = 1;


pub type Breakpoints = HashMap<u64, u8>;

pub trait ImplBreakpoints {
    fn add(&mut self, address: u64, byte: u8);
    fn add_future(&mut self, address: u64);
    fn rem(&mut self, address: u64) -> u8;
    fn _is_active(&self, address: u64) -> bool;
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

    fn _is_active(&self, address: u64) -> bool {
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

    fn enable_all(&mut self) -> Result<(), ()> { // we insert all of the breakpoints in the programs memory, and save the replaced bytes
        let copy = self.clone();
        let keys = copy.keys();
        for key in keys {
            let byte = insert_breakpoint(PID.access().unwrap(), anti_normal(*key)); // antinormal, because we are only saving the normalized values in the BREAKPOINTS addresses
            match byte {
                Ok(byte) => *self.get_mut(key).unwrap() = byte,
                Err(()) => {self.remove(key);}
            };
        };
        Ok(())
    }
}

// Program Memory Maps
#[derive(Debug)]
pub struct MapBits {
    pub _r: bool,
    pub _w: bool,
    pub _x: bool,
    _p: bool
}

#[derive(Debug)]
pub struct MemoryMap {
    pub name: String,
    pub range: std::ops::Range<u64>, // memory address range
    _permissions: MapBits,         // rwxp (read, write, execute, private)
    pub offset: u64,               // into file
}

#[derive(Debug, Clone)]
pub enum Operation {
    LoadFile,
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
    _ResetFile,
    Read(Result<(Vec<u8>, usize), ()>),
    Stack(Result<Vec<(usize, String)>, ()>)
}

// Tasks definition (to perform async)

fn task_wait() -> iced::Task<window::Message> { // task_wait creates an async thread to wait for the tracee to stop (by signal, breakpoint or user)
    iced::Task::perform(wait_async(PID.access().unwrap()), |result| window::Message::Operation(Operation::HandleSignal(result)))
}

fn task_reset() -> iced::Task<window::Message> {
    iced::Task::done(window::Message::Operation(Operation::Reset))
}

fn task_read() -> iced::Task<window::Message> {
    iced::Task::perform(object::read_stdout(), |result| window::Message::Operation(Operation::Read(result)))
}

pub fn task_content(file: PathBuf, index: SourceIndex, pane: Option<iced::widget::pane_grid::Pane>) -> iced::Task<window::Message> {
    iced::Task::perform(async {ui::source_content(file)}, move |content| if content.is_some() {
        window::Message::Pane(ui::PaneMessage::CodeLoad(pane, index.clone(), content.unwrap()))
    } else {
        window::Message::None
    })
}

pub fn task_breapoints(comp_dir: PathBuf, index: usize, len: usize, pane: iced::widget::pane_grid::Pane) -> iced::Task<window::Message> {
    iced::Task::perform(async move {ui::create_breakpoints(comp_dir, index, len)}, move |result| window::Message::Pane(ui::PaneMessage::CodeBreakpoints(pane, result)))
}

fn task_stack() -> iced::Task<window::Message> {
    iced::Task::perform(async {CallStack::stack_lines(call_stack())}, |result| window::Message::Operation(Operation::Stack(result)))
}

fn task_assembly(rip: u64) -> iced::Task<window::Message> {
    iced::Task::perform(async move {Assembly::create(rip)}, |result| window::Message::Pane(ui::PaneMessage::AssemblyUpdate(result)))
}

fn task_stack_update(id: iced::widget::pane_grid::Pane) -> iced::Task<window::Message> {
    iced::Task::done(window::Message::Pane(ui::PaneMessage::StackUpdate(id)))
}



// Inner Tracing Logic

pub fn operation_message(state: &mut window::State, operation: Operation, task: &mut Option<iced::Task<window::Message>>) {
    match operation {
        Operation::LoadFile => {
            let reset = FILE.access().is_some();

            let file = match Dialog::file(None, None) { // user file selection
                Some(file) => file,
                None => return
            };
            if reset { // if a different file was selected, then we clear the Globals
                if reset_file(state).is_err() {
                    return;
                };
            }

            match object::test_file(&file) { // we test the file, if its an executable
                Ok(_) => (),
                Err(()) => return
            };

            let data = object::read_file(&file); // we read the file contents and set the data to the GLOBAL
            unsafe {
                DATA = data
            }
            FILE.sets(file.clone()); // setting the path to the new file

            let no_debug = dwarf_set(state).is_err(); // preloading all dwarf related data, err when no debug information
            state.internal.no_debug = no_debug;
            if no_debug {
                Dialog::warning(&format!("This file ({}) does not contain debbuging data.", file.file_name().unwrap().to_str().unwrap()), None);
                BREAKPOINTS.sets(Breakpoints::new());
                return;
            }
            panes_preload(state, task); // preloading the panes (code widgets with the main file of the binary)
        },

        Operation::RunTracee => {
            let stdio = match object::open_child_stdio() {
                Ok(stdio) => stdio,
                Err(()) => return
            };

            let pid = match object::run_tracee(FILE.access().as_ref().unwrap(), Vec::new(), Some(stdio)) {
                Err(_) => return,
                Ok(pid) => {PID.sets(Pid::from_raw(pid)); Pid::from_raw(pid)}, // we save the tracee pid to the global
            };
            tracee_setup(state, pid, task); // we setup the tracee data

            *task = Some(task_read()); // we launch the reading from the PTY
        },
        Operation::StopTracee => {
            if PID.access().is_none() {
                return;
            };
            match kill_tracee(PID.access().unwrap()) {
                Ok(_) => (),
                Err(()) => return
            };

            *task = Some(task_reset()); // we reset the Trace data and globals
        },

        Operation::Step => {
            if step(PID.access().unwrap(), state.last_signal).is_err() {return;};
            state_cont(state);
            *task = Some(task_wait())
        },
        Operation::SourceStep => {
            let pid = PID.access().unwrap();
            if step(pid, None).is_err() {return;}; // we step away from the last line
            let _ = wait(pid);
            let mut breakpoints = Breakpoints::new(); // we create temporary breakpoints, where all addresses in the LINES get a breakpoint
            let bind = LINES.access();
            for (address, _) in bind.as_ref().unwrap().iter() { //we breakpoint every line for a single wait call, whatever the program stops at, we disable them again
                let byte = insert_breakpoint(pid, anti_normal(*address)).unwrap();
                breakpoints.add(*address, byte);
            }
            if restart_tracee(pid, None).is_err() { // we try continuing the program
                breakpoints.disable_all().unwrap();
                return;
            };
            state.internal.source_step = Some(breakpoints); // we save the breakpoints
            state.internal.stopped = false;
            state_cont(state); // we set the state to running
            *task = Some(task_wait()) // and wait for the next stop
        },
        Operation::Pause => {
            if send_signal(PID.access().unwrap(), Signal::SIGTRAP).is_err() { // we manually stop the program (as if it hit a breakpoint)
                return;
            };
            state.internal.manual = true; // we set the manual so we know the program stopped because of us
        },
        Operation::Continue => {
            let pid = PID.access().unwrap();
            if step(pid, None).is_err() {return;}; // we step away from the current line (as to not hit the same breakpoint again)
            let _ = wait(pid);
            if BREAKPOINTS.access().as_mut().unwrap().enable_all().is_err() { // we enable all of the breakpoints
                *task = Some(task_reset()); // if that fails we reset the file
                return;
            };
            if restart_tracee(pid, state.last_signal).is_err() { // and we continue the tracee
                if BREAKPOINTS.access().as_mut().unwrap().disable_all().is_err() {
                    *task = Some(task_reset())
                };
                return;
            };

            *task = Some(task_wait()); // we wait for the next stop

            state_cont(state);
            state.internal.stopped = false;
            state.internal.manual = false; // and manual gets reset
        },
        Operation::Kill => {
            if state.internal.stopped {
                state.last_signal = Some(Signal::SIGKILL);
                return;
            }
            if send_signal(PID.access().unwrap(), Signal::SIGKILL).is_err() { // if not stopped, we just send the signal right away
                return;
            };
        },
        Operation::Signal(sig) => {
            if state.internal.stopped {
                state.last_signal = Some(sig);
                return;
            }

            let pid = PID.access().unwrap();
            if send_signal(pid, Signal::SIGSTOP).is_err() { // if not stopped, we stop the tracee
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
        }, // if we encounter an error while waiting for the tracee to stop, we give the user an option to try again

        Operation::Reset => { // tracee state reset
            state.internal.stopped = false;
            state.internal.source_step = None;
            state.internal.manual = false;
            state.internal.breakpoint = false;
            state.internal.pane.file = None;
            state.internal.pane.output.clear();
            state.internal.pane.stack = None;
            state.last_signal = None;
            reset();
        },
        Operation::_ResetFile => {let _ = reset_file(state);}, // complete file 

        Operation::Read(result) => {
            if result.is_err() {
                return;
            }

            *task = Some(task_read()); // we launch a new read, creating an async loop

            let data = result.unwrap();
            if data.1 == 0 {
                return;
            }

            let mut text: String = data.0[..data.1].iter().map(|byte| *byte as char).collect(); // we create the string from the bytes

            ui::process_string(&mut text);

            state.internal.pane.output.push_str(&text); // pushing it into the output
        },
        Operation::Stack(result) => {
            match result {
                Ok(stack) => state.internal.pane.stack = Some(stack),
                Err(()) => {state.internal.pane.stack = None; return;}
            }

            state.internal.pane.unique_stack += 1; // We create a new unique (it doesnt have to be a random one, just a different one every time)
            // we need it because the PaneStack might be hidden due to sidebars, so we need a way to tell, if a pane has old data or not

            let mut tasks = Vec::new();

            for (id, pane) in state.layout.panes.iter() { // updating all of the active stack panes
                match pane {
                    ui::Pane::Stack(..) => tasks.push(task_stack_update(*id)),
                    _ => ()
                }
            }

            *task = Some(iced::Task::batch(tasks));
        },
    };
}

// setup functions

fn dwarf_set(state: &mut window::State) -> Result<(), ()> {
    #[allow(static_mut_refs)]
    let data = unsafe { // static reference
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

    match &object {
        object_foreign::File::Elf64(elf) => { // we check for static executable
            if elf.elf_header().e_type.get(object.endianness()) == 2 {
                state.internal.static_exec = true;
            }
        },
        _ => ()
    }

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
    state.internal.pane.comp_dir = Some(PathBuf::from(comp_dir.clone()));
    let mut tasks = Vec::new();

    for (id, pane) in panes.iter_mut() { // for every active code pane, select the main file of the program
        match pane {
            ui::Pane::Code(inner) => {
                inner.dir = Some(comp_dir.clone());
                tasks.push(iced::Task::done(window::Message::Pane(ui::PaneMessage::CodeSelectFile(*id, main_file.clone()))));
            },
            _ => ()
        }
    }
    *task = Some(iced::Task::batch(tasks)); // batch allowing for multiple code panes
}

fn tracee_setup(state: &mut window::State, pid: Pid, task: &mut Option<iced::Task<window::Message>>) {
    let proc_path = PathBuf::from(format!("/proc/{pid}/"));
    state.status = Some(wait(pid).unwrap());
    let path = get_tracee_path(&proc_path).unwrap();
    FILE.sets(path.clone());

    let maps = get_process_maps(&proc_path).unwrap();

    for map in &maps { // we find our exec shift, unless its a static executable
        if map.name != path.to_str().unwrap() {
            continue;
        }
        if map.offset == 0 {
            if state.internal.static_exec {break;}
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

    for (_id, pane) in panes.iter_mut() { // for every active memory pane we set the address to the antinormal
        match pane {
            ui::Pane::Memory(inner) => {
                inner.address = anti_normal(0);
                inner.field = ui::Base::form(&ui::Base::Hex, anti_normal(0));
                ui::update_memory(inner);
            },
            _ => ()
        }
    };

    if ui::check_for_assembly(state) { // and we update every active assembly pane
        *task = assembly_update(state, REGISTERS.access().unwrap().rip);
    }
}


fn handle(state: &mut window::State, status: wait::WaitStatus, task: &mut Option<iced::Task<window::Message>>) { // handling signals
    state.status = Some(status);
    match status {
        wait::WaitStatus::Exited(pid, exit) => {
            *task = Some(iced::Task::done(window::Message::Operation(Operation::Reset)));
            Dialog::info(&format!("Program exited with the code: {:-}.\nPid: {}", exit, pid), Some("Program exited"));
            return;
        },
        wait::WaitStatus::Signaled(pid, signal, _) => {
            if !test_pid(pid) {
                Dialog::warning(&format!("Program with pid {} was terminated by signal: {}", pid, signal), Some("Program ended"));
                *task = Some(task_reset());
                return;
            }
            if !state.internal.manual { // if manual, we dont want to display the signal
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
    match info.si_code { // if breakpoint, we remove the last signal
        TRAP_BRKPT|SI_KERNEL => {
            state.internal.breakpoint = true;
            state.last_signal = None
        },
        _ => state.internal.breakpoint = false
    };

    let mut regs = match get_registers(pid) { // new regs
        Ok(regs) => regs,
        Err(_) => return
    };
    if state.internal.breakpoint { // if breakpoint we have to move a byte back (because we stopped a byte further)
        regs.rip = regs.rip -1;
        let _ = set_registers(PID.access().unwrap(), regs);
    }
    REGISTERS.sets(regs);

    MAPS.sets(get_process_maps(PROC_PATH.access().as_ref().unwrap()).unwrap()); // new maps

    let _ = match &state.internal.source_step { // if sourcestep was active, we want to disable all of the temporary
        Some(breakpoints) => {
            let res = breakpoints.disable_all();
            state.internal.source_step = None;
            res
        },
        None => if !state.internal.stopped {BREAKPOINTS.access().as_mut().unwrap().disable_all()} else {Ok(())} // otherwise disable the normal breakpoints, but only if we ever ran the program (not stepping)
    };
    state.internal.stopped = true;

    let mut tasks = Vec::new();

    if ui::check_for_assembly(state) { // performance reasons
        tasks.push(task_assembly(regs.rip)); // updating assembly
    };

    if state.internal.no_debug { // if no debug, end here
        *task = Some(iced::Task::batch(tasks));
        return;
    }

    let bind = LINES.access();
    let file = bind.as_ref().unwrap().get_line(regs.rip);
    state.internal.pane.file = file.map(|index| index.clone());
    drop(bind);

    if ui::check_for_code(state) { // if any active code panes
        if let Some((scroll, load)) = ui::code_panes_update(state) { // if there are any updates
            let index = state.internal.pane.file.as_ref().unwrap();
            let bind = SOURCE.access();
            let source = bind.as_ref().unwrap().index_with_line(index);
            let mut file = index.hash_path.clone();
            file.push(source.path.clone());

            if source.content.is_none() { // if content is empty, we load it
                tasks.push(task_content(file, index.clone(), None).chain(load.chain(scroll)));
            } else {
                tasks.push(load.chain(scroll)); // otherwise just scroll the code panes
            }
        };
    }

    match &state.internal.pane.file { // if we are stopped at a line, create the callstack
        Some(_) => {
            tasks.push(task_stack());
        },
        None => ()
    }

    *task = Some(iced::Task::batch(tasks)); // return all of the tasks
}

fn reset() { // reset TRACE globals
    STDIO.none();
    PID.none();
    PROC_PATH.none();
    EXEC_SHIFT.none();
    MEMORY.none();
    REGISTERS.none();
    MAPS.none();
}

fn reset_file(state: &mut window::State) -> Result<(), ()> { // reset selected file and GLOBALS
    if PID.access().is_some() {
        match Dialog::warning_choice("The program is still running. Are you sure you want to stop the process and discard of the file?", None) {
            rfd::MessageDialogResult::No => return Err(()),
            _ => ()
        }
        reset();
    }
    state.internal.pane.comp_dir = None;
    state.internal.pane.file = None;
    state.internal.pane.stack = None;
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

fn state_cont(state: &mut window::State) { // set state to continue
    state.last_signal = None;
    state.internal.breakpoint = false;
    state.internal.pane.file = None;
}

fn assembly_update(state: &mut window::State, rip: u64) -> Option<iced::Task<window::Message>> { // while setup, we load the assembly and scroll to the rip address
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

    state.internal.pane.assembly = Some(assembly);

    ui::assembly_scroll(state, line, &mut task);

    task
}


// Tracing Interface

pub fn open_memory(proc_path: &PathBuf) -> Result<File, ()> { // opens the memory file from the proc_fs, (creating the access to the tracees memory)
    let mut path = proc_path.clone();
    path.push("mem");
    match File::open(path) {
        Ok(file) => Ok(file),
        Err(err) => {Dialog::error(&format!("Could not open memory of the tracee: {}", err), Some("Trace Error")); Err(())}
    }
}

fn close_memory() { // closes the file by dropping the FD and reference
    *MEMORY.access() = None;
}

pub fn get_process_maps(proc_path: &PathBuf) -> Result<Vec<MemoryMap>, ()> { // opens and reads the process memory maps
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

    for line in lines { // parsing the map entries
        mmap_vector.push({
            if line == "" {continue;} // skipping empty lines

            let mut split = line.split_ascii_whitespace();

            let mut range_split: std::str::Split<'_, &str> = split.next().unwrap().split("-");
            let range: (u64, u64) = (u64::from_str_radix(range_split.next().unwrap(), 16).unwrap(), u64::from_str_radix(range_split.next().unwrap(), 16).unwrap());

            let range = range.0..range.1;

            let permissions_split = split.next().unwrap();
            let _permissions = MapBits {
                _r: permissions_split[0..1] == *"r",
                _w: permissions_split[1..2] == *"w",
                _x: permissions_split[2..3] == *"x",
                _p: permissions_split[3..4] == *"p",
            };

            let offset = u64::from_str_radix(split.next().unwrap(), 16).unwrap();

            split.next();
            split.next();

            let name = split.next().unwrap_or("").to_string();

            if name == "" {continue;}

            MemoryMap {
                name,
                range,
                _permissions,
                offset
            }
        });
    };
    Ok(mmap_vector)
}

pub fn get_map_range(address: u64) -> Option<std::ops::Range<u64>> { // we iterate through the maps, returning the coresponding map, if we find any
    let bind = MAPS.access();
    let maps = bind.as_ref().unwrap();
    for map in maps {
        if map.range.contains(&address) {
            return Some(map.range.clone());
        }
    };
    None
}

pub fn get_tracee_path(proc_path: &PathBuf) -> Result<PathBuf, ()> { // we reload the path as the full and real path to the file (from proc_fs)
    let mut path = proc_path.clone();
    path.push("exe");
    match std::fs::read_link(path) {
        Ok(path) => Ok(path),
        Err(err) => {Dialog::error(&format!("Could not get tracee's path: {}", err), Some("Trace error")); Err(())}
    }
}


pub fn insert_breakpoint(pid: Pid, address: u64) -> Result<u8, ()> { // we first save the byte, then replace the it with the 0XCC byte (producing a SIGTRAP and breakpoint)
    let save = match ptrace::read(pid, address as *mut c_void) {
        Ok(long) => long as u64,
        Err(err) => {Dialog::error(&format!("Could not insert breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    };

    match ptrace::write(pid, address as *mut c_void, (0xcc | (save & 0xffffffffffffff00)) as i64) {
        Ok(()) => Ok((save & 0xff) as u8),
        Err(err) => {Dialog::error(&format!("Could not insert breakpoint at {}: {}", address, err), Some("Trace error")); Err(())}
    }
}

pub fn remove_breakpoint(pid: Pid, address: u64, byte: u8) -> Result<(), ()> { // we insert back the saved byte (removing the breakpoint)
    let save = match ptrace::read(pid, address as *mut c_void) {
        Ok(long) => long as u64,
        Err(err) => {Dialog::error(&format!("Could not remove breakpoint at {}: {}", address, err), Some("Trace error")); return Err(());}
    };

    match ptrace::write(pid, address as *mut c_void, (byte as u64 | (save & 0xffffffffffffff00)) as i64) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not remove breakpoint at {}: {}", address, err), Some("Trace error")); Err(())}
    }
}

pub fn get_registers(pid: Pid) -> Result<user_regs_struct, ()> { // wrapper for PTRACE_GETREGS
    match ptrace::getregs(pid) {
        Ok(regs) => Ok(regs),
        Err(err) => {Dialog::error(&format!("Could not get register values: {}", err), Some("Trace error")); Err(())}
    }
}

fn set_registers(pid: Pid, regs: user_regs_struct) -> Result<(), ()> { // wrapper for PTRACE_SETREGS
    match ptrace::setregs(pid, regs) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not set register values: {}", err), Some("Trace error")); Err(())}
    }
}

fn kill_tracee(pid: Pid) -> Result<(), ()> { // wrapper for PTRACE_KILL
    close_memory();
    match ptrace::kill(pid) {
        Ok(()) => (),
        Err(err) => {Dialog::error(&format!("Could not stop the tracee: {}", err), Some("Trace error")); return Err(());}
    };
    let _ = object::close_child_stdio(); // closing the child stdio
    Ok(())
}

fn restart_tracee(pid: Pid, signal: Option<Signal>) -> Result<(), ()> { // wrapper for PTRACE_CONT, also used for signaling the tracee,
    match ptrace::cont(pid, signal) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not deliver the signal to the tracee: {}", err), Some("Trace error")); Err(())}
    }
}

fn signal_tracee(pid: Pid, signal: Signal) -> Result<(), ()> { // wrapper for PTRACE_CONT with a signal
    restart_tracee(pid, Some(signal))
}

pub fn step(pid: Pid, signal: Option<Signal>) -> Result<(), ()> { // wrapper for PTRACE_STEP
    match ptrace::step(pid, signal) {
        Ok(()) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not step the tracee program: {}", err), Some("Trace error")); Err(())}
    }
}


fn seek_memory(address: u64, memory_file: &mut File) -> Result<(), ()> { // seeking into the memory file
    match memory_file.seek(std::io::SeekFrom::Start(address)) {
        Ok(_) => Ok(()),
        Err(err) => {Dialog::error(&format!("Could not seek into the memory file: {}", err), Some("Memory error")); Err(())}
    }
}

pub fn test_memory(address: u64) -> Result<(), bool> { // tests if the address is in valid memory space // if Err(true) -> start, if Err(false) -> End, if Ok() then we are in valid memory space
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

pub fn read_memory(address: u64, amount: usize) -> Result<Vec<u8>, ()> { // reads from the memory file
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


pub fn wait(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> { // waits for the next signal
    wait::waitpid(pid, None)
}

pub async fn wait_async(pid: Pid) -> Result<wait::WaitStatus, nix::errno::Errno> { // async wrapper for wait
    wait(pid)
}


fn get_sig_info(pid: Pid) -> Result<nix::libc::siginfo_t, ()> { // wrapper for PTRACE_GETSIGINFO
    match ptrace::getsiginfo(pid) {
        Ok(info) => Ok(info),
        Err(err) => {Dialog::error(&format!("Could not get signal info of the tracee program: {}", err), Some("Trace error")); Err(())}
    }
}

fn send_signal(pid: Pid, signal: Signal) -> Result<(), ()> { // wrapper for kill(pid, signal)
    signal::kill(pid, signal).map_err(|_| ())
}

fn test_pid(pid: Pid) -> bool { // testing if the program with PID is still alive
    signal::kill(pid, None).is_ok()
}