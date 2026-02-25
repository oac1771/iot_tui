use crate::pages::home::{HomePage, HomePageEvent};
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::{
    select,
    sync::mpsc::{self, Receiver},
    time::{self, Duration},
};

pub struct App {
    home_page: HomePage,
    home_page_event_rx: Receiver<HomePageEvent>,
    active_page: PageKind,
    exit: bool,
}

#[derive(Copy, Clone)]
enum PageKind {
    Home,
}

impl App {
    pub fn new() -> Self {
        let (home_page_event_tx, home_page_event_rx) = mpsc::channel(50);

        Self {
            home_page: HomePage::new(home_page_event_tx),
            home_page_event_rx,
            active_page: PageKind::Home,
            exit: false,
        }
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let mut reader = EventStream::new();
        let mut tick = time::interval(Duration::from_millis(50));

        while !self.exit {
            terminal.draw(|frame| {
                let area = frame.area();

                match &self.active_page {
                    PageKind::Home => {
                        frame.render_widget(&self.home_page, area);
                    }
                }
            })?;

            let result = select! {
                _ = tick.tick() => {
                    match &mut self.active_page {
                        PageKind::Home => {
                            self.home_page.tick().await;
                            Ok(())
                        }
                    }
                }

                key_event = reader.next() => {
                    if let Some(Ok(CrosstermEvent::Key(key_event))) = key_event {

                        self.handle_key_event(&key_event);

                        match &mut self.active_page {
                            PageKind::Home => {
                                self.home_page.handle_key_event(&key_event).await
                            }
                        }
                    } else {
                        Ok(())
                    }

                }

                Some(event) = self.home_page_event_rx.recv() => self.home_page.handle_page_event(event).await

            };

            if let Err(err) = result {
                panic!("Error {err}");
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
