mod state;
mod widget;

use crate::{
    pages::home::widget::{DisplayWidget, HomeWidget, PopUpErrorWidget},
    utils::{
        notifications::Notifications,
        peripherals::{PeripheralResponse, PeripheralsClient, ResponseType},
        spinner::Spinner,
    },
};

use super::Page;
use crossbeam::channel;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use iot_sdk::{CharPropFlags, Uuid, ValueNotification};
use ratatui::widgets::Widget;
use state::State;

pub struct HomePage {
    state: State,
    view: View,
    error: Option<String>,
    peripherals_client: PeripheralsClient,
}

#[derive(Clone)]
enum View {
    Peripheral(ViewState),
    Characteristic(ViewState),
    Error(String)
}

#[derive(Clone)]
enum ViewState {
    Idle,
    Scanning((Spinner, String)),
    Payload(Uuid),
    Editing,
    Notifying((channel::Receiver<ValueNotification>, Notifications)),
}

impl HomePage {
    pub fn new(peripherals_client: PeripheralsClient) -> Self {
        Self {
            state: State::default(),
            error: None,
            view: View::Peripheral(ViewState::Idle),
            peripherals_client,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.state.input.character_index.saturating_sub(1);
        self.state.input.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.state.input.character_index.saturating_add(1);
        self.state.input.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.state.input.value.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.state
            .input
            .value
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.state.input.character_index)
            .unwrap_or(self.state.input.value.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.state.input.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.state.input.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self
                .state
                .input
                .value
                .chars()
                .take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.state.input.value.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.state.input.value = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.state.input.value.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.state.input.character_index = 0;
    }
}

impl Page for HomePage {
    async fn tick(&mut self) -> Result<(), String> {
        match &mut self.view {
            View::Peripheral(ViewState::Scanning((spinner, _))) => spinner.tick(),
            View::Characteristic(ViewState::Scanning((spinner, _))) => spinner.tick(),
            _ => {}
        };

        Ok(())
    }

