pub mod home;

use crossterm::event::KeyEvent;
use futures_util::future::BoxFuture;
use ratatui::widgets::StatefulWidget;

pub trait Page: StatefulWidget + Send + Sync {
    type State;

    fn state(&self) -> BoxFuture<Result<<Self as Page>::State, String>>;
    fn handle_key_event(&self, key_event: KeyEvent) -> BoxFuture<Result<(), String>>;
}
