use iced::{
    Element, Theme, Task,
    application, window,
    widget::{
        column, stack
    }
};

use rfd;

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

        match message {
            Message::Operation(operation) => trace::operation_message(operation),
            Message::Pane(pane) => pane_message(state, pane),
            _ => ()
        };

        Task::none()
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

// DIALOG FUNCTIONS (those small windows when your program encounters an error, or when you wanna pick a file, you absolutely know what i mean just cant remember trust me)

pub struct Dialog; // Struct for dialog functions, and for easy export (i know i could make a module, but i dont wanna flood it with too many files, i already feel like ive got many)

impl Dialog {
    pub fn error(msg: &str, title: Option<&str>) {
        rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .set_title(format!("Three Body Debugger - {}", title.unwrap_or("Error")))
        .set_description(msg)
        .show();
    }

    pub fn info(msg: &str, title: Option<&str>) {
        rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::Ok)
        .set_title(format!("Three Body Debugger - {}", title.unwrap_or("Info")))
        .set_description(msg)
        .show();
    }

    pub fn warning(msg: &str, title: Option<&str>) {
        rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::Ok)
        .set_title(format!("Three Body Debugger - {}", title.unwrap_or("Warning")))
        .set_description(msg)
        .show();
    }

    pub fn file(dir: Option<std::path::PathBuf>, file: Option<String>) -> Option<std::path::PathBuf> {
        let dir = dir.unwrap_or(std::env::current_dir().unwrap_or("/".into()));
        let file = file.unwrap_or("".to_string());

        rfd::FileDialog::new()
        .set_directory(dir)
        .set_file_name(file)
        .set_title("Select an executable to debug.")
        .pick_file()
    }
}