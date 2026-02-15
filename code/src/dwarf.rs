use std::path::PathBuf;
use std::collections::HashMap;
use std::borrow::Cow;


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
    fn insert_file(&mut self, source_file: SourceFile, hash_dir: &PathBuf, line_number: u64) -> SourceLine;
    fn index_with_line(&self, line: SourceLine) -> &SourceFile;
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

    fn insert_file(&mut self, source_file: SourceFile, hash_dir: &PathBuf, line_number: u64) -> SourceLine {
        if self.contains_key(hash_dir) {
            let v =self.get_mut(hash_dir).unwrap();
            for i in 0..v.len() {
                if v[i] == source_file {
                    return SourceLine::new(hash_dir.clone(), i, line_number);
                }
            }
            v.push(source_file);
            SourceLine::new(hash_dir.clone(), v.len()-1, line_number)
        } else {
            self.insert(hash_dir.clone(), vec![source_file]);
            SourceLine::new(hash_dir.clone(), 0, line_number)
        }
    }

    fn index_with_line(&self, line: SourceLine) -> &SourceFile {
        let vec = self.get(&line.hash_path).unwrap();
        &vec[line.index]
    }
}

#[derive(PartialEq)]
pub struct SourceLine {
    number: u64,
    hash_path: PathBuf,
    index: usize
}

impl SourceLine {
    fn new(hash_path: PathBuf, index: usize, line_number: u64) -> Self {
        SourceLine {
            number: line_number,
            hash_path,
            index
        }
    }
}

pub type LineAddresses = HashMap<u64, SourceLine>;

pub trait ImplLineAddresses<'a> {
    fn get_line(&'a self, address: u64) -> Option<&'a SourceLine>;
    fn get_source_file(&'a self, address: u64) -> Option<SourceFile>;
    fn get_address(&'a self, line: &SourceLine) -> Option<u64>;
}

impl <'a> ImplLineAddresses<'a> for LineAddresses {
    fn get_line(&'a self, address: u64) -> Option<&'a SourceLine> {
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

    fn get_address(&'a self, line: &SourceLine) -> Option<u64> {
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

fn load_dwarf(binary: &Vec<u8>) -> DwarfSections {
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

    dwarf_sections
}

fn load_source(dwarf: Dwarf) {
    let mut line_addresses = LineAddresses::new();
    let mut source_map = SourceMap::new();

    let mut units = dwarf.units();
    while let Some(header) = units.next().unwrap() {
        let unit = dwarf.unit(header).unwrap();
        let unit = unit.unit_ref(&dwarf);
        if let Some(line_program) = unit.line_program.clone() {
            let comp_dir = PathBuf::from(unit.comp_dir.unwrap().to_string_lossy().into_owned());

            let mut rows = line_program.rows();
            while let Some((header, row)) = rows.next_row().unwrap() {
                if row.end_sequence() {
                    continue;
                }

                let file = row.file(header).unwrap();
                let file_name = file.path_name().string_value(&dwarf.debug_str).unwrap().to_string_lossy().into_owned();

                let mut dir_path = {
                    let include_dir = file.directory(header).unwrap();
                    PathBuf::from(
                        include_dir.string_value(&dwarf.debug_str)
                        .unwrap().to_string_lossy().into_owned()
                    )
                };

                let line = match row.line() {
                    Some(line) => line.get(),
                    None => continue
                };




                let address = row.address();
                if file.directory_index() == 0 {
                    dir_path.clear();
                    dir_path.push(file_name);



                } else if dir_path.is_relative() {
                    dir_path.push(file_name);

                    let source_file = SourceFile::new(dir_path, unit.debug_info_offset().unwrap());

                    //let a = &mut source_map;
                    //a.insert_file(source_file, &comp_dir);
                    //line_addresses.insert(address, (line, source_map[&comp_dir].last().unwrap()));

                } else {
                    
                };
            }

        }
    }
}
