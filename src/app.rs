use ratatui::DefaultTerminal;

use crate::{
    event::{self, Event},
    pages::{Page, foo, home::HomePage},
    state::State,
};

pub struct App;

impl App {
    pub async fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let events_rx = event::init();

        let pages: [Box<dyn Page<State = State>>; 2] = [Box::new(HomePage::default()), Box::new(foo::FooPage::default())];
        let active_page = 0;
        let page = &pages[active_page];
        let mut exit = false;

        while !exit {
            terminal.draw(|frame| {
                if let Err(err) = page.draw(frame) {
                    panic!("{err}")
                }
            })?;
            let result = match events_rx.recv() {
                Ok(Event::UserInput(key_event)) => {
                    page.as_ref()._handle_key_event(key_event, &mut exit).await
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
