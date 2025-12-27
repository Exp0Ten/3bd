use iced::{
    Task, Length,
    widget::{
        Container, MouseArea, Row, Text, Theme,
        button, column, container, mouse_area, pane_grid, row, svg, text,
        svg::{Handle, Svg}
    }
};

use crate::{
    window::*, data::*, trace::*, style
};



pub struct Layout {
    status_bar: bool,
    sidebar_left: bool,
    sidebar_right: bool,
    panel: bool,
    //panes: pane_grid::State<Pane>
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            status_bar: true,
            sidebar_left: false,
            sidebar_right: false,
            panel: false,
            //panes: pane_grid::State::with_configuration()
        }
    }
}

pub enum Pane { // Generic enum for all bars (completed widgets that can be moved around inside a window) (they will have their own structs if they need)
    Memory,
    Stack,
    Code,
    Assembly,
    Registers,
    Labels,
    Info, // ELF dump
    Control,
    //Terminal // maybe extrenal? well see
}

#[derive(Debug, Clone)]
pub enum PaneMessage {
    SidebarLeftToggle,
    SidebarRightToggle,
    PanelToggle
}

pub fn content(state: &State) -> Container<'_, Message> {
    container(column(
        //if state.layout.status_bar {vec![
        if true {vec![
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

    fn toolbar_button<'a>(icon: &str, size: f32, svg_style: Option<fn(&Theme, svg::Status) -> svg::Style>) -> button::Button<'a, Message> {
        button(
            svg(Handle::from_memory(Asset::get(icon).unwrap().data))
            .height(Length::Fill)
            .style(svg_style.unwrap_or(style::bar_svg))
        ).padding(4)
        .height(size as f32)
        .width(size as f32)
    }

    fn buttons<'a>(state: &State, size: f32) -> [button::Button<'a, Message>; 4] {
        let load_file = toolbar_button("icons/load_file.svg", size, None).on_press(Message::Operation(Operation::LoadFile)).style(style::bar_button);

        // Toggle buttons:
        let sidebar_left = toolbar_button("icons/sidebar_left.svg", size,
        Some(if state.layout.sidebar_left {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.sidebar_left {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Pane(PaneMessage::SidebarLeftToggle));

        let sidebar_right = toolbar_button("icons/sidebar_right.svg", size,
        Some(if state.layout.sidebar_right {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.sidebar_right {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Pane(PaneMessage::SidebarRightToggle));
        let panel = toolbar_button("icons/panel.svg", size,
        Some(if state.layout.panel {style::bar_svg_toggled} else {style::bar_svg})
        ).style(if state.layout.panel {style::bar_button_toggled} else {style::bar_button})
        .on_press(Message::Pane(PaneMessage::PanelToggle));

        [load_file, sidebar_left, sidebar_right, panel] //extend
    }

    let padding = 5.;

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
    .style(style::bar)
}


fn statusbar<'a>(state: &State, height: usize) -> Container<'a, Message> {

    container(row![
        text("Program State | Program Position | Backtrace ...").size((height-7) as f32)
    ]).height(Length::Fixed(height as f32))
    .width(Length::Fill)
    .padding(3)
    .style(style::bar)
}



fn main_frame<'a>(state: &State) -> Container<'a, Message> {
    container(
        ""
    ).center(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
}








pub fn pane_message(state: &mut State, pane: PaneMessage) -> Task<Message> {
    let task: Option<Task<Message>> = match pane {
        PaneMessage::SidebarLeftToggle => {state.layout.sidebar_left ^= true; None}
        PaneMessage::SidebarRightToggle => {state.layout.sidebar_right ^= true; None}
        PaneMessage::PanelToggle => {state.layout.panel ^= true; None}
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