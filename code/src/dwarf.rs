use std::path::PathBuf;
use std::collections::HashMap;


use gimli::DebugInfoOffset;
use gimli::UnitHeader;
use gimli::EndianSlice;
use gimli::UnwindSection;

use object::{Object, ObjectSection};

use crate::data::*;
use crate::trace;

const NOMASK:u64 = 0xFFFFFFFFFFFFFFFF; // because gimli's builtin function is stupid and i dont want make a new function for it

#[derive(PartialEq, Clone)]
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

pub type Endian = gimli::RunTimeEndian;
type Section<'data> = std::borrow::Cow<'data, [u8]>;
pub type DwarfSections<'data> = gimli::DwarfSections<Section<'data>>;
pub type Dwarf<'a> = gimli::Dwarf<EndianSlice<'a, gimli::RunTimeEndian>>;

pub struct EhFrame<'a> {
    frame: gimli::EhFrame<EndianSlice<'a, gimli::RunTimeEndian>>,
    section_base: u64,
    endian: Endian
}

trait SectionsToDwarf {
    fn dwarf(&self, endian: Endian) -> Dwarf;
}

impl SectionsToDwarf for DwarfSections<'_> {
    fn dwarf(&self, endian: Endian) -> Dwarf {
        self.borrow(|section| gimli::EndianSlice::new(Section::as_ref(section), endian))
    }
}

// Reading LINE_DATA

fn load_dwarf(binary: &Vec<u8>) -> (DwarfSections, object::File, Endian) {
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

    (dwarf_sections, object, endian)
}

type Unit<'a> = gimli::Unit<EndianSlice<'a, gimli::RunTimeEndian>, usize>;

fn load_source(dwarf: Dwarf) { // this is a hell of a function, but gimli doesnt provide much better ways other than this, so ill leave comments

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

// FUNCTION RANGES

type FunctionRange = std::ops::Range<u64>;

pub struct FunctionIndex<'a, Unit = DebugInfoOffset, FunctionOffset = DebugInfoOffset> {
    func_hash: HashMap<u64, FunctionOffset>,
    range_hash: HashMap<Unit, Vec<FunctionRange>>,
    subtype_parent: HashMap<FunctionOffset, Option<&'a str>> //kinda optional, but VERY useful // TODO, later, dont save name, but .debug_string offset !! (well see)
}

impl <'a>FunctionIndex<'a> {
    fn new() -> Self {
        FunctionIndex {
            func_hash: HashMap::new(),
            range_hash: HashMap::new(),
            subtype_parent: HashMap::new()
        }
    }

    fn insert_function(&mut self, range: FunctionRange, function_entry: DebugInfoOffset ,unit: DebugInfoOffset, subtype_parent: Option<&'a str>) {
        let range_hash = &mut self.range_hash;
        if range_hash.contains_key(&unit) {
            range_hash.get_mut(&unit).unwrap().push(range.clone());
        } else {
            range_hash.insert(unit, vec![range.clone()]);
        }
        let func_hash = &mut self.func_hash;
        func_hash.insert(range.start, function_entry);
        let parent_hash = &mut self.subtype_parent;
        parent_hash.insert(function_entry, subtype_parent);
    }

    fn direct_address(&self, address: u64) -> DebugInfoOffset {
        self.func_hash[&address]
    }

