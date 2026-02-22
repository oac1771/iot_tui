use ratatui::DefaultTerminal;
use crossterm::event::{Event as CrosstermEvent, EventStream};
use futures_util::StreamExt;
use crate::{
    pages::{Page, home::HomePage},
    state::State,
    util::evaluate_wrapping_index,
};

pub struct App;

impl App {
    pub async fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let mut reader = EventStream::new();

        let pages: [Box<dyn Page<State = State>>; 1] = [Box::new(HomePage::default())];
        let mut app_state = AppState::new(pages.len());

        while !app_state.exit {
            let page = &pages[app_state.active_page];

            terminal.draw(|frame| {
                if let Err(err) = page.draw(frame) {
                    panic!("{err}")
                }
            })?;
            if let Some(Ok(event)) = reader.next().await {
                if let CrosstermEvent::Key(key_event) = event {
                    if let Err(err) =
                        page._handle_key_event(key_event, &mut app_state).await
                    {
                        panic!("{err}");
                    }
                }
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
