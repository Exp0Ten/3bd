use iced::{
    Length, Task,
    widget::{
        Container, Row, Text, Theme,
        button, container, pane_grid, row, column, text, svg, mouse_area,
        svg::{Handle, Svg}
    },
    mouse::Interaction
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



pub fn titlebar(app: &App) -> Container<'_, Message> {
    container(row![
        icon(),
        title(),
        winbuttons(&app.state)
    ])
    //needs styling
    .into()
}

fn icon<'a>() -> Svg<'a, Theme> {
    Svg::new(Handle::from_memory(
        Asset::get("icons/TBD.svg").unwrap().data)
    )
    //needs styling
}

fn title<'a>() -> Text<'a, Theme> {
    text({
        match &INTERNAL.access().file {
            Some(path) =>  format!("{} - Three Body Debugger", path.file_name().unwrap().to_str().unwrap()),
            None => "Three Body Debugger".to_string()
        }
    })
    //needs styling
}

fn winbuttons<'a>(state: &State) -> Row<'a, Message> {
    let minbutton = button(
        svg(Handle::from_memory(
            Asset::get("icons/window_minimize.svg").unwrap().data
        ))
        //needs styling
    ).on_press(Message::Window(WinMessage::Minimize));

    let maxbutton = button(
        svg(Handle::from_memory(
            Asset::get(
                if state.maximized {"icons/window_restore.svg"} else {"icons/window_maximize.svg"}
            ).unwrap().data
        ))
        //needs styling
    ).on_press(if state.maximized {Message::Window(WinMessage::Restore)} else {Message::Window(WinMessage::Maximize)});

    let xbutton = button(
        svg(Handle::from_memory(
            Asset::get("icons/window_close.svg").unwrap().data
        ))
        //needs styling
    ).on_press(Message::Window(WinMessage::Close));


    row![
        minbutton,
        maxbutton,
        xbutton
    ]
    //needs styling
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

pub fn resize_area<'a>(width: u8) -> Container<'a, Message> {
    container(
        row![
            column![
                mouse_area(container(" ").height(Length::Fixed(width.into())).width(Length::Fixed(width.into())))
                .on_press(Message::Window(WinMessage::Resize(Direction::TopLeft)))
                .interaction(Interaction::ResizingDiagonallyDown),
                mouse_area(container(" ").center_y(Length::Fill))
                .on_press(Message::Window(WinMessage::Resize(Direction::Left)))
                .interaction(Interaction::ResizingHorizontally),
                mouse_area(container(" ").height(Length::Fixed(width.into())).width(Length::Fixed(width.into())))
                .on_press(Message::Window(WinMessage::Resize(Direction::BottomLeft)))
                .interaction(Interaction::ResizingDiagonallyUp)
                ]
            .width(Length::Fixed(width.into())),
            column![
                mouse_area(container(" ").center_x(Length::Fill).height(Length::Fixed(width.into())))
                .on_press(Message::Window(WinMessage::Resize(Direction::Top)))
                .interaction(Interaction::ResizingVertically),
                container(" ").center(Length::Fill),
                mouse_area(container(" ").center_x(Length::Fill).height(Length::Fixed(width.into())))
                .on_press(Message::Window(WinMessage::Resize(Direction::Bottom)))
                .interaction(Interaction::ResizingVertically)
            ]
            .width(Length::Fill),
            column![
                mouse_area(container(" ").height(Length::Fixed(width.into())).width(Length::Fixed(width.into())))
                .on_press(Message::Window(WinMessage::Resize(Direction::TopRight)))
                .interaction(Interaction::ResizingDiagonallyUp),
                mouse_area(container(" ").center_y(Length::Fill))
                .on_press(Message::Window(WinMessage::Resize(Direction::Right)))
                .interaction(Interaction::ResizingHorizontally),
                mouse_area(container(" ").height(Length::Fixed(width.into())).width(Length::Fixed(width.into())))
                .on_press(Message::Window(WinMessage::Resize(Direction::BottomRight)))
                .interaction(Interaction::ResizingDiagonallyDown)
            ]
            .width(Length::Fixed(width.into()))
        ]
        .height(Length::Fill)
        .width(Length::Fill)
    )
}