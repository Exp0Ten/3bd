use iced::widget::pane_grid;

pub enum Pane { // Generic enum for all bars (completed widgets that can be moved around inside a window)
    Memory,
    Stack,
    Code,
    Assembly,
    Registers,
    Labels,
    Info, // ELF dump
    Control,
    Terminal // maybe extrenal? well see
}

#[derive(Debug, Clone)]
pub enum PaneMessage {
    Memory(MemoryPaneMessage),
    Stack(StackPaneMessage),
    Code(CodePaneMessage),
    Assembly(AssemblyPaneMessage),
    Registers(RegistersPaneMessage),
    Labels(LabelsPaneMessage),
    Info(InfoPaneMessage), // ELF dump
    Control(ControlPaneMessage),
    Terminal(TerminalPaneMessage), // maybe extrenal? well see
}

#[derive(Debug, Clone)]
enum MemoryPaneMessage {

}

#[derive(Debug, Clone)]
enum StackPaneMessage {
    
}

#[derive(Debug, Clone)]
enum CodePaneMessage {
    
}

#[derive(Debug, Clone)]
enum AssemblyPaneMessage {
    
}

#[derive(Debug, Clone)]
enum RegistersPaneMessage {
    
}

#[derive(Debug, Clone)]
enum LabelsPaneMessage {
    
}

#[derive(Debug, Clone)]
enum InfoPaneMessage {

}

#[derive(Debug, Clone)]
enum ControlPaneMessage {

}

#[derive(Debug, Clone)]
enum TerminalPaneMessage {

}