use crossterm::event::KeyCode;
use std::{
    sync::mpsc::{self, Receiver, Sender, SyncSender, sync_channel},
    thread,
};

#[derive(Default)]
struct State {
    exit: bool,
    list_item_index: usize,
    popup: bool
}

pub struct StateClient {
    state_update_tx: Sender<StateActions>,
}

enum StateActions {
    UpdateExit(bool),
    ReadExit(SyncSender<bool>),
    UpdateListItemIndex(i8),
    ReadListItemIndex(SyncSender<usize>),
    UpdatePopup(bool),
    ReadPopup(SyncSender<bool>),

}

pub fn init() -> StateClient {
    let (state_update_tx, state_update_rx) = mpsc::channel::<StateActions>();

    thread::spawn(move || {
        let mut state = State::default();
        loop {
            if let Err(err) = State::handle_state_updates(&state_update_rx, &mut state) {
                println!("Err: {:?}", err);
            }
        }
    });

    StateClient::new(state_update_tx)
}

impl State {
    fn handle_state_updates(
        state_update_rx: &Receiver<StateActions>,
        state: &mut State,
    ) -> Result<(), String> {
        while let Ok(action) = state_update_rx.recv() {
            match action {
                StateActions::UpdateExit(v) => state.exit = v,
                StateActions::ReadExit(sender) => {
                    if let Err(err) = sender.send(state.exit) {
                        return Err(err.to_string());
                    }
                }
                StateActions::UpdateListItemIndex(update) => {
                    let len = 3;
                    let list_item_index = ((state.list_item_index as isize + update as isize)
                        .rem_euclid(len as isize))
                        as usize;
                    state.list_item_index = list_item_index;
                }
                StateActions::ReadListItemIndex(sender) => {
                    if let Err(err) = sender.send(state.list_item_index) {
                        return Err(err.to_string());
                    }
                },
                StateActions::UpdatePopup(v) => state.popup = v,
                StateActions::ReadPopup(sender) => {
                    if let Err(err) = sender.send(state.popup) {
                        return Err(err.to_string());
                    }
                }
            }
        }

        Ok(())
    }
}

impl StateClient {
    fn new(state_update_tx: Sender<StateActions>) -> Self {
        Self { state_update_tx }
    }

    pub fn update_exit(&self, exit: bool) {
        self.state_update_tx
            .send(StateActions::UpdateExit(exit))
            .unwrap();
    }

    pub fn update_popup(&self, popup: bool) {
        self.state_update_tx
            .send(StateActions::UpdatePopup(popup))
            .unwrap();
    }

    pub fn update_list_item_index(&self, code: KeyCode) -> Result<(), String> {
        let update = match code {
            KeyCode::Up => -1,
            KeyCode::Down => 1,
            _ => return Err(String::from("Foo")),
        };
        self.state_update_tx
            .send(StateActions::UpdateListItemIndex(update))
            .unwrap();

        Ok(())
    }

    pub fn read_exit(&self) -> bool {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadExit(sender))
            .unwrap();
        let exit = receiver.recv().unwrap();
        exit
    }

    pub fn read_list_item_index(&self) -> usize {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadListItemIndex(sender))
            .unwrap();
        let index = receiver.recv().unwrap();
        index
    }

    pub fn read_popup(&self) -> bool {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadPopup(sender))
            .unwrap();
        let popup = receiver.recv().unwrap();
        popup
    }
}
