use std::path::PathBuf;
use std::collections::HashMap;

pub struct SourceVec {
    pub compile_dir: PathBuf,
    pub files: Vec<SourceFile>
}

#[derive(PartialEq)]
pub struct SourceFile {
    pub name: String,
    pub path: PathBuf,
    //compile_unit: String,
    pub content: Vec<(String, usize)>
}

pub type LineAddresses<'a> = HashMap<u64, (usize, &'a SourceFile)>;

pub trait ImplLineAddresses<'a> {
    fn get_line(&'a self, address: u64) -> Option<&'a (usize, &'a SourceFile)>;
    fn get_address(&'a self, line: &(usize, &'a SourceFile)) -> Option<u64>;
}

impl <'a> ImplLineAddresses<'a> for LineAddresses<'_> {
    fn get_line(&'a self, address: u64) -> Option<&'a (usize, &'a SourceFile)> {
        self.get(&address)
    }

    fn get_address(&'a self, line: &(usize, &'a SourceFile)) -> Option<u64> {
        let keys = self.keys();
        for key in keys {
            let entry = self[key];
            if entry == *line {
                return Some(*key);
            }
        };
        None
    }
}