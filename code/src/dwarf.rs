use std::{
    path::PathBuf,
    collections::HashMap
};

use gimli::{
    DebugInfoOffset,
    UnitHeader,
    EndianSlice,
    UnwindSection
};

use object::{
    Object,
    ObjectSection
};

use iced_x86::{ // Disassembler
    Decoder,
    DecoderOptions,
    Formatter,
    Instruction,
    NasmFormatter
};


// internal imports
use crate::{
    data::*,
    trace
};


/// FILE: dwarf.rs - Loading, parsing and displaying the debbuging data from the DWARF standard, unwinding the STACK and function calls

const NOMASK: u64 = u64::MAX; // All bits set

#[derive(PartialEq, Clone, Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub compile_unit: DebugInfoOffset,
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
    fn get_file(&self, comp_dir: PathBuf, path: PathBuf) -> Option<(&SourceFile, usize)>;
    fn _get_comp_dir(&self, source_file: &SourceFile, dwarf: Dwarf) -> PathBuf;
    fn insert_file(&mut self, source_file: SourceFile, hash_dir: PathBuf, line_number: u64) -> SourceIndex;
    fn index_with_line(&self, line: &SourceIndex) -> &SourceFile;
    fn index_mut(&mut self, line: &SourceIndex) -> &mut SourceFile;
}

impl ImplSourceMap for SourceMap {
    fn get_file(&self, comp_dir: PathBuf, path: PathBuf) -> Option<(&SourceFile, usize)> {
        let mut n = 0;
        for file in self.get(&comp_dir).unwrap() {
            if file.path == path {
                return Some((file, n));
            }
            n+=1;
        };
        None
    }

