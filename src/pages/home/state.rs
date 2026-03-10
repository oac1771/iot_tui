use iot_sdk::Characteristic;

use crate::utils::evaluate_wrapping_index;

#[derive(Default, Debug)]
pub struct State {
    index: usize,
    local_names: Vec<String>,
    characteristics: Vec<Vec<Characteristic>>,
}

impl State {
    pub fn get_local_names(&self) -> &Vec<String> {
        &self.local_names
    }

    pub fn get_characteristics(&self) -> &Vec<Characteristic> {
        &self.characteristics[self.index]
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn update_local_names(&mut self, local_names: Vec<String>) {
        self.local_names = local_names
    }

    pub fn update_characteristics(&mut self, characteristics: Vec<Characteristic>) {
        self.characteristics[self.index] = characteristics;
    }

    pub fn clear_characteristics(&mut self, len: usize) {
        self.characteristics = vec![vec![]; len]
    }

    pub fn update_index(&mut self, update: i8) {
        let len = self.local_names.len();

        let index = if len == 0 {
            0
        } else {
            evaluate_wrapping_index(self.index as isize, update as isize, len as isize)
        };
        self.index = index;
    }
}
