#![allow(unused)]

// SAV - p1ut saves of functions you wish to use later here (NO PUB) along with their use crate
use std::process::{Command, Stdio};
fn child() {
    let child = Command::new("alacritty")
        .spawn()
        .expect("Alacritty couldnt start");
}
use std::path::Path;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
fn elfheader() {
    let path = Path::new("/home/azi/debug/test/elf/helloworld");
    let mut file = File::open(path).expect("Couldn't open file:");

    //elf-header = 64 bytes
    file.seek(SeekFrom::Start(0));

    let mut buffer:[u8; 64] = [0; 64];
    let bytes_read = file.read(&mut buffer).expect("Couldn't read the file:");
    println!("{bytes_read}");
    println!("{buffer:?}");
    let magic: &str = str::from_utf8(&buffer[0..4]).expect("utf8 err:");
    let format: u8 = buffer[4];
    let endian: u8 = buffer[5];
    let version: u8 = buffer[6];
    let os: u8 = buffer[7];
    let abi_version: u8 = buffer[8];
    //7 pad bytes
    let elf_type: u8 = buffer[16];
    if buffer[17] != 0 {panic!("WHAT DINOSAUR ARE YOU USING D:")}
    let is: u8 = buffer[18];
    if buffer[19] != 0 {panic!("WHAT DINOSAUR ARE YOU USING D:")}
    let elf_version: u32 = u32::from_le_bytes(buffer[20..24].try_into().expect(""));
    let entry: u64 = u64::from_le_bytes(buffer[24..32].try_into().expect(""));
    let program_header: u64 = u64::from_le_bytes(buffer[32..40].try_into().expect(""));
    let section_header: u64 = u64::from_le_bytes(buffer[40..48].try_into().expect(""));
    let flags = &buffer[48..52];
    let size = u16::from_le_bytes(buffer[52..54].try_into().expect(""));
    let program_size = u16::from_le_bytes(buffer[54..56].try_into().expect(""));
    let program_count = u16::from_le_bytes(buffer[56..58].try_into().expect(""));
    let section_size = u16::from_le_bytes(buffer[58..60].try_into().expect(""));
    let section_count = u16::from_le_bytes(buffer[60..62].try_into().expect(""));
    let section_names = u16::from_le_bytes(buffer[62..64].try_into().expect(""));
    // 4 bytes

    println!("magic {magic}");
    println!("format {format}");
    println!("endian {endian}");
    println!("version {version}");
    println!("os {os}");
    println!("abi_version {abi_version}");
    println!("elf_type {elf_type}");
    println!("is {is}");
    println!("elf_version {elf_version}");
    println!("entry 0x{entry:x}");
    println!("program_header {program_header}");
    println!("section_header {section_header}");
    println!("flags {flags:?}");
    println!("size {size}");
    println!("program_size {program_size}");
    println!("program_count {program_count}");
    println!("section_size {section_size}");
    println!("section_count {section_count}");
    println!("section_names {section_names}");

}




// write anything in this function you wanna test, then just call it in main.rs, fn main
pub fn test() {
    
}