    fn _get_comp_dir(&self, source_file: &SourceFile, dwarf: Dwarf) -> PathBuf {
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

    fn index_mut(&mut self, line: &SourceIndex) -> &mut SourceFile {
        let vec = self.get_mut(&line.hash_path).unwrap();
        vec.get_mut(line.index).unwrap()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct SourceIndex {
    pub line: u64, // in the SourceFile
    pub hash_path: PathBuf,
    pub index: usize // in the SourceMap Vec
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
    fn _get_source_file(&'a self, address: u64) -> Option<SourceFile>;
    fn get_address(&'a self, line: &SourceIndex) -> Option<u64>;
}

impl <'a> ImplLineAddresses<'a> for LineAddresses {
    fn get_line(&'a self, address: u64) -> Option<&'a SourceIndex> {
        match EXEC_SHIFT.access().as_ref() {
            Some(shift) => if *shift > address {return None},
            None => ()
        }
        self.get(&normal(address))
    }

    fn _get_source_file(&self, address: u64) -> Option<SourceFile> {
        match self.get(&normal(address)) {
            Some(line) => {
                let bind = SOURCE.access();
                let v= bind.as_ref().unwrap()[&line.hash_path].clone();
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

pub type Endian = gimli::RunTimeEndian;
type Section<'data> = std::borrow::Cow<'data, [u8]>;
pub type DwarfSections<'data> = gimli::DwarfSections<Section<'data>>;
pub type Dwarf<'a> = gimli::Dwarf<EndianSlice<'a, gimli::RunTimeEndian>>;

pub struct EhFrame<'a> {
    pub object: object::File<'a>,
    pub frame: std::borrow::Cow<'a, [u8]>,
    pub section_base: u64,
    pub main: Option<DebugInfoOffset>, // option for init
}

impl <'a>EhFrame<'a> {
    fn eh_frame(&'a self) -> gimli::EhFrame<EndianSlice<'a, Endian>> {
        gimli::EhFrame::new(&self.frame, ENDIAN.access().unwrap())
    }

    pub fn new(object: object::File<'static>) -> Self {
        let section = object.section_by_name(".eh_frame").unwrap();
        let base = section.address();
        let frame = section.uncompressed_data().unwrap();
        Self {
            object,
            frame,
            section_base: base,
            main: None
        }
    }
}

pub trait SectionsToDwarf {
    fn dwarf(&self, endian: Endian) -> Dwarf;
}

impl SectionsToDwarf for DwarfSections<'_> { // we cannot save the the borrow in the globals, so this wrapper saves space
    fn dwarf(&self, endian: Endian) -> Dwarf {
        self.borrow(|section| gimli::EndianSlice::new(Section::as_ref(section), endian))
    }
}

// Reading LINE_DATA

pub fn load_dwarf(binary: &'static Vec<u8>) -> Result<(DwarfSections<'static>, object::File<'static>), ()> {
    let object = object::File::parse(&**binary).unwrap();

    if object.section_by_name(".debug_info").is_none() {
        return Err(());
    };

    let load_section = |id: gimli::SectionId| -> Result<Section, Box<dyn std::error::Error>> {
        Ok(match object.section_by_name(id.name()) {
            Some(section) => section.uncompressed_data()?,
            None => Section::Borrowed(&[]),
        })
    };

    let dwarf_sections = gimli::DwarfSections::load(&load_section).map_err(|_| ())?;

    Ok((dwarf_sections, object))
}

type Unit<'a> = gimli::Unit<EndianSlice<'a, gimli::RunTimeEndian>, usize>;

pub fn load_source(dwarf: Dwarf) { // this is a hell of a function, but gimli doesnt provide much better ways other than this, so ill leave comments

    // here we create the hashmaps
    let mut source_files = SourceMap::new();
    let mut line_addresses = LineAddresses::new();

    //this is just type annotations so i wouldnt have to write them out in the closure definition
    type LineProgram<'a> = gimli::IncompleteLineProgram<EndianSlice<'a, Endian>, usize>;


    // this is extracted code for better readability into a closure. We take a single unit with its LineProgram, then we go row by row to link the RIP address and the lines, along with saving the files along the way
    let mut parse_line_program = |line_program:LineProgram, unit: Unit| {
        let comp_dir = PathBuf::from(unit.comp_dir.unwrap().to_string_lossy().into_owned());

        let mut rows = line_program.rows();
        while let Some((header, row)) = rows.next_row().unwrap() {
            if row.end_sequence() {
                continue;
            }

            // File name fetch
            let file = row.file(header).unwrap();
            let file_name = string(file.path_name(), &dwarf);

            // The directory in the units directory table
            let mut include_dir = {
                let include_dir = file.directory(header).unwrap();
                PathBuf::from(
                    string(include_dir, &dwarf)
                )
            };

            // if its relative, then we want to use the compilation directory as base
            let (rel_dir, hash_dir) = if include_dir.is_relative() {
                include_dir.push(file_name);
                (include_dir, comp_dir.clone())
            } else {
                match include_dir.strip_prefix(comp_dir.clone()) { // If it contains the compilation directory, we want to save it with the other files (easier for choosing files written by user, opposed to library includes)
                    Ok(path) => {
                        let mut rel_dir = PathBuf::from(path);
                        rel_dir.push(file_name);
                        (rel_dir, comp_dir.clone())
                    },
                    Err(_) => (PathBuf::from(file_name), include_dir), // If not, we just use the the include dir as the hash_path
                }
            };

            let line = match row.line() {
                Some(line) => line.get(),
                None => continue
            };
            let address = row.address();

            let source_file = SourceFile::new(rel_dir, unit.debug_info_offset().unwrap()); // we create the new SourceFile

            let source_index = source_files.insert_file(source_file, hash_dir, line); // insert it, getting back the index
            line_addresses.insert(address, source_index); // and finally we insert the SourceIndex (file index with the line info), to the address
        }
    };

    //we iterate over ALL units (for now) and find the line program for all.
    let mut units = dwarf.units();
    while let Some(header) = units.next().unwrap() {
        let unit = dwarf.unit(header).unwrap();
        if let Some(line_program) = unit.line_program.clone() {
            parse_line_program(line_program, unit);
        }
    };

    // Setting the Globals
    SOURCE.sets(source_files);
    LINES.sets(line_addresses);
}

pub fn get_main_file() -> (String, String) {
    let found = EHFRAME.access().as_ref().unwrap().main.is_some();
    let main_function = if found {
        EHFRAME.access().as_ref().unwrap().main.unwrap()
    } else {
        find_main()
    };

    let address = FUNCTIONS.access().as_ref().unwrap().get_address(main_function).unwrap();


    let lines_bind = LINES.access();
    let index = lines_bind.as_ref().unwrap().get_line(address).unwrap();

    let comp_dir = index.hash_path.clone();
    let source_bind = SOURCE.access();
    let file = source_bind.as_ref().unwrap().index_with_line(index).clone();
    let file_path = file.path;

    let mut path = comp_dir.clone();
    path.push(file_path.clone());

    (String::from(comp_dir.to_str().unwrap()), String::from(file_path.to_str().unwrap()))
}

// FUNCTION RANGES

type FunctionRange = std::ops::Range<u64>;

pub struct FunctionIndex<Unit = DebugInfoOffset, FunctionOffset = DebugInfoOffset> {
    pub func_hash: HashMap<u64, FunctionOffset>,
    pub range_hash: HashMap<Unit, Vec<FunctionRange>>,
    pub subtype_parent: HashMap<FunctionOffset, String> //kinda optional, but VERY useful // TODO, later, dont save name, but .debug_string offset !! (well see)
}

impl FunctionIndex {
    fn new() -> Self {
        FunctionIndex {
            func_hash: HashMap::new(),
            range_hash: HashMap::new(),
            subtype_parent: HashMap::new()
        }
    }

    fn insert_function(&mut self, range: FunctionRange, function_entry: DebugInfoOffset ,unit: DebugInfoOffset, subtype_parent: Option<&str>) {
        let range_hash = &mut self.range_hash;
        if range_hash.contains_key(&unit) {
            range_hash.get_mut(&unit).unwrap().push(range.clone());
        } else {
            range_hash.insert(unit, vec![range.clone()]);
        }
        let func_hash = &mut self.func_hash;
        func_hash.insert(range.start, function_entry);
        match subtype_parent {
            Some(str) => {
                let parent_hash = &mut self.subtype_parent;
                parent_hash.insert(function_entry, str.to_string());
            },
            None => ()
        };
    }

    fn direct_address(&self, address: u64) -> DebugInfoOffset {
        self.func_hash[&address]
    }

    pub fn get_function(&self, address: u64, unit: DebugInfoOffset) -> Option<DebugInfoOffset> {
        let range = match self.find_range(address, unit) {
            Some(range) => range,
            None => return None
        };

        Some(self.direct_address(range.start))
    }

    fn find_range(&self, address: u64, unit: DebugInfoOffset) -> Option<&FunctionRange> {
        let ranges = &self.range_hash[&unit];
        for range in ranges {
            if range.contains(&address) {
                return Some(range);
            }
        };
        None
    }

    fn get_address(&self, function: DebugInfoOffset) -> Option<u64> {
        for (address, info_offset) in self.func_hash.clone().into_iter() {
            if info_offset == function {
                return Some(address);
            }
        };
        None
    }
}

pub fn parse_functions(dwarf: Dwarf) {
    let mut function_index = FunctionIndex::new();

    let mut declarations: HashMap<DebugInfoOffset, &str>  = HashMap::new(); // mapping declaration function offset to its parent for later use, and they sit outside of units, in case some would be defined in different CU, although that is unlikely, also the reason for DebugInfoOffset instead

    let mut unit_headers = dwarf.units();


    while let Some(unit_header) = unit_headers.next().unwrap() {
        let unit = dwarf.unit(unit_header).unwrap();
        let base_address = unit.low_pc; // for ranges attr
        let mut entries = unit.entries();

        let mut parent_stack: Vec<&str> = Vec::new(); // i could technically always apppend the entire tree (like file::namespace::type ...), but i think just the last parent will be enough
        entries.next_entry().unwrap();

        loop {
            let prev_entry = entries.current(); // we need to do this here for coherent 'continue' logic in this loop

            match prev_entry {
                Some(entry) => {
                    if entries.next_depth() > entries.depth() { // even if this is a null entry, it will work, as the depth can never be higher after the null (which literally decreases the depth)
                        parent_stack.push( // pushing the parent up here (doesnt have to be a subprogram)
                            match entry.attr_value(gimli::DW_AT_name) {
                                Some(val) => string(val, &dwarf),
                                None => ""
                            }
                        );
                    };
                },
                None => ()
            };
            match entries.next_entry() {
                Ok(some) => if !some {
                    break;
                },
                Err(_) => break
            };
            let entry = match entries.current() {
                Some(entry) => entry,
                None => {parent_stack.pop(); continue}
            };
            if entry.tag() != gimli::DW_TAG_subprogram {continue;} // skipping over other entries

            if entry.has_attr(gimli::DW_AT_main_subprogram) { // Finding main subprogram
                let main = entry.offset().to_debug_info_offset(&unit_header);
                EHFRAME.access().as_mut().unwrap().main = main;
            };

            if entry.attr(gimli::DW_AT_declaration).is_some() { // save and skip declarations (no pc for us and such), they will be handled later
                declarations.insert(entry.offset.to_debug_info_offset(&unit).unwrap(), parent_stack.last().unwrap_or(&""));
                continue;
            };

            let parent: &str = match entry.attr(gimli::DW_AT_specification) { // if we have a specification, we want to get the parent of the declaration
                Some(function_declaration) => *declarations.get(&debug_reference(function_declaration.value(), &unit)).unwrap(),
                None => *parent_stack.last().unwrap_or(&"")
            };

            let applicable_parent = if parent == "" {None} else {Some(parent)}; // apart from the coherency

            // getting the PC ranges
            if let Some(value) = entry.attr_value(gimli::DW_AT_ranges) { // if it uses ranges (eg. functions that have inner functions), then we push all of them
                let range_offset = value.offset_value().unwrap();
                let mut ranges = dwarf.ranges.ranges(
                    gimli::RangeListsOffset(range_offset),
                    unit.encoding(),
                    base_address,
                    &dwarf.debug_addr,
                    unit.addr_base
                ).expect("Range Parsing Error");

                while let Some(range) = ranges.next().unwrap_or(None) { //iterating through the ranges
                    function_index.insert_function(
                        range.begin..range.end,
                        entry.offset.to_debug_info_offset(&unit).unwrap(),
                        unit.debug_info_offset().unwrap(),
                        applicable_parent
                    );
                }

                continue;
            }

            // Otherwise, it must contain low_pc and high_pc (if not we skip the function)
            let low_pc = match entry.attr_value(gimli::DW_AT_low_pc) {
                Some(value) => number(value),
                None => continue // if we dont have the ranges or the pc definitions, then saving the function only breaks the code
            };
            let high_pc = match entry.attr_value(gimli::DW_AT_high_pc) {
                Some(value) => number(value),
                None => continue
            };

            let function_range = low_pc..low_pc+high_pc;

            function_index.insert_function(
                function_range,
                entry.offset.to_debug_info_offset(&unit).unwrap(),
                unit.debug_info_offset().unwrap(),
                applicable_parent
            );
        }
    }

    // Setting the Global
    FUNCTIONS.sets(function_index);
}

pub fn find_main() -> DebugInfoOffset { // from the symbol, but only if main isnt already found (from parsing the functions)
    let mut ehframe_bind = EHFRAME.access();
    let ehframe = ehframe_bind.as_mut().unwrap();
    let symbol = ehframe.object.symbol_by_name("main").unwrap();
    let address = object::ObjectSymbol::address(&symbol);
    let function_bind = &FUNCTIONS.access();
    let functions = function_bind.as_ref().unwrap();
    let function = functions.direct_address(address);
    ehframe.main = Some(function);
    function
}


// Struct for returning the context and the row in the FDE
struct UnwindInfo {
    row: Option<gimli::UnwindTableRow<usize>>,
    ctx: gimli::UnwindContext<usize>
}

type GimliEhFrame<'a> = gimli::EhFrame<EndianSlice<'a, Endian>>;

fn get_unwind_for_address(address: u64, eh_frame: (&GimliEhFrame, &EhFrame) ) -> UnwindInfo {
    // getting the section base (gimli internal logic)
    let bases = gimli::BaseAddresses::default().set_eh_frame(eh_frame.1.section_base);
    // fetching the FDE
    let fde = eh_frame.0.fde_for_address(&bases, address, gimli::EhFrame::cie_from_offset).unwrap();

    let mut res = UnwindInfo {
        row: None,
        ctx: gimli::UnwindContext::new()
    };
    // we get the unwind info for the current address
    let unwind_info = fde.unwind_info_for_address(eh_frame.0, &bases, &mut res.ctx, address).unwrap();

    res.row = Some(unwind_info.clone());
    res
}


fn get_cfa(unwind: &UnwindInfo, regs: &mut nix::libc::user_regs_struct, eh_frame: &GimliEhFrame, encoding: gimli::Encoding) -> Result<u64, ()> {
    let cfa = unwind.row.as_ref().unwrap().cfa();
    match cfa {
        gimli::CfaRule::RegisterAndOffset {
            register,
            offset
        } => Ok((*match_register(register, regs) as i64 + offset) as u64), // we get the register and add the offset (casting to avoid overflows)
        gimli::CfaRule::Expression(expression) => {
            let expression = expression.get(eh_frame).unwrap();
            let piece = eval_expression(&expression, regs, None, None, encoding)?[0];
            match piece.location {
                gimli::Location::Value {value} => Ok(value.to_u64(NOMASK).unwrap()),
                _ => panic!("CFA parsing error") // if it would be a register, it would use the upper branch of matchs
            }
        },
    }
}

fn unwind_registers(unwind: &UnwindInfo, cfa: u64, regs: &mut nix::libc::user_regs_struct, eh_frame: &GimliEhFrame, encoding: gimli::Encoding) -> Result<(), ()> {
    // register unwind rules
    let rules = unwind.row.as_ref().unwrap().registers();
    for (reg, rule) in rules { // we iterate through the rules
        println!("{:?}", reg);
        let value = unwind_register(rule, cfa, regs, eh_frame, encoding);
        match value {
            Ok(value) => *match_register(reg, regs) = value,
            Err(_) => ()
        };
    };

    Ok(())
}

fn unwind_register(register_rule: &gimli::RegisterRule<usize>, cfa: u64, regs: &mut nix::libc::user_regs_struct, eh_frame: &GimliEhFrame, encoding: gimli::Encoding) -> Result<u64, ()> {
    println!("{:?}", register_rule);
    match register_rule {
        gimli::RegisterRule::Offset(addr_offset) => Ok(slice_to_u64(&unwind_memory((cfa as i64 + addr_offset) as u64, 8)?)),
        gimli::RegisterRule::Expression(expression) => {
            let expression = expression.get(eh_frame).unwrap();
            let piece = eval_expression(&expression, regs, Some(cfa), None, encoding)?[0];
            Ok(slice_to_u64(&unwind_memory(
                match piece.location {
                    gimli::Location::Value {value} => value.to_u64(NOMASK).unwrap(),
                    _ => panic!("Register Expression Error")
                }
            , 8)?)
        )},
        gimli::RegisterRule::ValOffset(offset) => Ok((cfa as i64 + offset) as u64),
        gimli::RegisterRule::ValExpression(expression) => {
            let expression = expression.get(eh_frame).unwrap();
            let piece = eval_expression(&expression, regs, Some(cfa), None, encoding)?[0];
            Ok(match piece.location {
                gimli::Location::Value {value} => value.to_u64(NOMASK).unwrap(),
                _ => panic!("Register Expression Error")
            })
        },
        gimli::RegisterRule::Register(register) => Ok(*match_register(register, regs)),
        gimli::RegisterRule::Constant(value) => Ok(*value),
        gimli::RegisterRule::SameValue => Err(()),
        _ => unimplemented!("Unimplemented register unwind rule")
    }
}

fn match_register<'a>(register: &gimli::Register, regs: &'a mut nix::libc::user_regs_struct) -> &'a mut u64 { //we match the gimli registers to the user regs
    match *register {
        gimli::X86_64::RAX => &mut regs.rax,
        gimli::X86_64::RBX => &mut regs.rbx,
        gimli::X86_64::RCX => &mut regs.rcx,
        gimli::X86_64::RDX => &mut regs.rdx,
        gimli::X86_64::RSI => &mut regs.rsi,
        gimli::X86_64::RDI => &mut regs.rdi,
        gimli::X86_64::RSP => &mut regs.rsp,
        gimli::X86_64::RBP => &mut regs.rbp,
        gimli::X86_64::R8 =>  &mut regs.r8,
        gimli::X86_64::R9 =>  &mut regs.r9,
        gimli::X86_64::R10 => &mut regs.r10,
        gimli::X86_64::R11 => &mut regs.r11,
        gimli::X86_64::R12 => &mut regs.r12,
        gimli::X86_64::R13 => &mut regs.r13,
        gimli::X86_64::R14 => &mut regs.r14,
        gimli::X86_64::R15 => &mut regs.r15,
        gimli::X86_64::RA =>  &mut regs.rip,
        _ => unimplemented!("Unimplemented Register Match")
    }
}

fn unwind_memory(address: u64, size: u8) -> Result<Vec<u8>, ()> { // for reading small amounts of data from the memory (a wrapper)
    trace::read_memory(address, size as usize)
}

fn eval_expression<'a>(
    expression: &'a gimli::Expression<EndianSlice<'_, Endian>>,
    regs: &mut nix::libc::user_regs_struct,
    cfa: Option<u64>,
    frame_base: Option<u64>,
    encoding: gimli::Encoding
) -> Result<Vec<gimli::Piece<EndianSlice<'a, gimli::RunTimeEndian>, usize>>, ()> {
    let mut evaluation = expression.evaluation(encoding);

    let mut result = evaluation.evaluate().map_err(|_| ())?;
    loop {
        println!("{:?}", result);
        match result {
            gimli::EvaluationResult::Complete => break,
            gimli::EvaluationResult::RequiresMemory { address, size, .. } => {
                let data = unwind_memory(address, size)?;
                result = evaluation.resume_with_memory(gimli::Value::U64(slice_to_u64(&data))).map_err(|_| ())?;
            },
            gimli::EvaluationResult::RequiresRegister { register, .. } => {
                result = evaluation.resume_with_register(gimli::Value::U64(*match_register(&register, regs))).map_err(|_| ())?;
            },
            gimli::EvaluationResult::RequiresFrameBase => {
                result = evaluation.resume_with_frame_base(frame_base.ok_or(())?).map_err(|_| ())?;
            },
            gimli::EvaluationResult::RequiresCallFrameCfa => {
                result = evaluation.resume_with_call_frame_cfa(cfa.ok_or(())?).map_err(|_| ())?;
            }
            gimli::EvaluationResult::RequiresRelocatedAddress(address) => {
                let new = anti_normal(address);
                result = evaluation.resume_with_relocated_address(new).map_err(|_| ())?;
            }
            _ => {println!("Unimplemented Evaluation Expression"); return Err(())}
        }
    }
    Ok(evaluation.result())
}

fn slice_to_u64(slice: &[u8]) -> u64 {
    let bytes: [u8; 8] = slice.try_into().unwrap(); // I know this could panic, but we shouldnt be fetching more than 8 bytes
    let endian: Endian = ENDIAN.access().unwrap();


    match endian {
        Endian::Big => u64::from_be_bytes(bytes),
        Endian::Little => u64::from_le_bytes(bytes)
    }
}


// CALL STACK PARSING

#[derive(Debug, Clone)]
pub struct CallStack (pub Vec<Function>);

impl CallStack {
    fn new() -> Self {
        CallStack(Vec::new())
    }

    pub fn stack_lines(stack: Result<Self, ()>) -> Result<Vec<(usize, String)>, ()> {
        if stack.is_err() {
            return Err(());
        }

        let functions_bind = FUNCTIONS.access();
        let dwarf_bind = DWARF.access();
        let dwarf = dwarf_bind.as_ref().unwrap().dwarf(ENDIAN.access().unwrap());

        let mut stack = stack.unwrap().0;
        stack.reverse();

        let mut result = Vec::new();

        for (call, function) in stack.iter().enumerate() {
            function.lines(call, &mut result, &dwarf, functions_bind.as_ref().unwrap());
        };
        Ok(result)
    }
}

type Type = DebugInfoOffset;
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Option<Vec<Parameter>>,
    pub variables: Option<Vec<Variable>>,
    pub return_type: Option<Type>,
    pub debug_info_offset: Option<DebugInfoOffset>,
}

impl Function {
    pub fn lines(&self, call: usize, res: &mut Vec<(usize, String)>, dwarf: &Dwarf, functions: &FunctionIndex) {
        let parent = match functions.subtype_parent.get(&self.debug_info_offset.unwrap()) {
            Some(parent) => format!("{parent}::"),
            None => "".to_string()
        };
        let return_type = match self.return_type {
            Some(vtype) => format!(" -> {}", unwind_type(vtype, &dwarf).name(&dwarf)),
            None => "".to_string()
        };

        if let Some(parameters) = &self.parameters {
            res.push((0, format!("{call}: {}{}(", parent, self.name)));
            for param in parameters {
                let mut temp_buf = Vec::new();
                let param_value = param.lines(&mut temp_buf, &dwarf);

                res.push((2, param_value));
                res.append(&mut temp_buf);
                res.last_mut().unwrap().1.push(',');
            }
            res.last_mut().unwrap().1.pop();
            res.push((1, format!("){}", return_type,))); // the depth 1 is rendered as depth 0, but gets hidden when the call is collapsed
        } else {
            res.push((0, format!("{call}: {}{}(){}", parent, self.name, return_type,)));
        };

        if let Some(variables) = &self.variables {
            for var in variables {
            let mut temp_buf = Vec::new();
            let var_value = var.lines(&mut temp_buf, &dwarf);

            res.push((2, var_value));
            res.append(&mut temp_buf);
            res.last_mut().unwrap().1.push(';');
        }}
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub location: Option<Location>,
    pub constant: Option<u64>,
    pub vtype: Type
}

impl Variable {
    pub fn lines(&self, res: &mut Vec<(usize, String)>, dwarf: & Dwarf) -> String {
        let vtype = unwind_type(self.vtype, dwarf);

        if let Some(location) = self.location.clone() {
            let value = vtype.value(location, res, 2, dwarf);
            return format!("{} {} = {}", vtype.name(dwarf), self.name, value);
        };
        if let Some(constant) = self.constant {
            let value = vtype.const_value(constant);
            return format!("{} {} = {}", vtype.name(dwarf), self.name, value);
        };

        format!("{} {}", vtype.name(dwarf), self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub location: Location,
    pub vtype: Type
}

impl Parameter {
    pub fn lines(&self, res: &mut Vec<(usize, String)>, dwarf: &Dwarf) -> String {
        let vtype = unwind_type(self.vtype, dwarf);
        let value = vtype.value(self.location.clone(), res, 2, dwarf);
        format!("{} {} = {}", vtype.name(dwarf), self.name, value)
    }
}

pub fn call_stack<'a>() -> Result<CallStack, ()> {
    let mut call_stack = CallStack::new();

    println!("call");

    let mut registers = REGISTERS.access().unwrap();

    let ehframe = EHFRAME.access();
    let lines = LINES.access();
    let source = SOURCE.access();
    let functions = FUNCTIONS.access();
    let dwarf = DWARF.access();

    let bindings = (
        ehframe.as_ref().unwrap(),
        lines.as_ref().unwrap(),
        source.as_ref().unwrap(),
        functions.as_ref().unwrap(),
        dwarf.as_ref().unwrap(),
    );

    loop {
        print!(".");
        if unwind(&mut call_stack, bindings, &mut registers)? {
            break;
        };
    }

    println!("ok");
    Ok(call_stack)
}

type Bindings<'a> = (
    &'a EhFrame<'a>,
    &'a LineAddresses,
    &'a SourceMap,
    &'a FunctionIndex,
    &'a DwarfSections<'a>,
);

fn unwind (
    call_stack: &mut CallStack,
    bindings: Bindings,
    regs: &mut nix::libc::user_regs_struct
) -> Result<bool, ()> {

    let eh_frame = bindings.0;
    let gimli_eh_frame = eh_frame.eh_frame();
    let line_addresses = bindings.1;
    let source_map = bindings.2;
    let function_index = bindings.3;
    let dwarf = bindings.4.dwarf(ENDIAN.access().unwrap());


    // We need info about the function and all
    let (index, _rip)= get_next_line(regs.rip, line_addresses)?; // if we arent in a source file, we cannot be debugging the info (can happen using step while stepping into a dynamic library function, and therefore it will not show any of the call stack info, as we have no Dwarf info)
    //regs.rip = rip;
    println!("{:?}", index);
    print!("a");
    let unit = source_map.get(&index.hash_path).unwrap()[index.index].compile_unit; // our function index is unit organised for faster lookup speed. this is possible thanks to the hashmap, which has constant access time, while searching through ranges is linear access time (in the end, this is MUCH faster, especially with more functions and source files)
    print!("b");
    let function = function_index.get_function(normal(regs.rip), unit).unwrap(); // now we get our function offset into the debug_info_section
    print!("c");
    let unit_header = dwarf.debug_info.header_from_offset(unit).unwrap();
    print!("d");
    let dwarf_unit = dwarf.unit(unit_header).unwrap();

    print!("e");
    let offset = function.to_unit_offset(&dwarf_unit).unwrap();
    print!("f");
    let mut entries = dwarf_unit.entries_at_offset(offset).unwrap();
    print!("g");
    let _ = entries.next_entry();
    print!("h");
    let die = entries.current().unwrap();
    print!("i");
    let (mut function_info, frame_attribute) = extract_function_info(die, &dwarf, &dwarf_unit);

    print!("j");
    function_info.debug_info_offset = Some(function);

    print!("k");
    let unwind_info = get_unwind_for_address(normal(regs.rip), (&gimli_eh_frame, eh_frame));
    print!("l");
    let encoding = dwarf_unit.encoding();
    print!("m");
    let cfa = get_cfa(&unwind_info, regs, &gimli_eh_frame, encoding)?;



    print!("o");
    let frame_base = if frame_attribute.is_some() {
        let expression = frame_attribute.unwrap().exprloc_value().unwrap();
        let frame_base = eval_expression(&expression, regs, Some(cfa), None, encoding)?[0];
        match frame_base.location {
            gimli::Location::Address { address } => Some(address),
            gimli::Location::Register {register} => Some(*match_register(&register, regs)),
            _ => panic!("Unknown FrameBase Expression")
        }
    } else {
        None
    };

    if CONFIG.access().as_ref().unwrap().feature.as_ref().unwrap().exp_rust_unwind.unwrap() { // FEATURE SETTING
        regs.rsp = cfa;
    }
    // This requires explanation:

    /*
    Because Rust creates only some Dwarf data and is not completely supported, it does things quite differently compared to other languages.
    For example most languages use the FrameBase as the CFA, and one could consider it the standard as every function creates a stack frame if it changes the rsp at any point.
    Now Rust on the other hand uses RSP as the FrameBase. This inherently disconnects it from the evaluation process of the CFA. But that should be fine right?

    Well... sort of. The CFA always has a rule as you always need a way to recover your return address (you could ofcourse just use rsp for that but uh oh, what if you have no rsp etc..)
    So first you unwind the CFA, and then you can proceed with all of the registers. But DWARF (and gimli) doesnt specify the unwindinfo if it uses the DEFAULT rule.
    And the default rule is REALLY HARD to get to.

    Now combining that with rust: our RSP stays the same during unwinds. BIG PROBLEM. the effect is simple: Variable values from previous calls wont be correct.
    Its not that big of an issue but to me, incredibly irritating. So i spent around 2 hours staring at the Output and comparing values.

    Apparently the rsp should be equal to the previous CFA. But not always, and what about the other languages? How will that work.
    Well i can just leave the regs to get unwound after, therefore if there IS a rule to unwind the RSP, it will use that.

    But its is INCREDIBLY unsafe, so much i want to create the only single feature flag for this project.
    either that, or i want to put a setting into the config.

    The problem with just overwriting a saved register is when a different register is bound to its value. In that case we would have to first check whether it actually is scheduled to be unwound.
    THEN and ONLY THEN we could change it AFTER all of the other register were unwound. but yea a long and tedious process

    TLDR: might not work, be careful
    */

    println!("{}\n {:?} {}",cfa, frame_base, regs.rsp);

    print!("p");
    //extract_variables(&mut function_info, regs, frame_base, entries, &dwarf, encoding, index.line, &dwarf_unit)?;
    extract_var(&mut function_info, regs, frame_base, entries, &dwarf, encoding, index.line, &dwarf_unit)?;

    print!("n");
    unwind_registers(&unwind_info, cfa, regs, &gimli_eh_frame, encoding)?;


    print!("q");
    if function_info.variables.as_ref().unwrap().len() == 0 { //space opt
        function_info.variables = None
    };

    print!("r");
    if function_info.parameters.as_ref().unwrap().len() == 0 { //space opt
        function_info.parameters = None
    };

    print!("s");
    let main_function = check_for_main(&function_info, eh_frame);

    print!("t");
    call_stack.0.push(function_info);

    print!("u");
    if main_function {return Ok(true);}

    Ok(false)
}

fn extract_function_info<'a>(
    entry: &gimli::DebuggingInformationEntry<EndianSlice<'a, Endian>, usize>,
    dwarf: &'a Dwarf, unit: &Unit
) -> (Function, Option<gimli::AttributeValue<EndianSlice<'a, Endian>>>) {

    let (name, return_type) = match entry.attr_value(gimli::DW_AT_specification) {
        Some(specification) => {
            let declaration_offset = get_unit_entry_offset(debug_reference(specification, unit), dwarf);
            let declaration_entry = dwarf.unit(dwarf.unit_header(declaration_offset.1).unwrap()).unwrap().entry(declaration_offset.0).unwrap();
            let name = string(declaration_entry.attr_value(gimli::DW_AT_name).unwrap(), dwarf);
            let return_type = match declaration_entry.attr_value(gimli::DW_AT_type) {
                    Some(attr) => Some(debug_reference(attr, unit)),
                    None => None
            };
            (name, return_type)
        },
        None => (
            string(entry.attr_value(gimli::DW_AT_name).unwrap(), dwarf),
            match entry.attr_value(gimli::DW_AT_type) {
                Some(attr) => Some(debug_reference(attr, unit)),
                None => None
            }
        )
    };

    let frame_base = match entry.attr_value(gimli::DW_AT_frame_base) {
        Some(attr) => Some(attr),
        None => None
    };

    (Function {
        name: String::from(name),
        parameters: Some(Vec::new()),
        variables: Some(Vec::new()),
        return_type: return_type,
        debug_info_offset: None
    }, frame_base)
}

#[derive(Debug, Clone)]
pub enum Location {
    Register(gimli::Register),
    Address(u64)
}

fn extract_var<'a>(
    function: &mut Function,
    regs: &mut nix::libc::user_regs_struct,
    frame_base: Option<u64>,
    mut entries: gimli::EntriesCursor<'_, EndianSlice<'a, Endian>>,
    dwarf: &'a Dwarf,
    encoding: gimli::Encoding,
    current_line: u64,
    unit: &Unit
) -> Result<(), ()> {
    let fn_depth = entries.depth(); // saving the original depth
    let mut first = true;
    entries.next_entry().unwrap(); //move from the Subprogram Entry

    let variables = function.variables.as_mut().unwrap(); // References to the function struct
    let parameters = function.parameters.as_mut().unwrap();

    loop {
        println!("{:?}", variables);
        println!("{:?}", parameters);
        if !first { // Would skip over the first entry otherwise and we dont want that
            match entries.current() {
                Some(entry) => match entry.tag() {
                    gimli::DW_TAG_subprogram | gimli::DW_TAG_inlined_subroutine => {entries.next_sibling().map_err(|_| ())?;} // skipping subfunctions and inlines (those have their own variables and parameters)
                    _ => {entries.next_entry().map_err(|_| ())?;}
                },
                None => {entries.next_entry().map_err(|_| ())?;}
            }
        } else {
            first = false;
        }
        println!("{}", entries.depth());
        if fn_depth == entries.depth() { // if the new entry has the same depth as the original fn_depth, then they are siblings and therefore we ran into the end of the function locals definition // we cant just use the null entry, because the some variables can be in deeper lexical fields, so this is the easiest
            return Ok(());
        }

        let entry = match entries.current() { // skip null entries
            Some(entry) => entry,
            None => continue
        };

        if (entry.tag() != gimli::DW_TAG_variable) && (entry.tag() != gimli::DW_TAG_formal_parameter) {
            continue;
        }

        let name = String::from(
            match entry.attr_value(gimli::DW_AT_name) {
                Some(attr) => string(attr, dwarf), //URL
                None => "0"
            }
        );
        let vtype = debug_reference(entry.attr_value(gimli::DW_AT_type).unwrap(), unit);

        let location = match entry.attr_value(gimli::DW_AT_location) { // we find the location (either from evaluating the expression, or by looking up the location list)
            Some(attr) => {
                if attr.exprloc_value().is_some() {
                    let expression = attr.exprloc_value().unwrap();
                    let piece = eval_expression(&expression, regs, None, frame_base, encoding)?[0];
                    match piece.location {
                        gimli::Location::Register {register} => Some(Location::Register(register)),
                        gimli::Location::Value {value} => Some(Location::Address(value.to_u64(NOMASK).unwrap())),
                        gimli::Location::Address {address} => Some(Location::Address(address)),
                        _ => panic!()
                    }
                } else {
                    if let Some(loclist) = dwarf.attr_locations(unit, attr).map_err(|_| ())? {
                        get_loclist_location(loclist, regs, frame_base, encoding).map_or(None, |loc| Some(loc))
                    } else {
                        None
                    }
                }
            },
            None => None
        };
        let constant = match entry.attr(gimli::DW_AT_const_value) {
            Some(attr) => match attr.udata_value() { // constant values only work for base_types right now
                Some(val) => Some(val),
                None => {println!("WTF CONSTANT"); None}
            },
            None => None
        };

        if entry.tag() == gimli::DW_TAG_variable {
            let declaration = number(entry.attr_value(gimli::DW_AT_decl_line).unwrap());

            if declaration >= current_line {continue;}; // if you have a variable you havent declared yet, you dont want to show it right, cause its gonna be random gibberish yk

            let var = Variable {
                name,
                location,
                constant,
                vtype
            };
            variables.push(var);
        } else {
            let param = Parameter {
                name,
                location: location.unwrap(),
                vtype
            };
            parameters.push(param);
        }
    }
}

fn get_loclist_location(
    mut loc_list: gimli::LocListIter<EndianSlice<'_, Endian>>,
    regs: &mut nix::libc::user_regs_struct,
    frame_base: Option<u64>,
    encoding: gimli::Encoding
) -> Result<Location, ()> {
    while let Some(entry) = loc_list.next().map_err(|_| ())? {
        if (entry.range.begin..entry.range.end).contains(&normal(regs.rip)) {
            let expression = entry.data;
            let piece = eval_expression(&expression, regs, None, frame_base, encoding)?[0];
            match piece.location {
                gimli::Location::Value {value} => return Ok(Location::Address(value.to_u64(NOMASK).unwrap()+frame_base.unwrap())),
                gimli::Location::Address {address} => return Ok(Location::Address(address)),
                gimli::Location::Register {register} => return Ok(Location::Register(register)),
                _ => panic!()
            };
        }
    };
    Err(())
}

fn check_for_main(info: &Function, eh_frame: &EhFrame) -> bool { // function because of language specifications, for now just name matching
    info.debug_info_offset == eh_frame.main
}

fn get_next_line(mut rip: u64, lines: &LineAddresses) -> Result<(&SourceIndex, u64), ()> { // calls are unfortunately outside of the lines addresses, so im using backwards byte search to find the correct address, thankfully this is an O(n) operation, where n shouldnt get larger than a 1000, which is really fast for me
    for _ in 0..100000 {
        println!("{}", rip);
        match lines.get_line(rip) {
            Some(index) => return Ok((index, rip)), // We need to return the updated rip, otherwise the framebase and such will be incorrect
            None => ()
        };
        rip = rip.checked_sub(1).map_or(Err(()), |x | Ok(x))?;
    };
    Err(())
}

// Type unwinding - especially for displaying the types
enum TypeDisplay<'a> {
    Base(BaseType<'a>),
    Pointer(PointerType<'a>),
    Modifier(ModifierType<'a>),
    Array(ArrayType<'a>),
    Struct(StructType<'a>),
    Enum(EnumType<'a>),
    Def(TypeDef<'a>),
}

impl <'a> TypeDisplay<'a> {
    fn name(&self, dwarf: &'a Dwarf) -> String {
        match self {
           Self::Base(base) => base.name.unwrap_or("").to_string(),
           Self::Pointer(pointer) => pointer.name(dwarf),
           Self::Modifier(modifier) => format!("{} {}", modifier.name(), unwind_type(modifier.vtype, dwarf).name(dwarf)), // will recurse name
           Self::Array(array) => array.name(dwarf),
           Self::Struct(str) => str.name.unwrap_or("").to_string(),
           Self::Enum(enume) => enume.name.to_string(),
           Self::Def(typedef) => typedef.name.to_string(),
        }
    }

    fn value(&self, location: Location, res: &mut Vec<(usize, String)>, depth: usize, dwarf: &Dwarf) -> String {
        match self {
            Self::Base(base) => base.value(location),
            Self::Pointer(pointer) => pointer.value(location),
            Self::Modifier(modifier) => modifier.value(location, res, depth, dwarf),
            Self::Array(array) => array.value(location, res, depth, dwarf),
            Self::Struct(str) => str.value(location, res, depth, dwarf),
            Self::Enum(enume) => enume.value(location, dwarf),
            Self::Def(typedef) => typedef.value(location, res, depth, dwarf)
        }
    }

    fn const_value(&self, constant: u64) -> String {
        match self {
            Self::Base(base) => {
                let slice = constant.to_le_bytes();
                base.constant_value(&slice, gimli::RunTimeEndian::Little) // because to_LE_bytes()
            },
            Self::Pointer(pointer) => {
                pointer.display(constant)
            }
            _ => "?".to_string()
        }
    }

    fn size(&self, dwarf: &Dwarf) -> BitByteSize {
        match self {
            Self::Base(base) => base.size,
            Self::Pointer(pointer) => {
                unwind_type(pointer.vtype, dwarf).size(dwarf)
            },
            Self::Modifier(modifier) => {
                unwind_type(modifier.vtype, dwarf).size(dwarf)
            }
            Self::Array(array) => match array.size {
                Some(size) => size,
                None => {
                    unwind_type(array.vtype, dwarf).size(dwarf)
                }
            },
            Self::Struct(str) => str.size,
            Self::Enum(enume) => {
                match enume.size {
                    Some(size) => size,
                    None => {
                        unwind_type(enume.vtype.unwrap(), dwarf).size(dwarf)
                    }
                }
            }
            Self::Def(typedef) => {
                unwind_type(typedef.vtype.unwrap(), dwarf).size(dwarf)
            }
        }
    }
}

pub struct BaseType<'a> {
    pub name: Option<&'a str>,
    pub encoding: gimli::DwAte,
    pub size: BitByteSize,
    pub endian: Option<Endian>
}

impl <'a>BaseType<'a> {
    fn constant_value(&self, slice: &[u8], endian: Endian) -> String {
        self.encoding(slice, endian)
    }

    fn value(&self, location: Location) -> String {
        let endian = self.endian.unwrap_or(*ENDIAN.access().as_ref().unwrap());
        if let Ok(slice) = location_memory(location, self.size, endian) {
            self.encoding(&slice, endian)
        } else {
            "?".to_string()
        }
    }

    fn encoding(&self, slice: &[u8], endian: Endian) -> String {
        let byte_size = slice.len();
        match self.encoding {
            gimli::DW_ATE_unsigned => {
                match byte_size {
                    1    => slice[0].to_string(),
                    2    => u16::from_bytes(slice, endian).to_string(),
                    3..4 => u32::from_bytes(slice, endian).to_string(),
                    4..8 => u64::from_bytes(slice, endian).to_string(),
                    _    => u128::from_bytes(slice, endian).to_string()
                }
            },
            gimli::DW_ATE_unsigned_char => {
                if slice[0].is_ascii_graphic() {
                    format!("'{}'", slice[0] as char)
                } else {
                    format!("{:2x}h", slice[0])
                }
            }
            gimli::DW_ATE_signed => {
                match byte_size {
                    1    => (slice[0] as i32).to_string(),
                    2    => i16::from_bytes(slice, endian).to_string(),
                    3..4 => i32::from_bytes(slice, endian).to_string(),
                    4..8 => i64::from_bytes(slice, endian).to_string(),
                    _    => i128::from_bytes(slice, endian).to_string()
                }
            },
            gimli::DW_ATE_signed_char => {
                format!("{}", slice[0] as i8)
            }
            gimli::DW_ATE_ASCII => {
                if slice[0].is_ascii_graphic() {
                    format!("'{}'", slice[0] as char)
                } else {
                    format!("{:2x}h", slice[0])
                }
            }
            gimli::DW_ATE_boolean => {
                if slice[0] != 0 {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            },
            gimli::DW_ATE_UTF => {
                let mut buf = String::new();
                slice.iter().for_each(|byte| buf.push_str(&format!("{:X}", byte)));
                buf.push('h');
                buf
            }
            gimli::DW_ATE_address => {
                format!("0x{:x}", u64::from_bytes(slice, endian))
            }
            _ => String::from("?")
        }
    }
}

struct PointerType <'a> {
    name: Option<&'a str>,
    vtype: Type,
}

impl <'a>PointerType<'a> {
    fn name(&self, dwarf: &Dwarf) -> String {
        match self.name {
            Some(name) => name.to_string(),
            None => format!("*{}", unwind_type(self.vtype, dwarf).name(dwarf))
        }
    }

    fn display(&self, value: u64) -> String {
        format!("<0x{:x}>", value)
    }

    fn value(&self, location: Location) -> String {
        let endian = ENDIAN.access().unwrap();
        if let Ok(slice) = location_memory(location, BitByteSize::Byte(8), endian) {
            let value = <u64>::from_bytes(&slice, endian);
            self.display(value)
        } else {
            "?".to_string()
        }
    }
}

struct ModifierType<'a> {
    name: Option<&'a str>,
    vtype: Type,
    modifier: Modifier
}

impl <'a>ModifierType<'a> {
    fn name(&self) -> String {
        match self.name {
            Some(name) => name.to_string(),
            None => self.modifier.to_string()
        }
    }

    fn value(
        &self,
        location: Location,
        res: &mut Vec<(usize, String)>,
        depth: usize,
        dwarf: &Dwarf
    ) -> String {
        let next = unwind_type(self.vtype, dwarf)
        .value(location, res, depth, dwarf);
        format!("{} {}", self.name(), next)
    }
}

enum Modifier {
    Atomic,
    Const,
    Immutable,
    Shared,
    Volatile
} // non-exhaustive

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Atomic => write!(f, "atomic"),
            Self::Const => write!(f, "const"),
            Self::Immutable => write!(f, "immutable"),
            Self::Shared => write!(f, "shared"),
            Self::Volatile => write!(f, "volatile"),
        }
    }
}

struct ArrayType<'a> {
    name: Option<&'a str>,
    vtype: Type,
    size: Option<BitByteSize>,
    count: Option<u64>
}

impl <'a>ArrayType<'a> {
    fn name(&self, dwarf: &Dwarf) -> String {
        match self.name {
            Some(name) => name.to_string(),
            None => format!("[{}]", unwind_type(self.vtype, dwarf).name(dwarf))
        }
    }

    fn value(&self, location: Location, res: &mut Vec<(usize, String)>, depth: usize, dwarf: &Dwarf) -> String {
        if self.size.is_none() && self.count.is_none() {
            if let Some(name) = self.name {
                return format!("{} []", name);
            } else {
                return "[]".to_string();
            }
        };

        let vtype = unwind_type(self.vtype, dwarf);
        let address = match location {
            Location::Address(address) => address,
            Location::Register(_) => if let Some(name) = self.name {
                return format!("{} [?]", name);
            } else {
                return "[?]".to_string();
            }
        };

        let element_size = match vtype.size(dwarf) {
            BitByteSize::Byte(size) => size,
            BitByteSize::Bit(_) => if let Some(name) = self.name {
                return format!("{} [?]", name);
            } else {
                return "[?]".to_string();
            }
        };

        let mut new_buf = Vec::new();

        let count = if self.count.is_some() {
            self.count.unwrap()
        } else {
            let array_size = match self.size.unwrap() {
                BitByteSize::Byte(size) => size,
                BitByteSize::Bit(_) => if let Some(name) = self.name {
                    return format!("{} [?]", name);
                } else {
                    return "[?]".to_string();
                }
            };
            array_size/element_size
        };

        for index in 0..count {
            let mut temp_buf = Vec::new();
            let element = vtype.value(
                Location::Address(address+(index*element_size)),
                &mut temp_buf,
                depth+1,
                dwarf
            );
            new_buf.push((depth+1, element));
            new_buf.append(&mut temp_buf);
            new_buf.last_mut().unwrap().1.push(',');
        }

        res.append(&mut new_buf);
        if let Some(last) = res.last_mut(){
            last.1.pop();
        }
        res.push((depth, "]".to_string()));
        if let Some(name) = self.name {
            format!("{} [", name)
        } else {
            "[".to_string()
        }
    }
}


struct StructType<'a> {
    name: Option<&'a str>,
    size: BitByteSize,
    members: Vec<Member<'a>>
}

impl <'a>StructType<'a> {
    fn value(&self, location: Location, res: &mut Vec<(usize, String)>, depth: usize, dwarf: &Dwarf) -> String {
        if self.size.is_zero() {
            if let Some(name) = self.name {
                return format!("{} {}", name, "{}");
            } else {
                return "{}".to_string();
            }
        };

        let address = match location {
            Location::Address(address) => address,
            Location::Register(_) => if let Some(name) = self.name {
                return format!("{} {}", name, "{?}");
            } else {
                return "{?}".to_string();
            }
        };

        let mut new_buf = Vec::new();

        for member in &self.members {
            let mut temp_buf = Vec::new();
            let vtype = unwind_type(member.vtype, dwarf);
            let member_value = vtype.value(
                Location::Address(address+member.offset),
                &mut temp_buf,
                depth+1,
                dwarf
            );

            new_buf.push((depth+1, format!("{} {}: {}", vtype.name(dwarf), member.name, member_value)));
            new_buf.append(&mut temp_buf);
            new_buf.last_mut().unwrap().1.push(',');
        };

        res.append(&mut new_buf);
        if let Some(res) = res.last_mut() {
            res.1.pop();
        };
        res.push((depth, "}".to_string()));
        if let Some(name) = self.name {
            format!("{} {}", name, '{')
        } else {
            "{".to_string()
        }
    }
}

struct Member<'a> {
    name: &'a str,
    vtype: Type,
    offset: u64, //data_member_location
}

struct EnumType<'a> {
    name: &'a str,
    vtype: Option<Type>,
    size: Option<BitByteSize>,
    enumerators: Vec<Enumerator<'a>>
}