    fn get_function(&self, address: u64, unit: DebugInfoOffset) -> Option<DebugInfoOffset> {
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
}

fn parse_functions(dwarf: Dwarf<'static>) {
    let mut function_index = FunctionIndex::new();

    let mut declarations: HashMap<DebugInfoOffset, &str>  = HashMap::new(); // mapping declaration function offset to its parent for later use, and they sit outside of units, in case some would be defined in different CU, tho unlikely, also the reason for DebugInfoOffset instead

    let mut unit_headers = dwarf.units();


    while let Some(unit_header) = unit_headers.next().unwrap() {
        let unit = dwarf.unit(unit_header).unwrap();
        let base_address = unit.low_pc; // for ranges attr
        let mut entries = unit.entries();

        let mut parent_stack: Vec<&str> = Vec::new(); // i could technically always apppend the entire tree yk, but i think just the last parent will be infact enough, but yk TODO, we'll see

        loop {
            let prev_entry = entries.current().unwrap(); // we need to do this here for coherent 'continue' logic in this loop
            if entries.next_depth() > prev_entry.depth() { // even if this is a null entry, it will work, as the depth can never be higher after the null (which literally decreases the depth)
                parent_stack.push(match prev_entry.attr_value(gimli::DW_AT_name) {
                    Some(val) => val.string_value(&dwarf.debug_str).unwrap().to_string().unwrap(),
                    None => ""
                });
            };

            match entries.next_entry() {
                Ok(some) => if !some {
                    parent_stack.pop();
                    continue;
                },
                Err(_) => break
            };

            let entry = entries.current().unwrap();

            if entry.tag() != gimli::DW_TAG_subprogram {continue;}
            if entry.attr(gimli::DW_AT_declaration).is_some() {
                declarations.insert(entry.offset.to_debug_info_offset(&unit).unwrap(), *parent_stack.last().unwrap_or(&""));
                continue;
            }; // save and skip declarations (no pc for us and such), they will be handled later

            let parent: &str = match entry.attr(gimli::DW_AT_specification) {
                Some(function_declaration) => *declarations.get(&DebugInfoOffset(function_declaration.offset_value().unwrap())).unwrap(),
                None => *parent_stack.last().unwrap_or(&"")
            };

            let applicable_parent = if parent == "" {None} else {Some(parent)}; // apart from the coherency, it saves us 7 bytes per empty parent

            if let Some(value) = entry.attr_value(gimli::DW_AT_ranges) {
                let range_offset = value.offset_value().unwrap();
                let mut ranges = dwarf.ranges.ranges(
                    gimli::RangeListsOffset(range_offset),
                    unit.encoding(),
                    base_address,
                    &dwarf.debug_addr,
                    unit.addr_base
                ).expect("WTF RANGES");

                while let Some(range) = ranges.next().unwrap_or(None) {
                    function_index.insert_function(range.begin..range.end, entry.offset.to_debug_info_offset(&unit).unwrap(), unit.debug_info_offset().unwrap(), applicable_parent);
                }

                continue;
            }

            let low_pc = match entry.attr_value(gimli::DW_AT_low_pc) {
                Some(value) => value.udata_value().unwrap(),
                None => continue // if we dont have the ranges or the pc definitions, then saving the function only breaks the code
            };

            let high_pc = match entry.attr_value(gimli::DW_AT_high_pc) {
                Some(value) => value.udata_value().unwrap(),
                None => continue
            };

            let function_range = low_pc..low_pc+high_pc;

            function_index.insert_function(function_range, entry.offset.to_debug_info_offset(&unit).unwrap(), unit.debug_info_offset().unwrap(), applicable_parent);
        }
    }

    let mut ibind = INTERNAL.access();
    ibind.function_index = Some(function_index);
}

// STACK UNWINDING

struct UnwindInfo {
    row: Option<gimli::UnwindTableRow<usize>>,
    ctx: gimli::UnwindContext<usize>
}


fn get_unwind_for_address<'a>(address: u64, eh_frame: &'a EhFrame) -> UnwindInfo {
    let bases = gimli::BaseAddresses::default().set_eh_frame(eh_frame.section_base);

    let fde = eh_frame.frame.fde_for_address(&bases, address, gimli::EhFrame::cie_from_offset).unwrap();

    let mut res = UnwindInfo {
        row: None,
        ctx: gimli::UnwindContext::new()
    };

    let unwind_info = fde.unwind_info_for_address(&eh_frame.frame, &bases, &mut res.ctx, address).unwrap();

    res.row = Some(unwind_info.clone());

    res
}


