use iced::{application};
use iced::{Element, Theme, Settings};


use crate::ui::*;
use crate::config;

#[derive(Default)]
pub struct App {
    state: State,
    theme: Theme,
    settings: Settings      //while these settings dont affect the window settings on runtime, we still save them for conditional use with the UI, like decorations or creating a new window
    // add more as needed
}

pub struct State {
//    panes: pane_grid::State<Pane>
    // add more as needed
}


#[derive(Debug, Clone)]
pub enum Message {
    //Pane(PaneMessage),
    Operation(Operation),
    Other
}

#[derive(Debug, Clone)]
enum Operation {
    NewWindow,
    LoadFile,
    ReloadFile,
    SaveLog,
    OpenSettings
}

pub fn run_app() -> iced::Result {
    application("Three Body Debugger", App::update, App::view)
    .theme(App::theme)
    .settings(App::default().settings) // because we have to specify settings before execution we have to run this function twice
    .run()
}

impl App {
    pub fn new() -> Self {
        Self {
            state: State::default(),
            theme: Theme::Dark,
            settings: Settings {
                ..Default::default()
            },
        }
    }

    fn default() -> Self {
        config::get_app().unwrap_or(Self::new())
    }

    fn update(&mut self, message: Message) {
        let state = &mut self.state;

    }


    fn view(&self) -> Element<'_, Message> {
        let state = &self.state;

        let titlebar: Option<_> = titlebar(&self);


    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn settings(&self) -> Settings {
        self.settings.clone()
    }

}

impl Default for State {
    fn default() -> Self {
        State {}
    }
}