impl <'a>EnumType<'a> {
    fn value(&self, location: Location, dwarf: &Dwarf) -> String {
        let endian = ENDIAN.access().unwrap();
        let size = match self.size {
            Some(size) => size,
            None => unwind_type(self.vtype.unwrap(), dwarf).size(dwarf)
        };
        if let Ok(slice) = location_memory(location, size, endian) {
            let normal = match slice.len() {
                1    => u8::from_bytes(&slice, endian) as u64,
                2    => u16::from_bytes(&slice, endian) as u64,
                3..4 => u32::from_bytes(&slice, endian) as u64 ,
                _ => u64::from_bytes(&slice, endian) as u64,
            };

            for member in &self.enumerators {
                let value = member.constant;
                if value == normal {
                    return format!("{}::{}", self.name, member.name);
                }

            };
            format!("{}::?", self.name)
        } else {
            format!("{}::?", self.name)
        }
    }
}

#[derive(Debug)]
struct Enumerator<'a> {
    name: &'a str,
    constant: u64
}

#[derive(Clone, Copy)]
pub enum BitByteSize {
    Bit(u64),
    Byte(u64),
}

impl BitByteSize {
    fn is_zero(&self) -> bool {
        match self {
            Self::Bit(0)|Self::Byte(0) => true,
            _ => false
        }
    }
}

