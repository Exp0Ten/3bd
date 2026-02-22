use std::path::PathBuf;
use std::collections::HashMap;


use gimli::DebugInfoOffset;
use gimli::UnitHeader;
use gimli::EndianSlice;
use gimli::UnwindSection;

use object::{Object, ObjectSection};

use crate::data::*;
use crate::trace;


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

pub struct FunctionIndex<Unit = DebugInfoOffset, FunctionOffset = DebugInfoOffset> {
    func_hash: HashMap<u64, FunctionOffset>,
    range_hash: HashMap<Unit, Vec<FunctionRange>>
}

impl FunctionIndex {
    fn new() -> Self {
        FunctionIndex {
            func_hash: HashMap::new(),
            range_hash: HashMap::new()
        }
    }

    fn insert_function(&mut self, range: FunctionRange, unit: DebugInfoOffset) {
        let mut range_hash = &mut self.range_hash;
        if range_hash.contains_key(&unit) {
            range_hash.get_mut(&unit).unwrap().push(range);
        } else {
            range_hash.insert(unit, vec![range]);
        }
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

fn parse_functions(dwarf: Dwarf) {
    let mut function_index = FunctionIndex::new();

    let mut unit_headers = dwarf.units();

    while let Some(unit_header) = unit_headers.next().unwrap() {
        let unit = dwarf.unit(unit_header).unwrap();
        // let pointer_size = unit.address_size();
        let base_address = unit.low_pc; // for ranges attr

        let mut entries = unit.entries();

        loop {
            let entry = entries.current().unwrap();

            if entry.tag() == gimli::DW_TAG_subprogram {
                let attributes = entry.attrs();

                let mut tup: (u64, u64) = (0, 0);
                let mut ranges = None;

                for attribute in attributes {
                    let name = attribute.name();
                    if name == gimli::DW_AT_low_pc {
                        tup.0 = attribute.udata_value().unwrap();
                        continue;
                    }
                    if name == gimli::DW_AT_high_pc {
                        tup.1 = attribute.udata_value().unwrap();
                        continue;
                    }
                    if name == gimli::DW_AT_ranges {
                        ranges = attribute.udata_value();

                        break;
                    }
                };

                if let Some(range_offset) = ranges {
                    let mut ranges = dwarf.ranges.ranges(
                        gimli::RangeListsOffset(range_offset as usize),
                        unit.encoding(),
                        base_address,
                        &dwarf.debug_addr,
                        unit.addr_base
                    ).expect("WTF RANGES");

                    while let Some(range) = ranges.next().unwrap_or(None) {
                        function_index.insert_function(range.begin..range.end, unit.debug_info_offset().unwrap());
                    }

                    continue;
                }

                if tup == (0, 0) {continue;}

                let function_range = tup.0..tup.0+tup.1;

                function_index.insert_function(function_range, unit.debug_info_offset().unwrap());
            }

            match entries.next_entry() { // TODO, consider if next entry or next sibling
                Ok(next) => if !next {break;},
                Err(_) => break
            }
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



fn get_cfa(unwind: &UnwindInfo, rsp: u64, rbp: u64, rip: u64, eh_frame: &EhFrame, encoding: gimli::Encoding) -> u64 {
    let cfa = unwind.row.as_ref().unwrap().cfa();
    match cfa {
        gimli::CfaRule::RegisterAndOffset {
            register,
            offset
        } => (get_register_value(*register, rsp, rbp, rip) as i64 + offset) as u64,
        gimli::CfaRule::Expression(expression) => {
            eval_expression(&expression.get(&eh_frame.frame).unwrap(), 0, rsp, rbp, rip, encoding, None).to_u64(0).unwrap() //cfa is none actually, but i know the cfa will not mention itself, otherwise its corrupted
        },
    }
}

fn get_return_address(unwind: &UnwindInfo, cfa: u64, rsp: u64, rbp: u64, rip: u64, eh_frame: &EhFrame,encoding: gimli::Encoding) -> u64 {
    let return_address = unwind.row.as_ref().unwrap().register(gimli::X86_64::RA).unwrap();
    unwind_register(return_address, cfa, rsp, rbp, rip, eh_frame, encoding).unwrap_or(rip)
}

fn get_previous_rsp(unwind: &UnwindInfo, cfa: u64, rsp: u64, rbp: u64, rip: u64, eh_frame: &EhFrame, encoding: gimli::Encoding) -> u64 {
    let previous_rsp = unwind.row.as_ref().unwrap().register(gimli::X86_64::RSP).unwrap();
    unwind_register(previous_rsp, cfa, rsp, rbp, rip, eh_frame, encoding).unwrap_or(rsp)
}

fn get_previous_rbp(unwind: &UnwindInfo, cfa: u64, rsp: u64, rbp: u64, rip: u64, eh_frame: &EhFrame, encoding: gimli::Encoding) -> u64 {
    let previous_rsp = unwind.row.as_ref().unwrap().register(gimli::X86_64::RSP).unwrap();
    unwind_register(previous_rsp, cfa, rsp, rbp, rip, eh_frame, encoding).unwrap_or(rbp)
}

fn unwind_register(register_rule: gimli::RegisterRule<usize>, cfa: u64, rsp: u64, rbp: u64, rip: u64, eh_frame: &EhFrame, encoding: gimli::Encoding) -> Option<u64> {
    match register_rule {
        gimli::RegisterRule::Offset(addr_offset) => Some(slice_to_u64(&unwind_memory((cfa as i64 + addr_offset) as u64, 8))),
        gimli::RegisterRule::Expression(expression) => {Some(slice_to_u64(
            &unwind_memory(eval_expression(&expression.get(&eh_frame.frame).unwrap(), cfa, rsp, rbp, rip, encoding, None).to_u64(0).unwrap(), 8)
        ))},
        gimli::RegisterRule::ValOffset(offset) => Some((cfa as i64 + offset) as u64),
        gimli::RegisterRule::ValExpression(expression) => {Some(
            eval_expression(&expression.get(&eh_frame.frame).unwrap(), cfa, rsp, rbp, rip, encoding, None).to_u64(0).unwrap()
        )},
        gimli::RegisterRule::Register(register) => Some(get_register_value(register, rsp, rbp, rip)),
        gimli::RegisterRule::Constant(value) => Some(value),
        gimli::RegisterRule::SameValue => None,
        _ => unimplemented!() // NOW THIS IS NOT IMPLEMENTED SO LETS HOPE IT WONT BE NEEDED YK :P
    }
}

fn get_register_value(register: gimli::Register, rsp: u64, rbp: u64, rip: u64) -> u64 { // Only these registers are used in expression and locations, if not im gonna cry
    match register {
        gimli::X86_64::RSP => rsp,
        gimli::X86_64::RA => rip,
        gimli::X86_64::RBP => rbp,
        _ => panic!("WHAT REG DO YOU WANT??? {register:?}")
    }
}

fn unwind_memory(address: u64, size: u8) -> Vec<u8> { // for reading small amounts of data from the memory (a wrapper)
    trace::read_memory(address, size as usize).expect("CORRUPTED DWARF OR IDK WTFFFF")
}

fn eval_expression(expression: &gimli::Expression<EndianSlice<'_, Endian>>, cfa: u64, rsp: u64, rbp: u64, rip: u64, encoding: gimli::Encoding, frame_base: Option<u64>) -> gimli::Value {
    let mut evaluation = expression.evaluation(encoding);

    let mut result = evaluation.evaluate().unwrap();
    loop {
        match result {
            gimli::EvaluationResult::Complete => break,
            gimli::EvaluationResult::RequiresMemory { address, size, space, base_type } => { // TODO if the size needs implementation
                let data = unwind_memory(address, size);
                result = evaluation.resume_with_memory(gimli::Value::U64(slice_to_u64(&data))).expect("Something failed");
            },
            gimli::EvaluationResult::RequiresRegister { register, base_type } => { // TODO if the type needs implementation
                result = evaluation.resume_with_register(gimli::Value::U64(get_register_value(register, rsp, rbp, rip))).unwrap();
            },
            gimli::EvaluationResult::RequiresFrameBase => {
                result = evaluation.resume_with_frame_base(frame_base.unwrap()).unwrap();
            },
            gimli::EvaluationResult::RequiresCallFrameCfa => {
                result = evaluation.resume_with_call_frame_cfa(cfa).unwrap();
            }
            _ => unimplemented!("DAMN WHAT")
        }
    }
    let result = evaluation.value_result().expect("WHAT IS IT THIS TIME");
    result
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

impl CallStack {
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
}

struct Variable {
    name: String,
    location: u64,
    vtype: Type
}

struct Parameter {
    name: String,
    location: u64,
    vtype: Type
}

fn call_stack(pid: nix::unistd::Pid) -> Result<CallStack, ()> {
    let mut call_stack = CallStack::new();

    let binding = INTERNAL.access();
    let registers = binding.registers.unwrap();
    let (rip, rsp, rbp) = (registers.rip, registers.rsp, registers.rbp);

    unwind(&mut call_stack, &binding, rip, rsp, rbp)?;

    Ok(call_stack)
}

fn unwind (
    call_stack: &mut CallStack,
    binding: &std::sync::MutexGuard<'_, Internal>,
    rip: u64,
    rsp: u64,
    rbp: u64
) -> Result<(), ()> {
    // parsed global DWARF INFO // make wrapper functions for these data
    let eh_frame = binding.eh_frame.as_ref().unwrap();
    let line_addresses = binding.line_addresses.as_ref().unwrap();
    let source_map = binding.source_files.as_ref().unwrap();
    let function_index = binding.function_index.as_ref().unwrap();
    let dwarf = binding.dwarf.as_ref().unwrap().dwarf(eh_frame.endian);

    // We need info about the function and all
    let index = line_addresses.get(&rip).ok_or(())?; // if we arent in a source file, we cannot be debugging the info (can happen using step while stepping into a dynamic library function, and therefore it will not show any of the call stack info, as we have no Dwarf info)
    let unit = source_map.get(&index.hash_path).unwrap()[index.index].compile_unit; // our function index is unit organised for faster lookup speed. this is possible thanks to the hashmap, which has constant access time, while searching through ranges is linear access time (in the end, this is MUCH faster, especially with more functions and source files)
    let function: DebugInfoOffset = function_index.get_function(rip, unit).unwrap(); // now we get our function offset into the debug_info_section
    let unit_header = dwarf.debug_info.header_from_offset(unit).unwrap();
    let dwarf_unit = dwarf.unit(unit_header).unwrap();

    let offset = function.to_unit_offset(&dwarf_unit).unwrap();
    let entries = dwarf_unit.entries_at_offset(offset).unwrap();
    let die = entries.current().unwrap();
    let (mut function_info, frame_attribute) = extract_function_info(die, &dwarf);

    let unwind_info = get_unwind_for_address(rip, eh_frame);
    let encoding = dwarf_unit.encoding();
    let cfa = get_cfa(&unwind_info, rsp, rbp, rip, eh_frame, encoding);

    let return_address = get_return_address(&unwind_info, cfa, rsp, rbp, rip, eh_frame, encoding);
    let previous_rsp= get_previous_rsp(&unwind_info, cfa, rsp, rbp, rip, eh_frame, encoding);
    let previous_rbp = get_previous_rbp(&unwind_info, cfa, rsp, rbp, rip, eh_frame, encoding);

    if frame_attribute.is_some() {
        let frame_base = eval_expression(&frame_attribute.unwrap().exprloc_value().unwrap(), cfa, rsp, rbp, rip, encoding, None);
        let current_line = index.line;
        extract_variables(&mut function_info, frame_base.to_u64(0).unwrap(), entries, &dwarf, encoding, current_line);
    }

    call_stack.0.push(function_info);

    // we want to free up as much memory as possible before recursing the function again // maybee???
    drop(dwarf);

    unwind(call_stack, binding, return_address, previous_rsp, previous_rbp)?;

    Ok(())
}

fn extract_function_info<'a>(die: &gimli::DebuggingInformationEntry<EndianSlice<'a, Endian>, usize>, dwarf: &'a Dwarf) -> (Function, Option<gimli::AttributeValue<EndianSlice<'a, Endian>>>) {
    let mut name = None;
    let mut return_type = None;
    let mut frame_base = None;

    let attributes = die.attrs();
    for attribute in attributes {
        match attribute.name() {
            gimli::DW_AT_name => {
                name = Some(attribute.string_value(&dwarf.debug_str).unwrap().to_string().unwrap().to_string());
            },
            gimli::DW_AT_frame_base => {
                frame_base = Some(attribute.value());
            },
            gimli::DW_AT_type => {
                return_type = Some(Type {0: attribute.offset_value().unwrap()});
            },
            _ => ()
        }
    };

    (Function {
        name: name.unwrap(),
        parameters: None,
        variables: None,
        return_type: return_type
    }, frame_base)
}

fn extract_variables<'a>(function: &mut Function, frame_base: u64, mut entries: gimli::EntriesCursor<'_, EndianSlice<'a, Endian>>, dwarf: &'a Dwarf, encoding: gimli::Encoding, current_line: u64) {
    let fn_depth = entries.depth();
    loop {
        entries.next_entry().unwrap();
        if fn_depth == entries.depth() { // if the new entry has the same depth as the original fn_depth, then they are siblings and therefore we ran into the end of the function locals definition
            return;
        }

        let entry = match entries.current() {
            Some(entry) => entry,
            None => continue
        };

        if entry.tag() == gimli::DW_TAG_variable {
            let declaration = entry.attr(gimli::DW_AT_decl_line).unwrap().udata_value().unwrap();

            if declaration > current_line {continue;}; // if you have a variable you havent declared yet, you dont want to show it right, cause its gonna be random gibberish yk

            let name = String::from(entry.attr(gimli::DW_AT_name).unwrap().string_value(&dwarf.debug_str).unwrap().to_string().unwrap());
            let vtype = Type{0: entry.attr(gimli::DW_AT_type).unwrap().offset_value().unwrap()};
            let location = eval_expression(&entry.attr(gimli::DW_AT_location).unwrap().exprloc_value().unwrap(), 0, 0, 0, 0, encoding, Some(frame_base)).to_u64(0).unwrap();

            let var = Variable {
                name,
                location,
                vtype,
            };

            function.variables.as_mut().unwrap().push(var);
        }

        if entry.tag() == gimli::DW_TAG_formal_parameter {
            let name = String::from(entry.attr(gimli::DW_AT_name).unwrap().string_value(&dwarf.debug_str).unwrap().to_string().unwrap());
            let vtype = Type{0: entry.attr(gimli::DW_AT_type).unwrap().offset_value().unwrap()};
            let location = eval_expression(&entry.attr(gimli::DW_AT_location).unwrap().exprloc_value().unwrap(), 0, 0, 0, 0, encoding, Some(frame_base)).to_u64(0).unwrap();

            let param = Parameter {
                name,
                location,
                vtype,
            };

            function.parameters.as_mut().unwrap().push(param);
        }
    }
}