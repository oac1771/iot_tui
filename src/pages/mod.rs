pub mod home;

use crossterm::event::KeyEvent;
use ratatui::{
    text::Line,
    widgets::{Block, Widget},
};

use crate::utils::peripherals::PeripheralResponse;

pub trait Page {
    fn generate_widget(&self) -> impl Widget {
        Block::bordered().title_top(Line::from("  Widget  ").centered())
    }

    fn tick(&mut self) -> impl Future<Output = Result<(), String>> {
        async { Ok(()) }
    }

    fn handle_key_event(
        &mut self,
        _key_event: &KeyEvent,
    ) -> impl Future<Output = Result<(), String>> {
        async { Ok(()) }
    }

    fn handle_peripheral_client_response(
        &mut self,
        _peripheral_client_response: PeripheralResponse,
    ) -> impl Future<Output = Result<(), String>> {
        async { Ok(()) }
    }
}