struct TypeDef<'a> {
    name: &'a str,
    vtype: Option<Type>
}

impl <'a>TypeDef<'a> { // when we define a type, we dont want to display the types it was composed of, so we display only the last one as that is our value (or brackets when its a struct or an array)
    fn value(&self, location: Location, res: &mut Vec<(usize, String)>, depth: usize, dwarf: &Dwarf) -> String {
        if let Some(vtype) = self.vtype {
            let next = unwind_type(vtype, dwarf);
            let text = next.value(location, res, depth, dwarf);
            let last = text.split_ascii_whitespace().last();
            format!("{} {}", self.name, last.unwrap_or(""))
        } else {
            self.name.to_string()
        }
    }
}


fn unwind_type<'a>(debug_info_offset: Type, dwarf: &'a Dwarf) -> TypeDisplay<'a> {
    let type_entry = get_unit_entry_offset(debug_info_offset, dwarf);
    let unit = dwarf.unit(dwarf.unit_header(type_entry.1).unwrap()).unwrap();
    let entry = unit.entry(type_entry.0).unwrap();

    let name = match entry.attr_value(gimli::DW_AT_name) {
        Some(value) => Some(string(value, dwarf)),
        None => None
    };

    match entry.tag() {
        gimli::DW_TAG_base_type => {
            let encoding = encoding(entry.attr_value(gimli::DW_AT_encoding).unwrap());
            let size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Some(value) => BitByteSize::Byte(number(value)),
                None => BitByteSize::Bit(number(entry.attr_value(gimli::DW_AT_bit_size).unwrap()))
            };
            let endian: Option<gimli::RunTimeEndian> = match entry.attr_value(gimli::DW_AT_endianity) {
                Some(value) => {
                    let res = gimli::DwEnd(value.u8_value().unwrap());
                    if res == gimli::DW_END_big {Some(Endian::Big)}
                    else if res == gimli::DW_END_little {Some(Endian::Little)}
                    else {None}
                },
                None => None
            };
            TypeDisplay::Base(BaseType {
                name,
                encoding,
                size,
                endian
            })
        },
        gimli::DW_TAG_pointer_type
        | gimli::DW_TAG_reference_type => {
            let vtype = debug_reference(entry.attr_value(gimli::DW_AT_type).unwrap(), &unit);
            TypeDisplay::Pointer(PointerType {
                name,
                vtype
            })
        },
        gimli::DW_TAG_atomic_type
        | gimli::DW_TAG_const_type
        | gimli::DW_TAG_immutable_type
        | gimli::DW_TAG_shared_type
        | gimli::DW_TAG_volatile_type => {
            let vtype = debug_reference(entry.attr_value(gimli::DW_AT_type).unwrap(), &unit);
            TypeDisplay::Modifier(ModifierType {
                name,
                vtype,
                modifier: match entry.tag {
                    gimli::DW_TAG_atomic_type => Modifier::Atomic,
                    gimli::DW_TAG_const_type => Modifier::Const,
                    gimli::DW_TAG_immutable_type => Modifier::Immutable,
                    gimli::DW_TAG_shared_type => Modifier::Shared,
                    gimli::DW_TAG_volatile_type => Modifier::Volatile,
                    _ => unimplemented!()
                }
            })
        },
        gimli::DW_TAG_array_type => {
            let vtype = debug_reference(entry.attr_value(gimli::DW_AT_type).unwrap(), &unit);
            let size = match entry.attr_value(gimli::DW_AT_bit_size) {
                Some(value) => Some(BitByteSize::Bit(number(value))),
                None => None
            };
            let size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Some(value) => Some(BitByteSize::Byte(number(value))),
                None => size
            };

            let mut cursor = unit.entries_at_offset(type_entry.0).unwrap();
            cursor.next_entry().unwrap();

            let count = match cursor.next_entry() {
                Ok(true) => {
                    let entry = cursor.current().unwrap();
                    if let Some(upper) = entry.attr_value(gimli::DW_AT_upper_bound) {
                        println!("noo");
                        let upper = number(upper);
                        if let Some(lower) =  entry.attr_value(gimli::DW_AT_lower_bound) {
                            Some(upper-number(lower)+1)
                        } else {
                            Some(upper+1)
                        }
                    } else {
                        println!("ohhh");
                        if let Some(count) = entry.attr_value(gimli::DW_AT_count) {
                            println!("{:?}", count);
                            Some(number(count))
                        } else {
                            None
                        }
                    }
                },
                _ => None
            };

            println!("{:?}", count);

            TypeDisplay::Array(ArrayType {
                name: name,
                vtype,
                size,
                count
            })
        },
        gimli::DW_TAG_structure_type
        | gimli::DW_TAG_union_type
        | gimli::DW_TAG_class_type => {
            let size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Some(value) => BitByteSize::Byte(value.udata_value().unwrap()),
                None => BitByteSize::Bit(entry.attr_value(gimli::DW_AT_bit_size).unwrap().udata_value().unwrap())
            };
            let mut members: Vec<Member> = Vec::new();
            let mut entries = unit.entries_at_offset(type_entry.0).unwrap();
            entries.next_entry().unwrap();

            let mut first = true;

            loop {
                let current = if first {
                    first = false;
                    if entries.next_entry().unwrap() {
                        match entries.current() {
                            Some(entry) => entry,
                            None => break
                        }
                    } else {
                        break;
                    }
                } else {
                    match entries.next_sibling().unwrap() { // iter only over the siblings
                        Some(entry) => entry,
                        None => break
                    }
                };

                if current.tag() != gimli::DW_TAG_member {
                    continue;
                }
                let name = string(current.attr_value(gimli::DW_AT_name).unwrap(), dwarf);
                let vtype = debug_reference(current.attr_value(gimli::DW_AT_type).unwrap(), &unit);
                let offset = if let Some(attr) = current.attr_value(gimli::DW_AT_data_member_location) { // i know im not handling the data_bit_offset, but bit structures seem a little over the top
                    number(attr)
                } else {
                    0
                };
                members.push(Member {
                    name,
                    vtype,
                    offset
                });
            };
            TypeDisplay::Struct(StructType{
                name,
                size,
                members
            })
        },
        gimli::DW_TAG_enumeration_type => {
            let size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Some(value) => Some(BitByteSize::Byte(number(value))),
                None => match entry.attr_value(gimli::DW_AT_bit_size) {
                    Some(value) => Some(BitByteSize::Bit(number(value))),
                    None => None
            }};
            let vtype = match entry.attr_value(gimli::DW_AT_type) {
                Some(value) => Some(DebugInfoOffset(number(value) as usize)),
                None => None
            };
            let mut enumerators: Vec<Enumerator> = Vec::new();
            let mut entries = unit.entries_at_offset(type_entry.0).unwrap();
            entries.next_entry().unwrap(); // Stepping into the children
            let mut first = true;
            println!("YOOO");
            loop {
                println!("DAMN");

                let current = if first {
                    first = false;
                    if entries.next_entry().unwrap() {
                        match entries.current() {
                            Some(entry) => entry,
                            None => break
                        }
                    } else {
                        break;
                    }
                } else {
                    match entries.next_sibling().unwrap() { // iter only over the siblings
                        Some(entry) => entry,
                        None => break
                    }
                };

                if current.tag() != gimli::DW_TAG_enumerator {
                    continue;
                }
                let name = string(current.attr_value(gimli::DW_AT_name).unwrap(), dwarf);
                let constant = number(current.attr_value(gimli::DW_AT_const_value).unwrap());
                enumerators.push(Enumerator {
                    name,
                    constant
                });
            };
            TypeDisplay::Enum(EnumType {
                name: name.unwrap(),
                vtype,
                size,
                enumerators
            })
        },
        gimli::DW_TAG_typedef => {
            let vtype = match entry.attr_value(gimli::DW_AT_type) {
                Some(value) => Some(debug_reference(value, &unit)),
                None => None
            };
            TypeDisplay::Def(TypeDef{
                name: name.unwrap(),
                vtype
            })
        },
        gimli::DW_TAG_packed_type
        | gimli::DW_TAG_restrict_type 
        | gimli::DW_TAG_rvalue_reference_type => { // fallback for unused modifier types
            let vtype = match entry.attr_value(gimli::DW_AT_type) {
                Some(value) => Some(debug_reference(value, &unit)),
                None => None
            };
            TypeDisplay::Def(TypeDef{
                name: "",
                vtype
            })
        },
        _ => TypeDisplay::Def(TypeDef{ // Real FallBack for any unimplemented type
                name: "",
                vtype: None
        })
    }
}

