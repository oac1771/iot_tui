use crate::util::evaluate_wrapping_index;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot,
};

#[derive(Default, Clone, Debug)]
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
    GetState(oneshot::Sender<State>),
    UpdateScanItemsIndex(i8),
    UpdateScanItems(Vec<String>),
    UpdateError(Option<String>),
    GetError(oneshot::Sender<Option<String>>),
}

pub fn init() -> StateClient {
    let (tx, rx) = mpsc::channel::<StateActions>(32);

    tokio::spawn(async move {
        let state = State::default();
        State::handle_state_updates(rx, state).await;
    });

    StateClient {
        state_update_tx: tx,
    }
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

    async fn handle_state_updates(mut state_update_rx: Receiver<StateActions>, mut state: State) {
        while let Some(action) = state_update_rx.recv().await {
            let result = match action {
                StateActions::GetState(sender) => {
                    sender.send(state.clone()).map_err(|_| String::from(""))
                }
                StateActions::UpdateScanItemsIndex(update) => {
                    let len = state.scan_items.1.len();

                    let scan_item_index = if len == 0 {
                        0
                    } else {
                        evaluate_wrapping_index(
                            state.scan_items.0 as isize,
                            update as isize,
                            len as isize,
                        )
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
                StateActions::GetError(sender) => sender
                    .send(state.error.clone())
                    .map_err(|_| String::from("")),
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

    pub async fn get_state(&self) -> Result<State, Error<StateActions>> {
        let (sender, receiver) = oneshot::channel();
        self.state_update_tx
            .send(StateActions::GetState(sender))
            .await?;
        let state = receiver.await?;
        Ok(state)
    }

    pub async fn update_scan_items_index(&self, update: i8) -> Result<(), Error<StateActions>> {
        self.state_update_tx
            .send(StateActions::UpdateScanItemsIndex(update))
            .await?;
        Ok(())
    }

    pub async fn update_scan_items(
        &self,
        scan_items: Vec<String>,
    ) -> Result<(), Error<StateActions>> {
        self.state_update_tx
            .send(StateActions::UpdateScanItems(scan_items))
            .await?;
        Ok(())
    }

    pub async fn update_error(&self, err: Option<String>) -> Result<(), Error<StateActions>> {
        self.state_update_tx
            .send(StateActions::UpdateError(err))
            .await?;

        Ok(())
    }

    pub async fn get_error(&self) -> Result<Option<String>, Error<StateActions>> {
        let (sender, receiver) = oneshot::channel();
        self.state_update_tx
            .send(StateActions::GetError(sender))
            .await?;
        let error = receiver.await?;
        Ok(error)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<T> {
    #[error("{source}")]
    Send {
        #[from]
        source: tokio::sync::mpsc::error::SendError<T>,
    },
    #[error("{source}")]
    OneshotRecv {
        #[from]
        source: tokio::sync::oneshot::error::RecvError,
    },
}
