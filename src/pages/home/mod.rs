mod state;
mod widget;

use crate::{
    pages::home::widget::{DisplayWidget, HomeWidget, PopUpErrorWidget},
    utils::{
        peripherals::{PeripheralResponse, PeripheralsClient},
        spinner::Spinner,
    },
};

use super::Page;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use iot_sdk::{CharPropFlags, Uuid};
use ratatui::widgets::Widget;
use state::State;

pub struct HomePage {
    state: State,
    view: View,
    error: Option<String>,
    peripherals_client: PeripheralsClient,
}

enum View {
    Peripheral(ViewState),
    Characteristic(ViewState),
}

enum ViewState {
    Idle,
    Scanning((Spinner, String)),
    Payload(Uuid),
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
            match self.view {
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
                            && characteristic.properties.contains(CharPropFlags::READ)
                        {
                            let peripheral = self.state.get_indexed_peripheral();
                            self.peripherals_client
                                .read(peripheral.clone(), characteristic.uuid)
                                .await?;
                        }
                    }
                    KeyCode::Char('w') => {
                        if let Some(characteristic) = self.state.get_indexed_characteristic()
                            && characteristic.properties.contains(CharPropFlags::WRITE)
                        {
                            println!("Sending write!")
                            // change view State to editing
                        }
                    }
                    _ => {}
                },
                View::Characteristic(ViewState::Payload(_)) => {
                    if key_event.code == KeyCode::Esc {
                        self.view = View::Characteristic(ViewState::Idle)
                    }
                }
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press
            && self.error.is_some()
            && let KeyCode::Esc = key_event.code
        {
            self.error = None
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
            PeripheralResponse::PeripheralScanError(err) => {
                self.view = View::Peripheral(ViewState::Idle);
                self.error = Some(err);
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
                let view_state = match &mut self.view {
                    View::Characteristic(state) => state,
                    View::Peripheral(state) => state,
                };

                if let ViewState::Scanning((_, scanning_message)) = view_state {
                    *scanning_message = message
                }
            }
            PeripheralResponse::CharacteristicScanError(err) => {
                self.view = View::Peripheral(ViewState::Idle);
                self.error = Some(err);
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
        };
        Ok(())
    }

    fn generate_widget(&self) -> impl Widget {
        if let Some(error) = &self.error {
            HomeWidget::PopUpError(PopUpErrorWidget::new(error.as_ref()))
        } else {
            HomeWidget::Display(DisplayWidget::new(&self.state, &self.view))
        }
    }
}