fn location_memory(location: Location, size: BitByteSize, endian: Endian) -> Result<Vec<u8>, ()> {
    let read_size = match size {
        BitByteSize::Byte(byte) => byte,
        BitByteSize::Bit(bit) => bit.div_ceil(8)
    };
    let res = match location {
        Location::Register(register) => {
            let mut bind = REGISTERS.access();
            let number = match_register(&register, bind.as_mut().unwrap());
            let data = match endian {
                gimli::RunTimeEndian::Big => number.to_be_bytes(),
                gimli::RunTimeEndian::Little => number.to_le_bytes()
            };
            Vec::from(&data[0..read_size as usize])
        }
        Location::Address(address) => {
            let slice = trace::read_memory(address, read_size as usize).expect("Memory Read Exception");
            slice
        }
    };
    if let BitByteSize::Bit(_) = size { // i have not seen any lang use it, even when compiled to optimize size (for now bit size is unimplemented but wont panic)
        Err(())
    } else {
        Ok(res)
    }
}


// CODE DISASSEMBLY


const DEPTH: u64 = 64; // iteration limit

#[derive(Debug, Clone)]
pub struct Assembly {
    pub text: String,
    pub bytes: String,
    pub addresses: Vec<u64>
}

impl Assembly {
    pub fn create(rip: u64) -> Result<(Self, usize), ()> {
        let range = match trace::get_map_range(rip) {
            Some(range) => range,
            None => return Err(())
        };
        let base = (rip - rip%8 - 512).max(range.start);
        let end = (base + 1024).min(range.end);
        let size = end - base;

        let bytes = trace::read_memory(base, size as usize)?;

        let (pointer, line) = align_pointer(base, rip, &bytes)?;

        let assembly = disassemble_code(pointer, &bytes[(pointer - base) as usize..])?;
        Ok((assembly, line))
    }
}

