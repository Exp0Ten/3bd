use iced::{application, widget::{pane_grid, text}};

struct State {
    panes: pane_grid::State<Pane>
}

impl Default for State {
    fn default() -> Self {
        State { panes: iced::widget::pane_grid::State::new(Pane::PaneOne).0 }
    }
}

enum Pane {
    PaneOne
}

#[derive(Debug, Clone)]
enum Message {

}


pub fn run() -> iced::Result {
    application("Three Body Debugger", update, view)
    .run()
}


fn update(state: &mut State, message: Message) {

}

fn view(state: &State) -> pane_grid::PaneGrid<Message> {
    pane_grid(&state.panes, |pane, state, is_maximized| {
        pane_grid::Content::new(text("Hello"))
    })
}