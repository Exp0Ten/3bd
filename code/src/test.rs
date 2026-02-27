#![allow(unused)]

//  FUNCTION SAV
fn fork_test() {
    use nix::unistd::{fork, ForkResult};
    fn child_fn() {
        println!("Hi, im doing good");
    }

    fn parent_fn(pid: i32) {
        println!("Hi {pid}, how are you doing?", )
    }

    println!("hiii");
    let fork = match unsafe {fork()} {
        Ok(pid) => pid,
        Err(e) => panic!(),
    };
    match fork {
        ForkResult::Child => child_fn(),
        ForkResult::Parent {child, ..} => parent_fn(child.as_raw())
    };
    println!("I can also do this.")
}

// using:
use crate::data::*;
use crate::object::*;
use crate::dwarf::*;
use crate::trace;
use crate::trace::*;
use object::Object;
use object::ObjectSymbol;
use nix::sys::ptrace;
use std::os::raw::c_void;

// write anything in this function you wanna test, then just call it in main.rs, fn main
pub fn test() {
    first(); //load
    second(); //dwarf
    third(); //line
    fourth(); // functions
    //WORKING
    fith(); //exec, mem, breakpoints
    sixth(); //callstack
    // WORKING TESTED FOR C
    seventh()
}

fn first() {
    let path = std::path::Path::new("a.out");
    let data = crate::object::read_file(path);
    println!("{:}", data.len());

    unsafe {
        DATA = data;
    }
    FILE.sets(path.into());
}

fn second() {
    #[allow(static_mut_refs)]
    let data = unsafe {
        &DATA
    };

    let (dwarf, object) = load_dwarf(data);
    DWARF.sets(dwarf);
    ENDIAN.sets(match object.endianness() {
        object::Endianness::Little => Endian::Little,
        object::Endianness::Big => Endian::Big
    });
    EHFRAME.sets(EhFrame::new(object));
}

fn third() {
    load_source(DWARF.access().as_ref().unwrap().dwarf(ENDIAN.access().unwrap()));

    for i in LINES.access().as_ref().unwrap().keys() {
        println!("{:x}", i)
    }
}

fn fourth() {
    let dwarf = DWARF.access();
    parse_functions(dwarf.as_ref().unwrap().dwarf(ENDIAN.access().unwrap()));
}

