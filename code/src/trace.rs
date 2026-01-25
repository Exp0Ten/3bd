

use crate::data::INTERNAL;

use crate::window::Dialog;

#[derive(Debug, Clone)]
pub enum Operation {
    LoadFile,
    ReloadFile
    //fill as needed
}

pub fn operation_message(operation: Operation) {
    match operation {
        Operation::LoadFile => {Dialog::file(None, None);},
        _ => ()
    }
}