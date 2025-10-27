
pub enum Bar { // Generic enum for all bars (completed widgets that can be moved around inside a window)
    Memory,
    Stack,
    Code,
    Assembly,
    Status,
    Registers,
    Labels,
    Info, // ELF dump
    Menu,
    Control,
    Terminal // maybe extrenal? well see
}