    async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<(), String> {
        if key_event.kind == KeyEventKind::Press && self.error.is_none() {
            match &self.view {
                View::Peripheral(ViewState::Idle) => match key_event.code {
                    KeyCode::Char('s') => {
                        self.peripherals_client.get_peripherals().await?;
                    }
                    KeyCode::Char('c') if !self.state.get_local_names().is_empty() => {
                        let peripheral = self.state.get_indexed_peripheral();
                        self.peripherals_client
                            .get_characteristics(peripheral)
                            .await?;
                    }
                    KeyCode::Up => self.state.update_peripheral_index(-1),
                    KeyCode::Down => self.state.update_peripheral_index(1),
                    KeyCode::Right if self.state.get_characteristics().is_some() => {
                        self.view = View::Characteristic(ViewState::Idle)
                    }
                    _ => {}
                },

                View::Characteristic(ViewState::Idle) => match key_event.code {
                    KeyCode::Up => self.state.update_characteristic_index(-1),
                    KeyCode::Down => self.state.update_characteristic_index(1),
                    KeyCode::Left => self.view = View::Peripheral(ViewState::Idle),
                    KeyCode::Char('r') => {
                        if let Some(characteristic) = self.state.get_indexed_characteristic()
                            && characteristic.properties().contains(CharPropFlags::READ)
                        {
                            let peripheral = self.state.get_indexed_peripheral();
                            self.peripherals_client
                                .read(peripheral.clone(), characteristic.id())
                                .await?;
                        }
                    }
                    KeyCode::Char('w') => {
                        if let Some(characteristic) = self.state.get_indexed_characteristic()
                            && characteristic.properties().contains(CharPropFlags::WRITE)
                        {
                            self.view = View::Characteristic(ViewState::Editing);
                        }
                    }
                    KeyCode::Char('n') => {
                        if let Some(characteristic) = self.state.get_indexed_characteristic()
                            && characteristic.properties().contains(CharPropFlags::NOTIFY)
                        {
                            let peripheral = self.state.get_indexed_peripheral();

                            let result = self
                                .peripherals_client
                                .notify(peripheral.clone(), characteristic.id())
                                .await;

                            match result {
                                Ok(notification_rx) => {
                                    self.view = View::Characteristic(ViewState::Notifying((
                                        notification_rx,
                                        Notifications::default(),
                                    )))
                                }
                                Err(err) => self.error = Some(err),
                            }
                        }
                    }
                    _ => {}
                },
                View::Characteristic(ViewState::Payload(_)) => {
                    if key_event.code == KeyCode::Esc {
                        self.view = View::Characteristic(ViewState::Idle)
                    }
                }
                View::Characteristic(ViewState::Editing) => match key_event.code {
                    KeyCode::Esc => self.view = View::Characteristic(ViewState::Idle),
                    KeyCode::Char(to_insert) => self.enter_char(to_insert),
                    KeyCode::Backspace => self.delete_char(),
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Right => self.move_cursor_right(),
                    KeyCode::Enter => {
                        let write_data = self.state.input.value.clone();
                        self.state.input.value.clear();
                        self.reset_cursor();

                        if let Some(characteristic) = self.state.get_indexed_characteristic() {
                            match characteristic.validate_write_data(write_data) {
                                Ok(data) => {
                                    let peripheral = self.state.get_indexed_peripheral();
                                    self.peripherals_client
                                        .write(peripheral.clone(), characteristic.id(), &data)
                                        .await?;
                                }
                                Err(err) => self.error = Some(err),
                            }
                        }
                    }
                    _ => {}
                },
                View::Characteristic(ViewState::Notifying(_)) => {
                    if key_event.code == KeyCode::Esc {
                        self.view = View::Characteristic(ViewState::Idle)
                    }
                }
                View::Error(err) => {
                    self.error = Some(err.to_string());
                }
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press
            && self.error.is_some()
            && let KeyCode::Esc = key_event.code
        {
            self.error = None;
            self.view = View::Peripheral(ViewState::Idle);
        }

        Ok(())
    }

    async fn handle_peripheral_client_response(
        &mut self,
        peripheral_client_response: PeripheralResponse,
    ) -> Result<(), String> {
        match peripheral_client_response {
            PeripheralResponse::PeripheralScanStarted => {
                self.state.clear_peripherals();
                self.view = View::Peripheral(ViewState::Scanning((
                    Spinner::default(),
                    String::from("Scanning For Peripherals..."),
                )))
            }
            PeripheralResponse::GetPheripherals(peripherals) => {
                self.view = View::Peripheral(ViewState::Idle);
                self.state.clear_characteristics(peripherals.len());
                self.state.clear_peripheral_local_names();
                self.state.update_peripherals(peripherals).await;
            }
            PeripheralResponse::CharacteristicScanStarted => {
                self.view = View::Characteristic(ViewState::Scanning((
                    Spinner::default(),
                    String::from("Scanning For Characteristics..."),
                )))
            }
            PeripheralResponse::GetCharacteristics(characteristics) => {
                self.view = View::Characteristic(ViewState::Idle);
                self.state.update_characteristics(characteristics);
            }
            PeripheralResponse::ScanningMessageUpdate(message) => {
                match &mut self.view {
                    View::Characteristic(state) |  View::Peripheral(state) => {
                        if let ViewState::Scanning((_, scanning_message)) = state {
                            *scanning_message = message
                        }
                    },
                    _ => {}
                };

            }
            PeripheralResponse::ReadCharacteristicCallStarted => {
                self.view = View::Characteristic(ViewState::Scanning((
                    Spinner::default(),
                    String::from("Sending Read Request..."),
                )))
            }
            PeripheralResponse::ReadCharacteristic((characteristic_id, response)) => {
                self.view = View::Characteristic(ViewState::Payload(characteristic_id));
                self.state
                    .update_characteristic_response(characteristic_id, response);
            }
            PeripheralResponse::WriteCharacteristicCallStarted => {
                self.view = View::Characteristic(ViewState::Scanning((
                    Spinner::default(),
                    String::from("Sending Write Request..."),
                )))
            }
            PeripheralResponse::WriteCharacteristic => {
                self.view = View::Characteristic(ViewState::Idle);
            }
            PeripheralResponse::Error((response_type, err)) => {
                match response_type {
                    ResponseType::Peripheral => self.view = View::Peripheral(ViewState::Idle),
                    ResponseType::Characteristic => self.view = View::Peripheral(ViewState::Idle),
                }
                self.error = Some(err);
            }
        };
        Ok(())
    }

    fn generate_widget(&mut self) -> impl Widget {
        if let Some(error) = &self.error {
            HomeWidget::PopUpError(PopUpErrorWidget::new(error.as_ref()))
        } else {
            HomeWidget::Display(DisplayWidget::new(&self.state, &mut self.view))
        }
    }
}
