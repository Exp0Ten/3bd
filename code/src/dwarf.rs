use std::path::PathBuf;
use std::collections::HashMap;


use gimli::DebugInfoOffset;
use gimli::UnitHeader;
use gimli::EndianSlice;

use object::{Object, ObjectSection};

use crate::data::*;



#[derive(PartialEq, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    compile_unit: DebugInfoOffset,
    pub content: Option<String>
}

impl SourceFile {
    fn new(rel_path: PathBuf, unit_offset: DebugInfoOffset) -> Self {
        SourceFile {
            path: rel_path,
            compile_unit: unit_offset,
            content: None
        }
    }

    fn get_unit_header<'a>(&self, dwarf: &'a Dwarf) -> UnitHeader<EndianSlice<'a, Endian>> {
        dwarf.debug_info.header_from_offset(self.compile_unit).unwrap()
    }
}

pub type SourceMap = HashMap<PathBuf, Vec<SourceFile>>;

pub trait ImplSourceMap {
    fn get_file(&self, comp_dir: PathBuf, path: PathBuf) -> Option<&SourceFile>;
    fn get_comp_dir(&self, source_file: &SourceFile, dwarf: Dwarf) -> PathBuf;
    fn insert_file(&mut self, source_file: SourceFile, hash_dir: PathBuf, line_number: u64) -> SourceIndex;
    fn index_with_line(&self, line: &SourceIndex) -> &SourceFile;
}

impl ImplSourceMap for SourceMap {
    fn get_file(&self, comp_dir: PathBuf, path: PathBuf) -> Option<&SourceFile> {
        for file in self.get(&comp_dir).unwrap() {
            if file.path == path {
                return Some(file);
            }
        };
        None
    }

    fn get_comp_dir(&self, source_file: &SourceFile, dwarf: Dwarf) -> PathBuf {
        PathBuf::from(
            dwarf.unit(
                source_file.get_unit_header(&dwarf)
            ).unwrap()
            .comp_dir.unwrap()
            .to_string_lossy()
            .into_owned()
        )
    }

    fn insert_file(&mut self, source_file: SourceFile, hash_dir: PathBuf, line_number: u64) -> SourceIndex {
        if self.contains_key(&hash_dir) {
            let v =self.get_mut(&hash_dir).unwrap();
            for i in 0..v.len() {
                if v[i] == source_file {
                    return SourceIndex::new(hash_dir, i, line_number);
                }
            }
            v.push(source_file);
            SourceIndex::new(hash_dir, v.len()-1, line_number)
        } else {
            self.insert(hash_dir.clone(), vec![source_file]);
            SourceIndex::new(hash_dir, 0, line_number)
        }
    }

    fn index_with_line(&self, line: &SourceIndex) -> &SourceFile {
        let vec = self.get(&line.hash_path).unwrap();
        &vec[line.index]
    }
}

#[derive(PartialEq)]
pub struct SourceIndex {
    line: u64, // in the SourceFile
    hash_path: PathBuf,
    index: usize // in the SourceMap Vec
}

impl SourceIndex {
    fn new(hash_path: PathBuf, index: usize, line: u64) -> Self {
        SourceIndex {
            line,
            hash_path,
            index
        }
    }
}

pub type LineAddresses = HashMap<u64, SourceIndex>;

