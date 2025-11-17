use iced::{
    Element, Theme, Task,
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
    pub layout: Layout
    //    panes: pane_grid::State<Pane>
    // add more as needed
}

impl Default for State {
    fn default() -> Self {
        State {
            layout: Layout::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(PaneMessage),
    Operation(trace::Operation),
    None
    // add more as needed
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
                decorations: true,
                ..Default::default()
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let state = &mut self.state;

        let task: Task<Message> = match message {
            Message::Operation(operation) => {trace::operation_message(operation); Task::none()},
            Message::Pane(pane) => pane_message(state, pane),
            _ => Task::none()
        };

        task
    }


    fn view(&self) -> Element<'_, Message> {
        let state = &self.state;

        let content = content(state);

        content.into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn settings(&self) -> window::Settings {
        self.settings.clone()
    }

}
