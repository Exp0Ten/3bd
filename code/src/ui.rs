use iced::widget::{container, pane_grid, row, text};
use iced::widget::{Container,};


use crate::window::*;
use crate::data::*;

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

pub fn titlebar(app: &App) -> Option<Container<'_, Message>> {



    Some(container(row![
        icon,
        title,
        winbuttons
    ]).into())
}