pub fn align_pointer(address: u64, rip: u64, bytes: &[u8]) -> Result<(u64, usize), ()> { // the second one is the line number
    for offset in 0..DEPTH {
        let mut decoder = Decoder::with_ip(64, &bytes[offset as usize..], address+offset, DecoderOptions::NONE);
        let mut instruction = Instruction::default();
        let mut counter = 0;
        while decoder.can_decode() {
            decoder.decode_out(&mut instruction);
            if instruction.is_invalid() {
                break;
            }
            if instruction.ip() == rip {
                return Ok((address+offset, counter));
            }
            counter += 1;
        }
    };
    Err(()) // WOW HOW
}

pub fn disassemble_code(address: u64, bytes: &[u8]) -> Result<Assembly, ()> {
    let mut decoder = Decoder::with_ip(64, bytes, address, DecoderOptions::NONE);
    let mut formatter = NasmFormatter::new();

    formatter.options_mut().set_digit_separator("");
    formatter.options_mut().set_first_operand_char_index(4);

    let mut instructions = String::new();
    let mut instructions_bytes = String::new();
    let mut addresses: Vec<u64> = Vec::new();

    let mut output = String::new();
    let mut instruction = Instruction::default();

    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        output.clear();

        formatter.format(&instruction, &mut output);
        addresses.push(instruction.ip());

        instructions.push_str(&output);
        instructions.push('\n');

        let index = (instruction.ip() - address) as usize;

        let bytes = Vec::from(&bytes[index..index+instruction.len()]);

        let byte_string: String = bytes.iter().map(|b| format!("{:02x} ", b)).collect();

        instructions_bytes.push_str(&byte_string);
        instructions_bytes.push('\n');
    };

    Ok(Assembly {
        text: instructions,
        bytes: instructions_bytes,
        addresses
    })
}