pub trait ImplLineAddresses<'a> {
    fn get_line(&'a self, address: u64) -> Option<&'a SourceIndex>;
    fn get_source_file(&'a self, address: u64) -> Option<SourceFile>;
    fn get_address(&'a self, line: &SourceIndex) -> Option<u64>;
}

impl <'a> ImplLineAddresses<'a> for LineAddresses {
    fn get_line(&'a self, address: u64) -> Option<&'a SourceIndex> {
        self.get(&address)
    }

    fn get_source_file(&self, address: u64) -> Option<SourceFile> {
        match self.get(&address) {
            Some(line) => {
                let ibind = INTERNAL.access();
                let v= ibind.source_files.as_ref().unwrap()[&line.hash_path].clone();
                Some(v.get(line.index).unwrap().clone())
            },
            None => None
        }
    }

    fn get_address(&'a self, line: &SourceIndex) -> Option<u64> {
        let keys = self.keys();
        for key in keys {
            let entry = &self[key];
            if entry == line {
                return Some(*key);
            }
        };
        None
    }
}

type Endian = gimli::RunTimeEndian;
type Section<'data> = std::borrow::Cow<'data, [u8]>;
pub type DwarfSections<'data> = gimli::DwarfSections<Section<'data>>;
pub type Dwarf<'a> = gimli::Dwarf<EndianSlice<'a, gimli::RunTimeEndian>>;

trait SectionsToDwarf {
    fn dwarf(&self, endian: Endian) -> Dwarf;
}

impl SectionsToDwarf for DwarfSections<'_> {
    fn dwarf(&self, endian: Endian) -> Dwarf {
        self.borrow(|section| gimli::EndianSlice::new(Section::as_ref(section), endian))
    }
}

// Reading LINE_DATA

fn load_dwarf(binary: &Vec<u8>) -> (DwarfSections, Endian) {
    let object = object::File::parse(&**binary).unwrap();
    let endian = if object.is_little_endian() {
        Endian::Little
    } else {
        Endian::Big
    };

    let load_section = |id: gimli::SectionId| -> Result<Section, Box<dyn std::error::Error>> {
        Ok(match object.section_by_name(id.name()) {
            Some(section) => section.uncompressed_data()?,
            None => Section::Borrowed(&[]),
        })
    };

    let dwarf_sections = gimli::DwarfSections::load(&load_section).expect("loading sections??");

    (dwarf_sections, endian)
}

fn load_source(dwarf: Dwarf) { // this is a hell of a function, but gimli doesnt provide much better ways other than this, so ill leave comments

    // here we create the hashmaps
    let mut source_files = SourceMap::new();
    let mut line_addresses = LineAddresses::new();

    //this is just type annotations so i wouldnt have to write them out in the closure definition
    type LineProgram<'a> = gimli::IncompleteLineProgram<EndianSlice<'a, Endian>, usize>;
    type Unit<'a> = gimli::Unit<EndianSlice<'a, gimli::RunTimeEndian>, usize>;

    // this is extracted code for better readability into a closure. We take a single unit with its LineProgram, then we go row by row to link the RIP address and the lines, along with saving the files along the way
    let mut parse_line_program = |line_program:LineProgram, unit: Unit| {
        let comp_dir = PathBuf::from(unit.comp_dir.unwrap().to_string_lossy().into_owned());

        let mut rows = line_program.rows();
        while let Some((header, row)) = rows.next_row().unwrap() {
            if row.end_sequence() {
                continue;
            }

            let file = row.file(header).unwrap();
            let file_name = file.path_name().string_value(&dwarf.debug_str).unwrap().to_string_lossy().into_owned();

            let mut include_dir = {
                let include_dir = file.directory(header).unwrap();
                PathBuf::from(
                    include_dir.string_value(&dwarf.debug_str)
                    .unwrap().to_string_lossy().into_owned()
                )
            };

            let (rel_dir, hash_dir) = if include_dir.is_relative() {
                include_dir.push(file_name);
                (include_dir, comp_dir.clone())
            } else {
                match include_dir.strip_prefix(comp_dir.clone()) {
                    Ok(path) => {
                        let mut rel_dir = PathBuf::from(path);
                        rel_dir.push(file_name);
                        (comp_dir.clone(), rel_dir)
                    },
                    Err(_) => (include_dir, PathBuf::from(file_name)),
                }
            };

            let line = match row.line() {
                Some(line) => line.get(),
                None => continue
            };
            let address = row.address();

            let source_file = SourceFile::new(rel_dir, unit.debug_info_offset().unwrap());

            let source_index = source_files.insert_file(source_file, hash_dir, line);
            line_addresses.insert(address, source_index);
        }
    };

    //we iterate over ALL units (for now) and find the line program for all. Later on ill add filters to this as neededs
    let mut units = dwarf.units();
    while let Some(header) = units.next().unwrap() {
        let unit = dwarf.unit(header).unwrap();
        //let unit = unit.unit_ref(&dwarf);
        if let Some(line_program) = unit.line_program.clone() {
            parse_line_program(line_program, unit);
        }
    };

    // here we set/overwrite the Internal data
    let mut ibind = INTERNAL.access();
    ibind.source_files = Some(SourceMap::new());
    ibind.line_addresses = Some(LineAddresses::new());
}


/*

Okay, i spent WAY TOO LONG on this and i have to explain:

So, we have dwarf info. nice. but compilers tend to (AS THEY SHOULD) add information from ALL of the compiled files.
This would be fine, if it werent for the SHEER amount of data, specifically in Rusts Dwarf output and hiararchy.
The point is, i wanted to include only the files written by the user, as those are the only ones that usually need debugging.
But rust creates hashlike names for all of the files when linking the libraries, and you know what that means.
I have no way of telling, if the file is actually the input, or just some library.

APART FROM THE FACT I DO! HAHAHA!

nice so i CANNNN actually tell them apart. but now it comes down to that id have to make an exception in the code for rust and any other language that decides to play stupid.
So, yeaaa. Weird, but its not too bad. I can just filter them out looking at the second entry in the comp unit in .debug_info:
it needs to be declaring a namespace AND the name has to be Rust. then there might be still some extra, but those can be dealt with by choosing only a single compilation directory.

Otherwise, yea, its pretty easy, although i might have to have more compilation directories and/or filter ones like "lib" and etc.

good luck

*/