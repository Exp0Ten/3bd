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

// write anything in this function you wanna test, then just call it in main.rs, fn main
pub fn test() {
}
