use std::path::PathBuf;
use std::collections::HashMap;

pub struct SourceVec {
    pub compile_dir: PathBuf,
    pub files: Vec<SourceFile>
}

pub struct SourceFile {
    pub name: String,
    pub path: PathBuf,
    //compile_unit: String,
    pub content: Vec<(String, usize)>
}

pub struct LineAddresses <'a> {
    // pub dict: HashMap<u64, (u64, PathBuf)>
    pub dict: HashMap<u64, (usize, &'a SourceFile)>
    // we would keep a pointer to the file and the line, so we could access the files easier and also for the sake of compactibility, but id have to setup the lifetime of it
}