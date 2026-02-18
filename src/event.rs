use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use crate::state::StateClient;

pub enum Event {
    UserInput(KeyEvent),
}

pub fn init(state_client: StateClient) -> Receiver<Event> {
    let (events_tx, events_rx) = mpsc::channel::<Event>();
    thread::spawn(move || {
        loop {
            if let Err(err) = handle_input_events(&events_tx) {
                state_client
                    .update_error(Some(err.to_string()))
                    .expect("REASON")
            }
        }
    });

    events_rx
}

fn handle_input_events(events_tx: &Sender<Event>) -> Result<(), String> {
    match event::read() {
        Ok(CrosstermEvent::Key(key_event)) => {
            if let Err(err) = events_tx.send(Event::UserInput(key_event)) {
                return Err(err.to_string());
            }
        }
        Err(err) => return Err(err.to_string()),
        _ => {}
    };

    Ok(())
}
