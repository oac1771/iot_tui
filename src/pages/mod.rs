pub mod foo;
pub mod home;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::{FutureExt, future::BoxFuture};
use ratatui::{Frame, widgets::StatefulWidget};

pub trait Page: StatefulWidget + Send + Sync {
    fn draw(&self, frame: &mut Frame) -> Result<(), String>;
    fn handle_key_event(&self, key_event: KeyEvent) -> BoxFuture<Result<(), String>>;

    fn _handle_key_event<'a>(
        &'a self,
        key_event: KeyEvent,
        exit: &'a mut bool,
    ) -> BoxFuture<'a, Result<(), String>>
    where
        Self: Send + Sync + 'static,
    {
        async move {
            if key_event.kind == KeyEventKind::Press
                && key_event.modifiers == KeyModifiers::CONTROL
                && key_event.code == KeyCode::Char('c')
            {
                *exit = true;
                Ok(())
            } else {
                self.handle_key_event(key_event).await
            }
        }
        .boxed()
    }
}
