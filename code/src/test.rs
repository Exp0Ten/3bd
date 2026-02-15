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
    let file = std::fs::read("/home/azi/debug/test/debug/a.out").unwrap();

    let object = object::File::parse(&*file).unwrap();
    let endian = if object::Object::is_little_endian(&object) {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    let dwarf_sections = load_dwarf_sections(object, endian);

    let dwarf = load_dwarf(&dwarf_sections, endian);

    dwarf_info(dwarf);

}

use gimli::read;
use gimli::Reader;
use object::Object;
use object::ObjectSection;

//struct Section<'data> {
//    data: std::borrow::Cow<'data, [u8]>,
//}

type Section<'data> = std::borrow::Cow<'data, [u8]>;
type Dwarf<'a> = read::Dwarf<read::EndianSlice<'a, gimli::RunTimeEndian>>;

fn load_dwarf_sections(object: object::File, endian: gimli::RunTimeEndian) -> read::DwarfSections<Section> {
    let load_section = |id: gimli::SectionId| -> Result<Section, Box<dyn std::error::Error>> {
        Ok(match object.section_by_name(id.name()) {
            Some(section) => section.uncompressed_data()?,
            None => std::borrow::Cow::Borrowed(&[]),
        })
    };


    let dwarf_sections = gimli::DwarfSections::load(&load_section).unwrap();

    dwarf_sections
}

fn load_dwarf<'a>(dwarf_sections: &'a read::DwarfSections<Section>, endian: gimli::RunTimeEndian) -> read::Dwarf<read::EndianSlice<'a, gimli::RunTimeEndian>> {
    let borrow_section = |section| gimli::EndianSlice::new(Section::as_ref(section), endian);

    let dwarf = dwarf_sections.borrow(borrow_section);
    dwarf
}

fn dwarf_info(dwarf: Dwarf) {
    for header in dwarf.units() {
        let unit = dwarf.unit(header.unwrap()).unwrap();

        let mut entries = unit.entries();
        while let Some(entry) = entries.next_dfs().unwrap_or(None) {
            println!(
            "<{}><{:x}> {}",
            entry.depth(),
            entry.offset().0,
            entry.tag()
        );

        for attr in entry.attrs() {
            print!("   {}: {:?}", attr.name(), attr.value());
            if let Ok(s) = dwarf.attr_line_string(attr.value()) {
                print!(" '{}'", s.to_string().unwrap());
            }
            println!()
        }
        }
    };
}