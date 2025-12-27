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
    panes: pane_grid::State<Pane>,
    _focus: Option<pane_grid::Pane>
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            status_bar: true,
            sidebar_left: false,
            sidebar_right: false,
            panel: false,
            panes: pane_grid::State::with_configuration(pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Horizontal,
                    ratio: 0.,
                    a: Box::new(pane_grid::Configuration::Pane(Pane::Registers(PaneRegisters {}))),
                    b: Box::new(pane_grid::Configuration::Pane(Pane::Registers(PaneRegisters {})))
                }), // implement later from toml
            //panes: pane_grid::State::new(Pane::Registers).0,
            _focus: None
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pane { // Generic enum for all bars (completed widgets that can be moved around inside a window) (they will have their own structs if they need)
    Memory(PaneMemory),
    Stack(PaneStack),
    Code(PaneCode),
    Assembly(PaneAssembly),
    Registers(PaneRegisters),
    Variables(PaneVariables),
    Info(PaneInfo), // ELF dump
    Control(PaneControl),
    Terminal(PaneTerminal)
}

#[derive(Debug, Clone)]
struct PaneMemory {}

#[derive(Debug, Clone)]
struct PaneStack {}

#[derive(Debug, Clone)]
struct PaneCode {}

#[derive(Debug, Clone)]
struct PaneAssembly {}

#[derive(Debug, Clone)]
struct PaneRegisters {}

#[derive(Debug, Clone)]
struct PaneVariables {}

#[derive(Debug, Clone)]
struct PaneInfo {}

#[derive(Debug, Clone)]
struct PaneControl {}

#[derive(Debug, Clone)]
struct PaneTerminal {}

#[derive(Debug, Clone)]
pub enum PaneMessage {
    SidebarLeftToggle,
    SidebarRightToggle,
    PanelToggle,
    _Focus(pane_grid::Pane),
    Drag(pane_grid::DragEvent),
    Resize(pane_grid::ResizeEvent),
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

fn main_frame<'a>(state: &'a State) -> Container<'a, Message> {
    container(
        pane_grid(&state.layout.panes, pane_view).spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_click(|pane| Message::Pane(PaneMessage::_Focus(pane)))
        .on_drag(|drag_event| Message::Pane(PaneMessage::Drag(drag_event)))
        .on_resize(10, |resize_event| Message::Pane(PaneMessage::Resize(resize_event)))
    ).center(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
}

fn pane_view(id: pane_grid::Pane, pane: &Pane, _maximized: bool) -> pane_grid::Content<'_, Message> {
    let (content, titlebar) = match pane {
        Pane::Code(state) => pane_view_code(state),
        Pane::Control(state) => pane_view_control(state),
        Pane::Memory(state) => pane_view_memory(state),
        Pane::Variables(state) => pane_view_variables(state),
        Pane::Stack(state) => pane_view_stack(state),
        Pane::Registers(state) => pane_view_registers(state),
        Pane::Assembly(state) => pane_view_assembly(state),
        Pane::Terminal(state) => pane_view_terminal(state),
        Pane::Info(state) => pane_view_info(state),

        _ => (container(text("Some other pane")), pane_grid::TitleBar::new(text("UNDEFINED")))
    };

    pane_grid::Content::new(content).title_bar(titlebar)
}

fn pane_titlebar(title: &str, height: usize) -> pane_grid::TitleBar<'_, Message> {
    pane_grid::TitleBar::new(
        container(column![text(title), widget_fill()]).center(Length::Fill)
        .height(Length::Fixed(height as f32))
        .width(Length::Fill)
        //TODO
    )
}

fn pane_view_code<'a>(state: &PaneCode) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Code", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_control<'a>(state: &PaneControl) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Control", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_memory<'a>(state: &PaneMemory) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Memory", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_variables<'a>(state: &PaneVariables) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Variables", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_stack<'a>(state: &PaneStack) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Stack", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_registers<'a>(state: &PaneRegisters) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Registers", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_assembly<'a>(state: &PaneAssembly) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Assembly", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_terminal<'a>(state: &PaneTerminal) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Terminal", 10);
    let content = todo!();
    (content, titlebar)
}

fn pane_view_info<'a>(state: &PaneInfo) -> (Container<'a, Message>, pane_grid::TitleBar<'a, Message>) {
    let titlebar = pane_titlebar("Info", 10);
    let content = todo!();
    (content, titlebar)
}




//message Handle

pub fn pane_message(state: &mut State, pane: PaneMessage) {
    match pane {
        PaneMessage::SidebarLeftToggle =>   state.layout.sidebar_left ^= true,
        PaneMessage::SidebarRightToggle =>  state.layout.sidebar_right ^= true,
        PaneMessage::PanelToggle =>         state.layout.panel ^= true,

        PaneMessage::_Focus(pane) =>   state.layout._focus = Some(pane),
        PaneMessage::Drag(pane_grid::DragEvent::Dropped {pane, target}) => {
            //let target = match target {
            //    pane_grid::Target::Pane(pane, _)
            //    _ => ()
            //}
            state.layout.panes.drop(pane, target)
        }
        PaneMessage::Resize(pane_grid::ResizeEvent {split, ratio}) => {
            state.layout.panes.resize(split, ratio)
        }
        //fill in later
        _ => ()
    };
}


// Misc widgets

fn widget_fill<'a>() -> Container<'a, Message> {
    container("").width(Length::Fill).height(Length::Fill)
}

fn delimiter<'a>(width: usize) -> Container<'a, Message> {
    container(text("|")).width(Length::Fixed(width as f32)).height(Length::Fill)
}