

use crate::data::INTERNAL;

#[derive(Debug, Clone)]
pub enum Operation {
    NewWindow,
    LoadFile,
    ReloadFile,
    SaveLog,
    OpenSettings
    //fill as needed
}

pub fn operation_message(operation: Operation) {

}