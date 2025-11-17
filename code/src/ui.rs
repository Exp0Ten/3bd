use iced::{
    Task, Length,
    widget::{
        Container, MouseArea, Row, Text, Theme,
        button, column, container, mouse_area, pane_grid, row, svg, text,
        svg::{Handle, Svg}
    }
};

use crate::{
    window::*, data::*, trace::*
};


pub struct Layout {
    status_bar: bool,
    sidebar_left: bool,
    sidebar_right: bool,
    panel: bool,
//    panes: pane_grid::State<Pane>
}

impl Default for Layout {
    fn default() -> Self {
        Layout { status_bar: true, sidebar_left: true, sidebar_right: true, panel: true}
    }
}

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
    SidebarLeftToggle,
    SidebarRightToggle,
    PanelToggle
}

pub fn content(state: &State) -> Container<'_, Message> {
    container(column(
        if state.layout.status_bar {vec![
            toolbar(state, 50).into(),
            main_frame(state).into(),
            statusbar(state, 20).into()
        ]} else {vec![
            toolbar(state, 50).into(),
            main_frame(state).into()
        ]}
    ))
}



fn toolbar<'a>(state: &State, height: usize) -> Container<'a, Message> {

    fn toolbar_button<'a>(icon: &str, size: f32) -> button::Button<'a, Message> {
        button(svg(Handle::from_memory(Asset::get(icon).unwrap().data)).height(Length::Fill)).padding(4).height(size as f32).width(size as f32)

    }

    fn buttons<'a>(state: &State, size: f32) -> [button::Button<'a, Message>; 4] {
        let load_file = toolbar_button("icons/load_file.svg", size).on_press(Message::Operation(Operation::LoadFile));
        let sidebar_left = toolbar_button("icons/sidebar_left.svg", size).on_press(Message::None);
        let sidebar_right = toolbar_button("icons/sidebar_right.svg", size).on_press(Message::None);
        let panel = toolbar_button("icons/panel.svg", size).on_press(Message::None);
        [load_file, sidebar_left, sidebar_right, panel] //extend
    }

    let padding = 10.;

    let (
        load_file,
        sidebar_left,
        sidebar_right,
        panel
    ) = buttons(state, (height as f32)-padding*2.).into();

    let left_buttons: Row<'_, Message> = row![
        load_file
    ].spacing(padding);

    let right_buttons: Row<'_, Message> = row![
        sidebar_left,
        sidebar_right,
        panel
    ].spacing(padding);

    container(row([
        left_buttons.into(),
        widget_fill().into(),
        right_buttons.into()
    ])).height(Length::Fixed(height as f32))
    .width(Length::Fill)
    .padding(padding) //padding around the buttons
}

fn statusbar<'a>(state: &State, height: usize) -> Container<'a, Message> {
    container(row([

    ])).height(Length::Fixed(height as f32))
    .width(Length::Fill)
    .padding(3)
}

fn main_frame<'a>(state: &State) -> Container<'a, Message> {
    container(
        ""
    ).width(Length::Fill)
    .height(Length::Fill)
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


// Misc widgets

fn widget_fill<'a>() -> Container<'a, Message> {
    container("").width(Length::Fill).height(Length::Fill)
}

fn delimiter<'a>(width: usize) -> Container<'a, Message> {
    container(text("|")).width(Length::Fixed(width as f32)).height(Length::Fill)
}

fn toggle_button_wrapper<'a>(state: &State, ) -> button::Button<'a, Message> {
    todo!()
}