fn fith() {
    let pid = crate::object::run_tracee(FILE.access().as_ref().unwrap(), Vec::new(), None).expect("");
    let pid = nix::unistd::Pid::from_raw(pid);
    let source = SOURCE.access();
    let lines = LINES.access();
    let breakpoints = trace::Breakpoints::new();
    BREAKPOINTS.sets(breakpoints);

    find_main();
    let main= EHFRAME.access().as_ref().unwrap().object.symbol_by_name("main").unwrap().address();

    BREAKPOINTS.access().as_mut().unwrap().add_future(main);

    PID.sets(pid);

    let proc_path = std::path::PathBuf::from(format!("/proc/{pid}/"));

    println!("child: {pid}");
    let status = trace::wait(pid).unwrap();
    println!("status: {status:?}");

    let path = trace::get_tracee_path(&proc_path).unwrap();
    FILE.sets(path); // update, now with the full real path

    let maps = trace::get_process_maps(&proc_path).unwrap();

    for map in maps {
        if map.name != FILE.access().as_ref().unwrap().to_str().unwrap() {
            continue;
        }
        if map.permissions.x {
            EXEC_SHIFT.sets(map.range.start - map.offset);
            break;
        }
    };

    let file = trace::open_memory(&proc_path).unwrap();

    MEMORY.sets(file);

    let mut breakpoints = BREAKPOINTS.access();




    breakpoints.as_mut().unwrap().enable_all();

    trace::continue_tracee(pid).unwrap();
    let status = trace::wait(pid).unwrap();
    println!("status: {status:?}");

    let mut rip = get_registers(pid).unwrap().rip-1;
    set_register_value(pid, Register::RIP(rip));
    println!("{rip}");
    println!("{}", EXEC_SHIFT.access().unwrap());
    println!("{}, {}", main, normal(rip));
    let line = lines.as_ref().unwrap().get_line(rip).unwrap();
    let source_file = source.as_ref().unwrap().index_with_line(line);
    println!("{}: {}, {}", rip, line.line, source_file.path.display());


    breakpoints.as_ref().unwrap().disable_all();

    let (rip, line) = trace::source_step(pid, lines.as_ref().unwrap()).unwrap();

    let source_file = source.as_ref().unwrap().index_with_line(line);
    println!("{}: {}, {}", rip, line.line, source_file.path.display());


    let line = source.as_ref().unwrap().get_file("/home/azi/debug/test/languages/c".into(), "other.c".into()).unwrap();
    let index = SourceIndex {
        line: 3,
        hash_path: "/home/azi/debug/test/languages/c".into(),
        index: line.1
    };
    let address = lines.as_ref().unwrap().get_address(&index).unwrap();
    breakpoints.as_mut().unwrap().add_future(address);

    breakpoints.as_mut().unwrap().enable_all();
    trace::continue_tracee(pid);
    let status = trace::wait(pid).unwrap();
    println!("status: {status:?}");

    let mut rip = get_registers(pid).unwrap().rip-1;
    set_register_value(pid, Register::RIP(rip));
    let line = lines.as_ref().unwrap().get_line(rip).unwrap();
    let source_file = source.as_ref().unwrap().index_with_line(line);
    println!("{}: {}, {}", rip, line.line, source_file.path.display());
    REGISTERS.sets(trace::get_registers(pid).unwrap());
}

fn sixth() {
    let callstack = call_stack().unwrap(); // WORKSSSSSSSSSSSS YESSSSSSSSSSSSSSSS
    let bind = DWARF.access();
    let dwarf = bind.as_ref().unwrap().dwarf(ENDIAN.access().unwrap());
    let first = &callstack.0[0];
    let var = &first.parameters.as_ref().unwrap()[0];
    let typ = var.vtype;
    let typedisplay = unwind_type(typ, &dwarf);
    let basetype = match typedisplay {
        TypeDisplay::Base(base) => base,
        _ => unimplemented!()
    };

    let size = match basetype.size {
        BitByteSize::Byte(val) => val,
        _ => unimplemented!()
    };

    let location = match var.location {
        Location::Address(a) => a,
        Location::Register(_) => unimplemented!()
    };

    let bytes = read_memory(location, size as usize).unwrap();
    println!("{:?}", bytes);


}

fn seventh() {
    let pid = PID.access().unwrap();
    let rip = get_registers(pid).unwrap().rip;
    let bind = BREAKPOINTS.access();
    let breakpoints = bind.as_ref().unwrap();
    trace::step_over(pid, rip, *breakpoints.get(&normal(rip)).unwrap());
    trace::continue_tracee(pid);
    let status = wait(pid).unwrap();
    println!("{status:?}");
}

// SAVE FOR BREAKPOINTS

fn add_breakpoint(address: u64) -> Result<(), ()> {
    let mut bind = BREAKPOINTS.access();
    let breakpoints= bind.as_mut().unwrap();

    let byte = trace::insert_breakpoint(PID.access().unwrap(), address)?;
    breakpoints.add(address, byte);
    Ok(())
}

fn clear_breakpoint(address: u64) -> Result<(), ()> {
    let mut bind = BREAKPOINTS.access();
    let breakpoints= bind.as_mut().unwrap();

    let byte = breakpoints.remove(&address).unwrap();
    trace::remove_breakpoint(PID.access().unwrap(), address, byte)?;
    Ok(())
}


// !!!!! NOTE: ALL PTRACE FUNCTIONS NEED THE PROGRAM TO BE STOPPED !!!!!!!