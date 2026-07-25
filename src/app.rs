use crate::{
    pages::{Page, home::HomePage},
    utils::peripherals::{self, PeripheralResponse, PeripheralsInit},
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
    home_page: HomePage,
    active_page: PageKind,
    exit: bool,
    peripherals_init: PeripheralsInit,
}

#[derive(Copy, Clone)]
enum PageKind {
    Home,
}

impl App {
    pub async fn new() -> Result<Self, String> {
        let (peripherals_init, peripherals_client) =
            peripherals::init().await.map_err(|e| e.to_string())?;

        let app = Self {
            home_page: HomePage::new(peripherals_client),
            active_page: PageKind::Home,
            exit: false,
            peripherals_init,
        };

        Ok(app)
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> Result<(), String> {
        let mut reader = EventStream::new();
        let mut tick = time::interval(Duration::from_millis(TICK_IN_MILLISECONDS));

        while !self.exit {
            self.render(terminal).await?;

            let result = select! {
                    _ = tick.tick() => {self.tick().await}

                key_event = reader.next() => {
                    if let Some(Ok(CrosstermEvent::Key(key_event))) = key_event {
                        self.handle_app_key_event(&key_event);
                        self.handle_key_event(&key_event).await
                    } else {
                        Ok(())
                    }
                }

                Some(peripheral_client_request) = self.peripherals_init.peripherals_req_rx.recv() => {
                    self.peripherals_init.peripherals.handle_request(peripheral_client_request, &self.peripherals_init.peripherals_resp_tx).await;
                    Ok(())
                }
                Some(peripheral_client_response) = self.peripherals_init.peripherals_resp_rx.recv() => {self.handle_peripheral_client_response(peripheral_client_response).await}

            };

            if let Err(err) = result {
                println!("Error in event loop {err}");
                break;
            }
        }

        Ok(())
    }

    async fn tick(&mut self) -> Result<(), String> {
        match &mut self.active_page {
            PageKind::Home => <HomePage as Page>::tick(&mut self.home_page).await,
        }
    }

    async fn render(&mut self, terminal: &mut DefaultTerminal) -> Result<(), String> {
        match &mut self.active_page {
            PageKind::Home => {
                terminal
                    .draw(|frame| {
                        let area = frame.area();
                        frame.render_widget(self.home_page.generate_widget(), area);
                    })
                    .map_err(|err| err.to_string())?;
            }
        };

        Ok(())
    }

    async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<(), String> {
        match &mut self.active_page {
            PageKind::Home => {
                <HomePage as Page>::handle_key_event(&mut self.home_page, key_event).await
            }
        }
    }

    async fn handle_peripheral_client_response(
        &mut self,
        peripheral_client_response: PeripheralResponse,
    ) -> Result<(), String> {
        match &mut self.active_page {
            PageKind::Home => {
                <HomePage as Page>::handle_peripheral_client_response(
                    &mut self.home_page,
                    peripheral_client_response,
                )
                .await
            }
        }
    }

    fn handle_app_key_event(&mut self, key_event: &KeyEvent) {
        if key_event.kind == KeyEventKind::Press
            && key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('c')
        {
            self.exit = true;
        }
    }
}
