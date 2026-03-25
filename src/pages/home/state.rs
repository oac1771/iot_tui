use iot_sdk::Characteristic;

use crate::utils::evaluate_wrapping_index;

#[derive(Default, Debug)]
pub struct State {
    peripheral_index: usize,
    characteristic_index: usize,
    local_names: Vec<String>,
    characteristics: Vec<Vec<Characteristic>>,
}

impl State {
    pub fn get_local_names(&self) -> &Vec<String> {
        &self.local_names
    }

    pub fn get_characteristics(&self) -> &Vec<Characteristic> {
        &self.characteristics[self.peripheral_index]
    }

    pub fn get_peripheral_index(&self) -> usize {
        self.peripheral_index
    }

    pub fn get_characteristic_index(&self) -> usize {
        self.characteristic_index
    }

    pub fn update_local_names(&mut self, local_names: Vec<String>) {
        self.local_names = local_names
    }

    pub fn update_characteristics(&mut self, characteristics: Vec<Characteristic>) {
        self.characteristics[self.peripheral_index] = characteristics;
    }

    pub fn clear_characteristics(&mut self, len: usize) {
        self.characteristics = vec![vec![]; len]
    }

    pub fn update_peripheral_index(&mut self, update: i8) {
        let len = self.local_names.len();

        let index = if len == 0 {
            0
        } else {
            evaluate_wrapping_index(
                self.peripheral_index as isize,
                update as isize,
                len as isize,
            )
        };
        self.peripheral_index = index;
    }

    pub fn update_characteristic_index(&mut self, update: i8) {
        let len = self.characteristics[self.peripheral_index].len();

        let index = if len == 0 {
            0
        } else {
            evaluate_wrapping_index(
                self.characteristic_index as isize,
                update as isize,
                len as isize,
            )
        };
        self.characteristic_index = index;
    }
}
