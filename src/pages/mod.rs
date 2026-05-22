pub mod home;

use crossterm::event::KeyEvent;
use ratatui::widgets::Widget;

use crate::utils::peripherals::PeripheralResponse;

pub trait Page {
    fn generate_widget(&self) -> impl Widget;

    fn tick(&mut self) -> impl Future<Output = Result<(), String>>;

    fn handle_key_event(
        &mut self,
        key_event: &KeyEvent,
    ) -> impl Future<Output = Result<(), String>>;

    fn handle_peripheral_client_response(
        &mut self,
        peripheral_client_response: PeripheralResponse,
    ) -> impl Future<Output = Result<(), String>>;
}
