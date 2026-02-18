use crossterm::event::KeyCode;
use std::{
    sync::mpsc::{self, Receiver, Sender, SyncSender, sync_channel},
    thread,
};

#[derive(Default)]
pub struct State {
    exit: bool,
    list_item_index: usize,
    error: Option<String>,
}

#[derive(Clone)]
pub struct StateClient {
    state_update_tx: Sender<StateActions>,
}

#[derive(Debug)]
pub enum StateActions {
    UpdateExit,
    ReadExit(SyncSender<bool>),
    UpdateListItemIndex(i8),
    ReadListItemIndex(SyncSender<usize>),
    UpdateError(Option<String>),
    ReadError(SyncSender<Option<String>>),
}

pub fn init() -> StateClient {
    let (state_update_tx, state_update_rx) = mpsc::channel::<StateActions>();
    let state_client = StateClient::new(state_update_tx);
    let task_state_client = state_client.clone();

    thread::spawn(move || {
        let mut state = State::default();
        loop {
            if let Err(err) = State::handle_state_updates(&state_update_rx, &mut state) {
                task_state_client.update_error(Some(err)).expect("REASON")
            }
        }
    });

    state_client
}

impl State {
    pub fn read_list_item_index(&self) -> usize {
        self.list_item_index
    }

    pub fn read_error(&self) -> &Option<String> {
        &self.error
    }

    fn handle_state_updates(
        state_update_rx: &Receiver<StateActions>,
        state: &mut State,
    ) -> Result<(), String> {
        while let Ok(action) = state_update_rx.recv() {
            match action {
                StateActions::UpdateExit => state.exit = !state.exit,
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
                }
                StateActions::UpdateError(err) => state.error = err,
                StateActions::ReadError(sender) => {
                    if let Err(err) = sender.send(state.error.clone()) {
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

    pub fn to_state(&self) -> Result<State, Error<StateActions>> {
        Ok(State {
            exit: self.read_exit()?,
            list_item_index: self.read_list_item_index()?,
            error: self.read_error()?,
        })
    }

    pub fn update_exit(&self) -> Result<(), Error<StateActions>> {
        self.state_update_tx.send(StateActions::UpdateExit)?;
        Ok(())
    }

    pub fn update_list_item_index(&self, code: KeyCode) -> Result<(), Error<StateActions>> {
        let update = match code {
            KeyCode::Up => -1,
            KeyCode::Down => 1,
            _ => return Err(Error::UnexpectedKeyCode),
        };
        self.state_update_tx
            .send(StateActions::UpdateListItemIndex(update))?;

        Ok(())
    }

    pub fn update_error(&self, err: Option<String>) -> Result<(), Error<StateActions>> {
        self.state_update_tx.send(StateActions::UpdateError(err))?;

        Ok(())
    }

    pub fn read_exit(&self) -> Result<bool, Error<StateActions>> {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx.send(StateActions::ReadExit(sender))?;
        let exit = receiver.recv()?;
        Ok(exit)
    }

    pub fn read_list_item_index(&self) -> Result<usize, Error<StateActions>> {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadListItemIndex(sender))?;
        let index = receiver.recv()?;
        Ok(index)
    }

    pub fn read_error(&self) -> Result<Option<String>, Error<StateActions>> {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx.send(StateActions::ReadError(sender))?;
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
    #[error("Unexpected Key Code")]
    UnexpectedKeyCode,
}
