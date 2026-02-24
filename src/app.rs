use crate::pages::home::HomePage;
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;

pub struct App {
    home_page: HomePage,
    active_page: PageKind,
    exit: bool,
}

#[derive(Copy, Clone)]
enum PageKind {
    Home,
}

impl App {
    pub fn new() -> Self {
        Self {
            home_page: HomePage::default(),
            active_page: PageKind::Home,
            exit: false,
        }
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let mut reader = EventStream::new();

        while !self.exit {
            terminal.draw(|frame| {
                let area = frame.area();

                match &self.active_page {
                    PageKind::Home => {
                        frame.render_widget(&self.home_page, area);
                    }
                }
            })?;

            if let Some(Ok(CrosstermEvent::Key(key_event))) = reader.next().await {
                self.handle_key_event(&key_event);

                match &mut self.active_page {
                    PageKind::Home => {
                        self.home_page.handle_key_event(&key_event).await.unwrap();
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: &KeyEvent) {
        if key_event.kind == KeyEventKind::Press
            && key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('c')
        {
            self.exit = true;
        }
    }
}