fn get_cfa(unwind: &UnwindInfo, regs: &mut nix::libc::user_regs_struct, eh_frame: &EhFrame, encoding: gimli::Encoding) -> Result<u64, ()> {
    let cfa = unwind.row.as_ref().unwrap().cfa();
    match cfa {
        gimli::CfaRule::RegisterAndOffset {
            register,
            offset
        } => Ok((*match_register(register, regs) as i64 + offset) as u64),
        gimli::CfaRule::Expression(expression) => {
            let expression = expression.get(&eh_frame.frame).unwrap();
            let piece = eval_expression(&expression, regs, None, None, encoding)?[0];
            match piece.location {
                gimli::Location::Value {value} => Ok(value.to_u64(NOMASK).unwrap()),
                _ => panic!("i am so tired")
            }
        },
    }
}

fn unwind_registers(unwind: &UnwindInfo, cfa: u64, regs: &mut nix::libc::user_regs_struct, eh_frame: &EhFrame,encoding: gimli::Encoding) -> Result<(), ()> {

    let rules = unwind.row.as_ref().unwrap().registers();

    for (reg, rule) in rules {
        let value = unwind_register(rule, cfa, regs, eh_frame, encoding);
        match value {
            Ok(value) => *match_register(reg, regs) = value,
            Err(_) => ()
        };
    };

    Ok(())
}

fn unwind_register(register_rule: &gimli::RegisterRule<usize>, cfa: u64, regs: &mut nix::libc::user_regs_struct, eh_frame: &EhFrame, encoding: gimli::Encoding) -> Result<u64, ()> {
    match register_rule {
        gimli::RegisterRule::Offset(addr_offset) => Ok(slice_to_u64(&unwind_memory((cfa as i64 + addr_offset) as u64, 8))),
        gimli::RegisterRule::Expression(expression) => {
            let expression = expression.get(&eh_frame.frame).unwrap();
            let piece = eval_expression(&expression, regs, Some(cfa), None, encoding)?[0];
            Ok(slice_to_u64(&unwind_memory(
                match piece.location {
                    gimli::Location::Value {value} => value.to_u64(NOMASK).unwrap(),
                    _ => panic!("MORE EXPLANATIONS")
                }
            , 8))
        )},
        gimli::RegisterRule::ValOffset(offset) => Ok((cfa as i64 + offset) as u64),
        gimli::RegisterRule::ValExpression(expression) => {
            let expression = expression.get(&eh_frame.frame).unwrap();
            let piece = eval_expression(&expression, regs, Some(cfa), None, encoding)?[0];
            Ok(match piece.location {
                gimli::Location::Value {value} => value.to_u64(NOMASK).unwrap(),
                _ => panic!("EXPLAIN YOURSELF PEASANT")
            })
        },
        gimli::RegisterRule::Register(register) => Ok(*match_register(register, regs)),
        gimli::RegisterRule::Constant(value) => Ok(*value),
        gimli::RegisterRule::SameValue => Err(()),
        _ => unimplemented!() // NOW THIS IS NOT IMPLEMENTED SO LETS HOPE IT WONT BE NEEDED YK :P
    }
}

fn match_register<'a>(register: &gimli::Register, regs: &'a mut nix::libc::user_regs_struct) -> &'a mut u64 { // Only these registers are used in expression and locations, if not im gonna cry
    match *register {
        gimli::X86_64::RAX => &mut regs.rax,
        gimli::X86_64::RBX => &mut regs.rbx,
        gimli::X86_64::RCX => &mut regs.rcx,
        gimli::X86_64::RDX => &mut regs.rdx,
        gimli::X86_64::RSI => &mut regs.rsi,
        gimli::X86_64::RDI => &mut regs.rdi,
        gimli::X86_64::RSP => &mut regs.rsp,
        gimli::X86_64::RBP => &mut regs.rbp,
        gimli::X86_64::R8 => &mut regs.r8,
        gimli::X86_64::R9 => &mut regs.r9,
        gimli::X86_64::R10 => &mut regs.r10,
        gimli::X86_64::R11 => &mut regs.r11,
        gimli::X86_64::R12 => &mut regs.r12,
        gimli::X86_64::R13 => &mut regs.r13,
        gimli::X86_64::R14 => &mut regs.r14,
        gimli::X86_64::R15 => &mut regs.r15,
        gimli::X86_64::RA => &mut regs.rip,
        _ => unimplemented!("THIS HAS TO BE ENOUGH RIGHT") // use result later - URL
    }
}

