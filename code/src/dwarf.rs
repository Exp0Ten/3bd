use std::path::PathBuf;
use std::collections::HashMap;

struct SourceFile {
    name: String,
    pathbuf: PathBuf,
    //compile_unit: String,
    content: Vec<(String, u64)>
}

pub struct LineAddresses {
    pub dict: HashMap<u64, (u64, PathBuf)>
    // This is just an idea:
    // dict: HashMap<u64, (u64, &'a SourceFile)>
    // we would keep a pointer to the file and the line, so we could access the files easier and also for the sake of compactibility, but id have to setup the lifetime of it
}