// HELPER FUNCTIONS

pub fn normal(rip: u64) -> u64 {
    match EXEC_SHIFT.access().as_ref() {
        Some(offset) => rip - offset,
        None => rip
    }
}

pub fn anti_normal(address: u64) -> u64 {
    match EXEC_SHIFT.access().as_ref() {
        Some(offset) => address + offset,
        None => address
    }
}

fn get_unit_entry_offset(offset: DebugInfoOffset, dwarf: &Dwarf) -> (gimli::UnitOffset, DebugInfoOffset) { // unit offset AND the offset of the Unit
    let mut units = dwarf.units();
    while let Some(unit) = units.next().unwrap() {
        match offset.to_unit_offset(&unit) {
            Some(offset) => return (offset, unit.debug_info_offset().unwrap()),
            _ => ()
        };
    }
    panic!(); // in case it fails, meaning there is an error with the dwarf data OR an error on my side
}

// Attribute Value, interpretation

fn string<'a>(attr: gimli::AttributeValue<EndianSlice<'a, Endian>>, dwarf: &'a Dwarf) -> &'a str {
    match attr {
        gimli::AttributeValue::DebugLineStrRef(offset) => dwarf.line_string(offset).unwrap().to_string().unwrap(),
        gimli::AttributeValue::DebugStrRef(offset) => dwarf.string(offset).unwrap().to_string().unwrap(),
        gimli::AttributeValue::String(string) => string.to_string().unwrap(),
        _ => ""
    }
}