fn unwind_memory(address: u64, size: u8) -> Vec<u8> { // for reading small amounts of data from the memory (a wrapper)
    trace::read_memory(address, size as usize).expect("CORRUPTED DWARF OR IDK WTFFFF")
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
        match result {
            gimli::EvaluationResult::Complete => break,
            gimli::EvaluationResult::RequiresMemory { address, size, space, base_type } => { // TODO if the size needs implementation
                let data = unwind_memory(address, size);
                result = evaluation.resume_with_memory(gimli::Value::U64(slice_to_u64(&data))).expect("Something failed");
            },
            gimli::EvaluationResult::RequiresRegister { register, base_type } => { // TODO if the type needs implementation
                result = evaluation.resume_with_register(gimli::Value::U64(*match_register(&register, regs))).map_err(|_| ())?;
            },
            gimli::EvaluationResult::RequiresFrameBase => {
                result = evaluation.resume_with_frame_base(frame_base.ok_or(())?).map_err(|_| ())?;
            },
            gimli::EvaluationResult::RequiresCallFrameCfa => {
                result = evaluation.resume_with_call_frame_cfa(cfa.ok_or(())?).map_err(|_| ())?;
            }
            _ => unimplemented!("DAMN WHAT")
        }
    }
    let result = evaluation.result();
    Ok(result)
}

fn slice_to_u64(slice: &[u8]) -> u64 {
    let bytes: [u8; 8] = slice.try_into().unwrap(); // please just dont pass down a wrong size or that oki
    let endian: Endian = INTERNAL.access().eh_frame.as_ref().unwrap().endian;

    match endian {
        Endian::Big => u64::from_be_bytes(bytes),
        Endian::Little => u64::from_le_bytes(bytes)
    }
}

// CALL STACK PARSING

struct CallStack (Vec<Function>);

impl  CallStack {
    fn new() -> Self {
        CallStack(Vec::new())
    }
}

type Type = DebugInfoOffset;

struct Function {
    name: String,
    parameters: Option<Vec<Parameter>>,
    variables: Option<Vec<Variable>>,
    return_type: Option<Type>,
    debug_info_offset: Option<DebugInfoOffset>,
}

struct Variable {
    name: String,
    location: Option<Location>,
    constant: Option<u64>,
    vtype: Type
}

struct Parameter {
    name: String,
    location: Location,
    vtype: Type
}

fn call_stack<'a>() -> Result<CallStack, ()> {
    let mut call_stack = CallStack::new();

    let binding = INTERNAL.access();
    let mut registers = binding.registers.unwrap();

    loop {
        if unwind(&mut call_stack, &binding, &mut registers)? {
            break;
        };
    }

    Ok(call_stack)
}

