use ratatui::DefaultTerminal;

use crate::{
    event::{self, Event},
    pages::{Page, home::HomePage},
};

pub struct App;

impl App {
    pub fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let events_rx = event::init();

        let pages = [Box::new(HomePage::default())];
        let active_page = 0;
        let page = &pages[active_page];
        let mut exit = false;

        while !exit {
            terminal.draw(|frame| page.render(frame).unwrap())?;
            match events_rx.recv() {
                Ok(Event::UserInput(key_event)) => page._handle_key_event(key_event, &mut exit),
                Err(err) => panic!("Error receiving crossterm event {err}"),
            };
        }

        Ok(())
    }
}
