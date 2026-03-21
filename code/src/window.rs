use iced::{
    Element, Theme, Task,
    application, window,
};

use rfd;

use crate::{
    ui, trace, data::*
};

#[derive(Default)]
pub struct App {
    pub state: State,
    pub theme: Theme,
    pub settings: window::Settings, //while these settings dont affect the window settings on runtime, we still save them for conditional use with the UI, like decorations or creating a new window
}

#[derive(Default)]
pub struct State {
    pub layout: ui::Layout,
    pub internal: Internal,
    pub status: Option<nix::sys::wait::WaitStatus>,
    pub last_signal: Option<nix::sys::signal::Signal>,
}

#[derive(Debug, Default)]
pub struct Internal {
    pub no_debug: bool,
    pub static_exec: bool,
    pub stopped: bool,
    pub breakpoint: bool,
    pub manual: bool,
    pub source_step: Option<trace::Breakpoints>,
    pub pane: PaneData
}

#[derive(Debug, Default)]
pub struct PaneData {
    pub file: Option<crate::dwarf::SourceIndex>,
    pub comp_dir: Option<std::path::PathBuf>,
    pub output: String,
    pub assembly: Option<crate::dwarf::Assembly>,
    pub stack: Option<Vec<(usize, String)>>,
    pub unique_stack: u32
}

#[derive(Debug, Clone)]
pub enum Message {
    Layout(ui::LayoutMessage),
    Pane(ui::PaneMessage),
    Operation(trace::Operation),
    None
}

pub fn run_app() -> iced::Result {
    application("Three Body Debugger", App::update, App::view)
    .theme(App::theme)
    .window(App::default().settings)
    .run_with(|| (App::default(), Task::none()))
}

impl App {
    fn default() -> Self {
        let config = CONFIG.access().as_ref().unwrap().window.clone().unwrap();
        let size = match config.size {
            Some((width, height)) => iced::Size::new(width as f32, height as f32),
            None => iced::window::Settings::default().size
        };
        let position = match config.position {
            Some((x, y)) => iced::window::Position::Specific(iced::Point { x: x as f32, y: y as f32 }),
            None => iced::window::Position::Default
        };
        Self {
            state: State::default(),
            theme: config.theme.unwrap().to_iced_theme(),
            settings: window::Settings { // no icon unfortunately, no support for Wayland
                decorations: true,
                size,
                position,
                min_size: Some(iced::Size { width: 400., height: 400. }),
                ..Default::default()
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let mut task = None;
        let state = &mut self.state;
        match message {
            Message::Operation(operation) => trace::operation_message(state, operation, &mut task),
            Message::Layout(layout) => ui::layout_message(state, layout),
            Message::Pane(pane) => ui::pane_message(state, pane, &mut task),
            _ => ()
        };

        task.unwrap_or(Task::none())
    }

    fn view(&self) -> Element<'_, Message> {
        let state = &self.state;

        let content = ui::content(state);

        content.into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
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

    pub fn warning_choice(msg: &str, title: Option<&str>) -> rfd::MessageDialogResult {
        rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNo)
        .set_title(format!("Three Body Debugger - {}", title.unwrap_or("Warning")))
        .set_description(msg)
        .show()
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