fn unwind (
    call_stack: &mut CallStack,
    binding: &std::sync::MutexGuard<'_, Internal>,
    regs: &mut nix::libc::user_regs_struct
) -> Result<bool, ()> {
    // parsed global DWARF INFO // make wrapper functions for these data
    let eh_frame = binding.eh_frame.as_ref().unwrap();
    let line_addresses = binding.line_addresses.as_ref().unwrap();
    let source_map = binding.source_files.as_ref().unwrap();
    let function_index = binding.function_index.as_ref().unwrap();
    let dwarf = binding.dwarf.as_ref().unwrap().dwarf(eh_frame.endian);

    // We need info about the function and all
    let index = line_addresses.get(&normal(regs.rip)).ok_or(())?; // if we arent in a source file, we cannot be debugging the info (can happen using step while stepping into a dynamic library function, and therefore it will not show any of the call stack info, as we have no Dwarf info)
    let unit = source_map.get(&index.hash_path).unwrap()[index.index].compile_unit; // our function index is unit organised for faster lookup speed. this is possible thanks to the hashmap, which has constant access time, while searching through ranges is linear access time (in the end, this is MUCH faster, especially with more functions and source files)
    let function = function_index.get_function(normal(regs.rip), unit).unwrap(); // now we get our function offset into the debug_info_section
    let unit_header = dwarf.debug_info.header_from_offset(unit).unwrap();
    let dwarf_unit = dwarf.unit(unit_header).unwrap();

    let offset = function.to_unit_offset(&dwarf_unit).unwrap();
    let mut entries = dwarf_unit.entries_at_offset(offset).unwrap();
    let die = entries.current().unwrap();
    let (mut function_info, frame_attribute) = extract_function_info(die, &dwarf);

    let unwind_info = get_unwind_for_address(normal(regs.rip), eh_frame);
    let encoding = dwarf_unit.encoding();
    let cfa = get_cfa(&unwind_info, regs, eh_frame, encoding)?;

    unwind_registers(&unwind_info, cfa, regs, eh_frame, encoding)?;

    let frame_base = if frame_attribute.is_some() {
        let expression = frame_attribute.unwrap().exprloc_value().unwrap();
        let frame_base = eval_expression(&expression, regs, Some(cfa), None, encoding)?[0];
        match frame_base.location {
            gimli::Location::Value {value} => Some(value.to_u64(NOMASK).unwrap()),
            gimli::Location::Register {register} => Some(*match_register(&register, regs)),
            _ => panic!("huh")
        }
    } else {
        None
    };

    extract_variables(&mut function_info, regs, frame_base, entries, &dwarf, encoding, index.line, &dwarf_unit)?;

    let main_function = check_for_main(&function_info);

    call_stack.0.push(function_info);

    if main_function {return Ok(true);}

    // we want to free up as much memory as possible before recursing the function again // maybee???
    drop(dwarf);

    Ok(false)
}

