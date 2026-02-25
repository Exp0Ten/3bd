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


// write anything in this function you wanna test, then just call it in main.rs, fn main
pub fn test() {
    first(); //load
    second(); //dwarf
    third();

    let s = SOURCE.access();
    let source = s.as_ref().unwrap();

    println!("{:?}", source.keys());
    for key in source.keys() {
        println!("{:?}", source.get(key).unwrap())
    }

    let l = LINES.access();
    let lines = l.as_ref().unwrap();

    let file = source.get_file("/home/azi/debug/test/languages/c".into(), "main.c".into()).unwrap();

    let address = lines.get_address(&SourceIndex {
        line: 11,
        hash_path: "/home/azi/debug/test/languages/c".into(),
        index: file.1
    }).unwrap();
    println!("{address}");
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
    EHFRAME.sets(EhFrame::new(object));
}

fn third() {
    load_source(DWARF.access().as_ref().unwrap().dwarf(EHFRAME.access().as_ref().unwrap().endian));
}