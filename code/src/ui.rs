use iced::{
    Task,
    widget::{
        Container, MouseArea, Row, Text, Theme,
        button, column, container, mouse_area, pane_grid, row, svg, text,
        svg::{Handle, Svg}
    }
};

use crate::{
    window::*, data::*
};

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
    //fill in later
}

pub fn content(state: &State) -> Container<'_, Message> {
    container(
        ""
    )
}

pub fn pane_message(state: &mut State, pane: PaneMessage) -> Task<Message> {
    let task: Option<Task<Message>> = match pane {
        //fill in later
        _ => None
    };

    match task {
        Some(task) => task,
        None => Task::none()
    }
}
