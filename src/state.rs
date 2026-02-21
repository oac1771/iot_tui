use std::{
    sync::mpsc::{self, Receiver, Sender, SyncSender, sync_channel},
    thread,
};

#[derive(Default, Clone)]
pub struct State {
    error: Option<String>,
    scan_items: (usize, Vec<String>),
}

#[derive(Clone)]
pub struct StateClient {
    state_update_tx: Sender<StateActions>,
}

#[derive(Debug)]
pub enum StateActions {
    GetState(SyncSender<State>),
    UpdateScanItemsIndex(i8),
    UpdateScanItems(Vec<String>),
    UpdateError(Option<String>),
    GetError(SyncSender<Option<String>>),
}

pub fn init() -> StateClient {
    let (state_update_tx, state_update_rx) = mpsc::channel::<StateActions>();
    let state_client = StateClient::new(state_update_tx);

    thread::spawn(move || {
        let state = State::default();
        State::handle_state_updates(&state_update_rx, state);
    });

    state_client
}

impl State {
    pub fn get_scan_items(&self) -> (usize, impl Iterator<Item = &str>) {
        (
            self.scan_items.0,
            self.scan_items.1.iter().map(|i| i.as_str()),
        )
    }

    pub fn get_error(&self) -> &Option<String> {
        &self.error
    }

    fn handle_state_updates(state_update_rx: &Receiver<StateActions>, mut state: State) {
        while let Ok(action) = state_update_rx.recv() {
            let result = match action {
                StateActions::GetState(sender) => {
                    if let Err(err) = sender.send(state.clone()) {
                        Err(err.to_string())
                    } else {
                        Ok(())
                    }
                }
                StateActions::UpdateScanItemsIndex(update) => {
                    let len = state.scan_items.1.len();

                    let scan_item_index = if len == 0 {
                        0
                    } else {
                        ((state.scan_items.0 as isize + update as isize).rem_euclid(len as isize))
                            as usize
                    };
                    state.scan_items.0 = scan_item_index;

                    Ok(())
                }
                StateActions::UpdateScanItems(scan_items) => {
                    state.scan_items.1 = scan_items;

                    Ok(())
                }
                StateActions::UpdateError(err) => {
                    state.error = err;
                    Ok(())
                }
                StateActions::GetError(sender) => {
                    if let Err(err) = sender.send(state.error.clone()) {
                        Err(err.to_string())
                    } else {
                        Ok(())
                    }
                }
            };

            if let Err(err) = result {
                state.error = Some(err);
            };
        }
    }
}

impl StateClient {
    fn new(state_update_tx: Sender<StateActions>) -> Self {
        Self { state_update_tx }
    }

    pub fn get_state(&self) -> Result<State, Error<StateActions>> {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx.send(StateActions::GetState(sender))?;
        let state = receiver.recv()?;
        Ok(state)
    }

    pub fn update_scan_items_index(&self, update: i8) -> Result<(), Error<StateActions>> {
        self.state_update_tx
            .send(StateActions::UpdateScanItemsIndex(update))?;

        Ok(())
    }

    pub fn update_scan_items(&self, scan_items: Vec<String>) -> Result<(), Error<StateActions>> {
        self.state_update_tx
            .send(StateActions::UpdateScanItems(scan_items))?;

        Ok(())
    }

    pub fn update_error(&self, err: Option<String>) -> Result<(), Error<StateActions>> {
        self.state_update_tx.send(StateActions::UpdateError(err))?;

        Ok(())
    }

    pub fn get_error(&self) -> Result<Option<String>, Error<StateActions>> {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx.send(StateActions::GetError(sender))?;
        let error = receiver.recv()?;
        Ok(error)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<T> {
    #[error("{source}")]
    Send {
        #[from]
        source: std::sync::mpsc::SendError<T>,
    },
    #[error("{source}")]
    Recv {
        #[from]
        source: std::sync::mpsc::RecvError,
    },
}