fn extract_function_info<'a>(entry: &gimli::DebuggingInformationEntry<EndianSlice<'a, Endian>, usize>, dwarf: &'a Dwarf) -> (Function, Option<gimli::AttributeValue<EndianSlice<'a, Endian>>>) {

    let (name, return_type) = match entry.attr_value(gimli::DW_AT_specification) {
        Some(specification) => {
            let declaration_offset = get_unit_entry_offset(DebugInfoOffset(specification.offset_value().unwrap()), dwarf);
            let declaration_entry = dwarf.unit(dwarf.unit_header(declaration_offset.1).unwrap()).unwrap().entry(declaration_offset.0).unwrap();
            let name = declaration_entry.attr(gimli::DW_AT_name).unwrap().string_value(&dwarf.debug_str).unwrap().to_string().unwrap();
            let return_type = match declaration_entry.attr(gimli::DW_AT_type) {
                    Some(attr) => Some(DebugInfoOffset(attr.offset_value().unwrap())),
                    None => None
            };
            (name, return_type)
        },
        None => (
            entry.attr(gimli::DW_AT_name).unwrap().string_value(&dwarf.debug_str).unwrap().to_string().unwrap(),
            match entry.attr(gimli::DW_AT_type) {
                Some(attr) => Some(DebugInfoOffset(attr.offset_value().unwrap())),
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
        parameters: None,
        variables: None,
        return_type: return_type,
        debug_info_offset: None
    }, frame_base)
}

enum Location {
    Register(gimli::Register),
    Address(u64)
}

fn extract_variables<'a>(
    function: &mut Function,
    regs: &mut nix::libc::user_regs_struct,
    frame_base: Option<u64>,
    mut entries: gimli::EntriesCursor<'_, EndianSlice<'a, Endian>>,
    dwarf: &'a Dwarf,
    encoding: gimli::Encoding,
    current_line: u64,
    unit: &Unit
) -> Result<(), ()>{
    let fn_depth = entries.depth();
    let mut first = true;
    loop {
        if first {
            first = false;
            entries.next_entry().unwrap();
        } else {
            match entries.current().unwrap().tag() {
                gimli::DW_TAG_subprogram | gimli::DW_TAG_inlined_subroutine => {entries.next_sibling().map_err(|_| ())?;} // skipping subfunctions and inlines
                _ => {entries.next_entry().map_err(|_| ())?;}
            };
        };

        if fn_depth == entries.depth() { // if the new entry has the same depth as the original fn_depth, then they are siblings and therefore we ran into the end of the function locals definition // we cant just use the null entry, because the some variables can be in deeper lexical fields, so this is the easiest
            return Ok(());
        }

        let entry = match entries.current() { // skip null entries
            Some(entry) => entry,
            None => continue
        };

        if entry.tag() == gimli::DW_TAG_variable {
            let declaration = entry.attr(gimli::DW_AT_decl_line).unwrap().udata_value().unwrap();

            if declaration > current_line {continue;}; // if you have a variable you havent declared yet, you dont want to show it right, cause its gonna be random gibberish yk

            let name = String::from(
                match entry.attr(gimli::DW_AT_name) {
                    Some(attr) => attr.string_value(&dwarf.debug_str).unwrap().to_string().unwrap(), //URL
                    None => "0"
                }
            );

            let vtype = Type{0: entry.attr(gimli::DW_AT_type).unwrap().offset_value().unwrap()};

            let location = match entry.attr(gimli::DW_AT_location) {
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
                        let loclist = gimli::LocationListsOffset(attr.udata_value().unwrap() as usize);
                        Some(get_loclist_location(loclist, dwarf, unit, regs, frame_base, encoding)?)
                    }
                },
                None => None
            };

            let constant = match entry.attr(gimli::DW_AT_const_value) {
                Some(attr) => match attr.udata_value() {
                    Some(val) => Some(val),
                    None => {println!("WTF CONSTANT"); None} // i want to know if it does some weird shit
                },
                None => None
            };

            let var = Variable {
                name,
                location,
                constant,
                vtype,
            };

            function.variables.as_mut().unwrap().push(var);
        }

        if entry.tag() == gimli::DW_TAG_formal_parameter {
            let name = String::from(
                match entry.attr(gimli::DW_AT_name) {
                    Some(attr) => attr.string_value(&dwarf.debug_str).unwrap().to_string().unwrap(), //URL
                    None => "0"
                }
            );

            let vtype = Type{0: entry.attr(gimli::DW_AT_type).unwrap().offset_value().unwrap()};

            let location = match entry.attr(gimli::DW_AT_location) {
                Some(attr) => {
                    if attr.exprloc_value().is_some() {
                        let expression = attr.exprloc_value().unwrap();
                        let piece = eval_expression(&expression, regs, None, frame_base, encoding)?[0];
                        match piece.location {
                            gimli::Location::Register {register} => Location::Register(register),
                            gimli::Location::Value {value} => Location::Address(value.to_u64(NOMASK).unwrap()),
                            gimli::Location::Address {address} => Location::Address(address),
                            _ => panic!()
                        }
                    } else {
                        let loclist = gimli::LocationListsOffset(attr.udata_value().unwrap() as usize); //URL
                        get_loclist_location(loclist, dwarf, unit, regs, frame_base, encoding)?
                    }
                },
                //Some(attr) => Some(eval_expression(&attr.exprloc_value().unwrap(), 0, 0, 0, 0, encoding, Some(frame_base)).to_u64(0).unwrap()),
                None => panic!("No param location, WHAT")
            };

            let param = Parameter {
                name,
                location,
                vtype,
            };

            function.parameters.as_mut().unwrap().push(param);
        }

        // + there are other ones but whatever for now
    }
}

fn get_loclist_location(
    loc_list: gimli::LocationListsOffset<usize>,
    dwarf: &Dwarf,
    unit: &Unit,
    regs: &mut nix::libc::user_regs_struct,
    frame_base: Option<u64>,
    encoding: gimli::Encoding) -> Result<Location, ()> {
    let mut locations = dwarf.locations(unit, loc_list).map_err(|_| ())?;
    loop {
        let entry = locations.next().map_err(|_| ())?.ok_or(())?;
        if (entry.range.begin..entry.range.end).contains(&normal(regs.rip)) {
            let expression = entry.data;
            let piece = eval_expression(&expression, regs, None, frame_base, encoding)?[0];
            match piece.location {
                gimli::Location::Value {value} => return Ok(Location::Address(value.to_u64(NOMASK).unwrap())),
                gimli::Location::Address {address} => return Ok(Location::Address(address)),
                gimli::Location::Register {register} => return Ok(Location::Register(register)),
                _ => panic!("PLEASE NOT AGAIN")
            };
        }
    }
}

