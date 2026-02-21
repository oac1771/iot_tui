use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Frame, widgets::StatefulWidget};

pub mod home;

pub trait Page: StatefulWidget {
    fn render(&self, frame: &mut Frame) -> Result<(), String>;
    fn handle_key_event(&self, key_event: KeyEvent) -> Result<(), String>;

    fn _handle_key_event(&self, key_event: KeyEvent, exit: &mut bool) -> Result<(), String> {
        if key_event.kind == KeyEventKind::Press
            && key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('c')
        {
            *exit = true;
            Ok(())
        } else {
            self.handle_key_event(key_event)
        }
    }
}