fn number(attr: gimli::AttributeValue<EndianSlice<'_, Endian>>) -> u64 {
    match attr {
        gimli::AttributeValue::Addr(val) => val,
        gimli::AttributeValue::Udata(val) => val,
        gimli::AttributeValue::Data1(data) => data as u64,
        gimli::AttributeValue::Data2(data) => data as u64,
        gimli::AttributeValue::Data4(data) => data as u64,
        gimli::AttributeValue::Data8(data) => data as u64,
        _ => 0
    }
}

fn encoding(attr: gimli::AttributeValue<EndianSlice<'_, Endian>>) -> gimli::DwAte {
    match attr {
        gimli::AttributeValue::Encoding(e) => e,
        _ => unimplemented!() // dont call encoding on a non encoding attribute
    }
}

fn debug_reference(attr: gimli::AttributeValue<EndianSlice<'_, Endian>>, unit: &Unit) -> DebugInfoOffset {
    match attr {
        gimli::AttributeValue::DebugInfoRef(value) => value,
        gimli::AttributeValue::UnitRef(value) => value.to_debug_info_offset(unit).unwrap(),
        _ => DebugInfoOffset(0)
    }
}

// Creating numbers from bytes

trait FromBytes<const N: usize> {
    fn from_bytes(bytes: &[u8], endian: Endian) -> Self;
    fn fixed(bytes: &[u8], endian: Endian) -> [u8; N];
}

macro_rules! impl_from_bytes {
    ($($t:ty), *) => {
        $(
            impl FromBytes<{std::mem::size_of::<$t>()}> for $t {
                fn from_bytes(bytes: &[u8], endian: Endian) -> Self {
                    match endian {
                        Endian::Little => <$t>::from_le_bytes(Self::fixed(bytes, endian)),
                        Endian::Big => <$t>::from_be_bytes(Self::fixed(bytes, endian))
                    }
                }
                fn fixed(bytes: &[u8], endian: Endian) -> [u8; {std::mem::size_of::<$t>()}] {
                    let size = {std::mem::size_of::<$t>()} - bytes.len();
                    let mut add = vec![0;size];
                    match endian {
                        Endian::Big => {
                            let mut vec = Vec::from(bytes);
                            add.append(&mut vec);
                            add.try_into().unwrap()
                        }
                        Endian::Little => {
                            let mut vec = Vec::from(bytes);
                            vec.append(&mut add);
                            vec.try_into().unwrap()
                        }
                    }
                }
            }
        )*
    };
}

impl_from_bytes!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);