fn check_for_main(info: &Function) -> bool { // function because of language specifications, for now just name matching
    info.name == "name"
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

struct BaseType<'a> {
    name: Option<&'a str>,
    encoding: gimli::DwAte,
    size: BitByteSize,
    endian: Option<Endian>
}

struct PointerType <'a> {
    name: Option<&'a str>,
    vtype: Type,
}

struct ModifierType<'a> {
    name: Option<&'a str>,
    vtype: Type,
    modifier: Modifier
}

enum Modifier {
    Atomic,
    Const,
    Immutable,
    Shared,
    Volatile
    // add the rest if you want. i couldnt care less
}

struct ArrayType<'a> {
    name: &'a str,
    vtype: Type,
    size: BitByteSize
}

struct StructType<'a> {
    name: Option<&'a str>,
    size: BitByteSize,
    members: Vec<Member<'a>>
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

struct Enumerator<'a> {
    name: &'a str,
    constant: u64
}

enum BitByteSize {
    Bit(u64),
    Byte(u64),
}

struct TypeDef<'a> {
    name: &'a str,
    vtype: Option<Type>
}

fn unwind_type<'a>(debug_info_offset: Type, dwarf: &'a Dwarf) -> TypeDisplay<'a> {
    let type_entry = get_unit_entry_offset(debug_info_offset, dwarf);
    let unit = dwarf.unit(dwarf.unit_header(type_entry.1).unwrap()).unwrap();
    let entry = unit.entry(type_entry.0).unwrap();

    let name = match entry.attr_value(gimli::DW_AT_name) {
        Some(value) => Some(value.string_value(&dwarf.debug_str).unwrap().to_string().unwrap()),
        None => None
    };

    match entry.tag() {
        gimli::DW_TAG_base_type => {
            let encoding = gimli::DwAte(entry.attr_value(gimli::DW_AT_encoding).unwrap().u8_value().unwrap());
            let size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Some(value) => BitByteSize::Byte(value.udata_value().unwrap()),
                None => BitByteSize::Bit(entry.attr_value(gimli::DW_AT_bit_size).unwrap().udata_value().unwrap())
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
            let vtype = DebugInfoOffset(entry.attr_value(gimli::DW_AT_type).unwrap().offset_value().unwrap());
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
            let vtype = DebugInfoOffset(entry.attr_value(gimli::DW_AT_type).unwrap().offset_value().unwrap());
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
            let vtype = DebugInfoOffset(entry.attr_value(gimli::DW_AT_type).unwrap().offset_value().unwrap());
            let size = match entry.attr_value(gimli::DW_AT_byte_size) {
                Some(value) => BitByteSize::Byte(value.udata_value().unwrap()),
                None => BitByteSize::Bit(entry.attr_value(gimli::DW_AT_bit_size).unwrap().udata_value().unwrap())
            };
            TypeDisplay::Array(ArrayType {
                name: name.unwrap(),
                vtype,
                size
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
            entries.next_entry().unwrap(); // Stepping into the children
            loop {
                let current = match entries.next_sibling().unwrap() { // iter only over the siblings
                    Some(entry) => entry,
                    None => break
                };

                if current.tag() != gimli::DW_TAG_member {
                    continue;
                }
                let name = entry.attr_value(gimli::DW_AT_name).unwrap().string_value(&dwarf.debug_str).unwrap().to_string().unwrap();
                let vtype = DebugInfoOffset(entry.attr_value(gimli::DW_AT_type).unwrap().offset_value().unwrap());
                let offset = entry.attr_value(gimli::DW_AT_data_member_location).unwrap().udata_value().unwrap();
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
                Some(value) => Some(BitByteSize::Byte(value.udata_value().unwrap())),
                None => match entry.attr_value(gimli::DW_AT_bit_size) {
                    Some(value) => Some(BitByteSize::Bit(value.udata_value().unwrap())),
                    None => None
            }};
            let vtype = match entry.attr_value(gimli::DW_AT_type) {
                Some(value) => Some(DebugInfoOffset(value.udata_value().unwrap() as usize)),
                None => None
            };
            let mut enumerators: Vec<Enumerator> = Vec::new();
            let mut entries = unit.entries_at_offset(type_entry.0).unwrap();
            entries.next_entry().unwrap(); // Stepping into the children
            loop {
                let current = match entries.next_sibling().unwrap() { // iter only over the siblings
                    Some(entry) => entry,
                    None => break
                };

                if current.tag() != gimli::DW_TAG_member {
                    continue;
                }
                let name = entry.attr_value(gimli::DW_AT_name).unwrap().string_value(&dwarf.debug_str).unwrap().to_string().unwrap();
                let constant = entry.attr_value(gimli::DW_AT_const_value).unwrap().udata_value().unwrap();
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
                Some(value) => Some(DebugInfoOffset(value.udata_value().unwrap() as usize)),
                None => None
            };
            TypeDisplay::Def(TypeDef{
                name: name.unwrap(),
                vtype
            })
        },
        _ => panic!()
    };
    todo!()
}

// Assembly

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};

