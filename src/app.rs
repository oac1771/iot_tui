use ratatui::DefaultTerminal;

use crate::{
    event::{self, Event},
    pages::{Page, home::HomePage},
    state::State,
    util::evaluate_wrapping_index,
};

pub struct App;

impl App {
    pub async fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let events_rx = event::init();

        let pages: [Box<dyn Page<State = State>>; 1] = [
            Box::new(HomePage::default()),
        ];
        let mut app_state = AppState::new(pages.len());

        while !app_state.exit {
            let page = &pages[app_state.active_page];

            terminal.draw(|frame| {
                if let Err(err) = page.draw(frame) {
                    panic!("{err}")
                }
            })?;
            let result = match events_rx.recv() {
                Ok(Event::UserInput(key_event)) => {
                    page.as_ref()
                        ._handle_key_event(key_event, &mut app_state)
                        .await
                }
                Err(_err) => Err("Error receiving crossterm event {err}".to_string()),
            };

            if let Err(err) = result {
                panic!("{err}")
            }
        }

        Ok(())
    }
}

pub struct AppState {
    exit: bool,
    pub active_page: usize,
    page_len: usize,
}

impl AppState {
    pub fn new(page_len: usize) -> Self {
        Self {
            exit: false,
            active_page: 0,
            page_len,
        }
    }
    pub fn update_exit(&mut self, exit: bool) {
        self.exit = exit;
    }

    pub fn update_active_page(&mut self, update: i8) {
        self.active_page = evaluate_wrapping_index(
            self.active_page as isize,
            update as isize,
            self.page_len as isize,
        );
    }
}
