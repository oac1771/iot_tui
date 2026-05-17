use crate::{
    pages::home::HomePage,
    utils::peripherals::{self, PeripheralsInit},
};
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::{
    select,
    time::{self, Duration},
};

const TICK_IN_MILLISECONDS: u64 = 50;

pub struct App {
    active_page: PageKind,
    exit: bool,
}

#[derive(Copy, Clone)]
enum PageKind {
    Home,
}

impl App {
    pub async fn new() -> Result<Self, String> {
        let app = Self {
            active_page: PageKind::Home,
            exit: false,
        };

        Ok(app)
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let mut reader = EventStream::new();
        let mut tick = time::interval(Duration::from_millis(TICK_IN_MILLISECONDS));

        let PeripheralsInit {
            peripherals,
            peripherals_client,
            mut peripherals_req_rx,
            peripherals_resp_tx,
            mut peripherals_resp_rx,
        } = peripherals::start().await.map_err(std::io::Error::other)?;

        let mut home_page = HomePage::new(peripherals_client);

        while !self.exit {
            terminal.draw(|frame| {
                let area = frame.area();

                match &self.active_page {
                    PageKind::Home => {
                        frame.render_widget(&home_page, area);
                    }
                }
            })?;

            let result = select! {
                _ = tick.tick() => {
                    match &mut self.active_page {
                        PageKind::Home => {
                            home_page.tick().await;
                            Ok(())
                        }
                    }
                }

                key_event = reader.next() => {
                    if let Some(Ok(CrosstermEvent::Key(key_event))) = key_event {

                        self.handle_key_event(&key_event);

                        match &mut self.active_page {
                            PageKind::Home => {
                                home_page.handle_key_event(&key_event).await
                            }
                        }
                    } else {
                        Ok(())
                    }

                }

                Some(peripheral_client_request) = peripherals_req_rx.recv() => {
                    peripherals.handle_request(peripheral_client_request, &peripherals_resp_tx).await;
                    Ok(())
                }

                Some(peripheral_client_response) = peripherals_resp_rx.recv() => {
                    match &mut self.active_page {
                        PageKind::Home => {
                            home_page.handle_peripheral_client_response(peripheral_client_response).await
                        }
                    };
                    Ok(())
                }

            };

            if let Err(err) = result {
                panic!("Error in event loop {err}");
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