const AREA: usize = 2048; // how many bytes around the address to parse
const BACKSCAN: u64 = 64;
const BYTE_COLUMN: usize = 10;

pub struct Assembly {
    pointer: u64,
    text: String,
    addresses: Vec<u64>
}

fn align_pointer(address: u64, bytes: &[u8]) -> Result<u64, ()> {
    'outer: for offset in 0..BACKSCAN {
        let mut decoder = Decoder::with_ip(64, &bytes, address + offset, DecoderOptions::NONE);
        let mut instruction = Instruction::default();
        while decoder.can_decode() {
            decoder.decode_out(&mut instruction);
            if instruction.is_invalid() {
                continue 'outer;
            }
        }
        return Ok(address + offset);
    };
    Err(()) //HOW UNLUCKY WHAT
}

fn disassemble_code(address: u64) -> Result<Assembly, ()> {
    let pointer = address-(AREA/2) as u64;
    let bytes = trace::read_memory(pointer, AREA)?;
    let valid_pointer = align_pointer(pointer, &bytes)?;

    let mut decoder = Decoder::with_ip(64, &bytes, valid_pointer, DecoderOptions::NONE);
    let mut formatter = NasmFormatter::new();

    formatter.options_mut().set_digit_separator("_");
    formatter.options_mut().set_first_operand_char_index(10);

    let mut result = String::new();
    let mut addresses: Vec<u64> = Vec::new();

    let mut output = String::new();
    let mut instruction = Instruction::default();

    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        output.clear();
        formatter.format(&instruction, &mut output);

        addresses.push(instruction.ip());

        result.push_str(&format!("0x{:016x}    ", instruction.ip()));
        result.push_str(&output);
        result.push('\n');
    };

    Ok(Assembly {
        pointer: valid_pointer,
        text: result,
        addresses
    })

}



// HELPER

fn normal(rip: u64) -> u64 {
    match INTERNAL.access().dynamic_exec_shift {
        Some(offset) => rip - offset,
        None => rip
    }
}

fn get_unit_entry_offset(offset: DebugInfoOffset, dwarf: &Dwarf) -> (gimli::UnitOffset, DebugInfoOffset) { //unit offset AND the offset of the Unit
    let mut units = dwarf.units();
    while let Some(unit) = units.next().unwrap() {
        match offset.to_unit_offset(&unit) {
            Some(offset) => return (offset, unit.debug_info_offset().unwrap()),
            _ => ()
        };
    }
    panic!(); // how did you get this far yk

}