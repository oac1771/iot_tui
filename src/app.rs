use crate::pages::{Page as PageT, home::HomePage};
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;

pub struct App<H: PageT> {
    home_page: H,
}

impl App<HomePage> {
    pub fn new() -> App<HomePage> {
        Self {
            home_page: HomePage::default(),
        }
    }

    pub async fn run(self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let mut reader = EventStream::new();
        let mut app_state = AppState::new();

        while !app_state.exit {
            let page = self.page(&app_state);
            let mut state = page.state().await.unwrap();

            terminal.draw(|frame| {
                frame.render_stateful_widget(page.clone(), frame.area(), &mut state);
            })?;

            if let Some(Ok(event)) = reader.next().await {
                if let CrosstermEvent::Key(key_event) = event {
                    app_state.handle_key_event(key_event);
                    page.handle_key_event(key_event).await.unwrap();
                }
            }
        }

        Ok(())
    }

    fn page(&self, app_state: &AppState) -> &HomePage {
        match app_state.active_page {
            Page::Home => &self.home_page,
        }
    }
}

pub struct AppState {
    exit: bool,
    active_page: Page,
}

enum Page {
    Home,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            exit: false,
            active_page: Page::Home,
        }
    }
    pub fn update_exit(&mut self, exit: bool) {
        self.exit = exit;
    }

    pub fn update_active_page(&mut self) {
        self.active_page = Page::Home
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Press
            && key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('c')
        {
            self.update_exit(true);
        } else if key_event.kind == KeyEventKind::Press
            && key_event.modifiers == KeyModifiers::SHIFT
            && key_event.code == KeyCode::Right
        {
            self.update_active_page();
        } else if key_event.kind == KeyEventKind::Press
            && key_event.modifiers == KeyModifiers::SHIFT
            && key_event.code == KeyCode::Left
        {
            self.update_active_page();
        }
    }
}
