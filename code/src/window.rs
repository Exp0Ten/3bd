use iced::{
    Element, Task, Theme, Point,
    application, window,
    widget::{
        column, stack
    }
};

use crate::{
    ui::*, config, trace
};

#[derive(Default)]
pub struct App {
    pub state: State,
    pub theme: Theme,
    pub settings: window::Settings,      //while these settings dont affect the window settings on runtime, we still save them for conditional use with the UI, like decorations or creating a new window
    // add more as needed
}

pub struct State {
    //    panes: pane_grid::State<Pane>
    resizing: Option<Direction>,
    pub maximized: bool,
    // add more as needed
}

impl Default for State {
    fn default() -> Self {
        State {
            resizing: None,
            maximized: false
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Window(WinMessage),
    Pane(PaneMessage),
    Operation(trace::Operation),
    Other
    // add more as needed
}


#[derive(Debug, Clone)]
pub enum WinMessage {
    ResizeStart(Direction),
    ResizeMove(iced::Point),
    ResizeDone,
    Close,
    Maximize,
    Restore,
    Minimize
}


#[derive(Debug, Clone)]
pub enum Direction {
    TopLeft,
    Left,
    BottomLeft,
    Top,
    Bottom,
    TopRight,
    Right,
    BottomRight
}


pub fn run_app() -> iced::Result {
    application("Three Body Debugger", App::update, App::view)
    //.theme(App::theme)
    .theme(App::theme)
    .window(App::default().settings)
//    .subscription() //probably will be needed
    .run_with(|| (App::default(), Task::none())) // make a function to select between default config and user modified config
}

impl App {
    fn default() -> Self {
        //config::get_app().unwrap_or(Self::new())
        Self {
            state: State::default(),
            theme: Theme::Dark,
            settings: window::Settings {
                decorations: false,
                ..Default::default()
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let state = &mut self.state;

        let task: Task<Message> = match message {
            Message::Window(window) => window_message(self, window),
            Message::Operation(operation) => {trace::operation_message(operation); Task::none()},
            Message::Pane(pane) => pane_message(state, pane),
            Message::Other => Task::none(),
            _ => Task::none()
        };

        task
    }


    fn view(&self) -> Element<'_, Message> {
        let state = &self.state;

        let content = content(state);

        let result: Element<'_, Message> = if self.settings.decorations {
            content.into()
        } else {
            let titlebar = titlebar(&self);
            let display = column![titlebar, content];
            match state.maximized {
                false => {
                stack([
                    resize_area(15).into(),
                    display.into()
                ]).into()
                }

                true => display.into()
            }
        };

        result
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn settings(&self) -> window::Settings {
        self.settings.clone()
    }

}

fn window_message(app: &mut App, window: WinMessage) -> Task<Message> {
    match window {
        WinMessage::ResizeStart(direction) => {app.state.resizing = Some(direction); Task::none()},
        WinMessage::ResizeMove(point) => if let Some(direction) = &app.state.resizing
            {resize(direction, point)} else {Task::none()},
        WinMessage::ResizeDone => {app.state.resizing = None; Task::none()},
        _ => Task::none()
    }
}

fn resize(direction: &Direction, point: Point) -> Task<Message> {
    println!("{:?}", direction);
    Task::none()
}
