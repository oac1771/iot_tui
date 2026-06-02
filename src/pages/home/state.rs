use iot_sdk::{Characteristic, Peripheral, PlatformPeripheral};

#[derive(Debug)]
pub struct State {
    peripheral_index: usize,
    characteristic_index: usize,
    peripherals: Vec<PlatformPeripheral>,
    local_names: Vec<String>,
    characteristics: Vec<Vec<Characteristic>>,
}

impl State {
    pub fn get_local_names(&self) -> &Vec<String> {
        &self.local_names
    }

    pub fn get_indexed_peripheral(&self) -> &PlatformPeripheral {
        &self.peripherals[self.peripheral_index]
    }

    pub fn get_indexed_characteristic(&self) -> Option<&Characteristic> {
        if let Some(characteristic) = self.get_characteristics() {
            Some(&characteristic[self.characteristic_index])
        } else {
            None
        }
    }

    pub fn get_characteristics(&self) -> Option<&Vec<Characteristic>> {
        if self.characteristics[self.peripheral_index].is_empty() {
            None
        } else {
            Some(&self.characteristics[self.peripheral_index])
        }
    }

    pub fn get_peripheral_index(&self) -> usize {
        self.peripheral_index
    }

    pub fn get_characteristic_index(&self) -> usize {
        self.characteristic_index
    }

    pub async fn update_peripherals(&mut self, peripherals: Vec<PlatformPeripheral>) {
        self.local_names.clear();
        for p in &peripherals {
            if let Some(local_name) = p.properties().await.unwrap().unwrap().local_name {
                self.local_names.push(local_name)
            }
        }

        self.peripherals = peripherals
    }

    pub fn update_characteristics(&mut self, characteristics: Vec<Characteristic>) {
        self.characteristics[self.peripheral_index] = characteristics;
    }

    pub fn clear_characteristics(&mut self, len: usize) {
        self.characteristics = vec![vec![]; len]
    }

    pub fn clear_peripherals(&mut self) {
        self.local_names = Vec::new();
        self.peripherals = Vec::new();
    }

    pub fn update_peripheral_index(&mut self, update: i8) {
        let len = self.peripherals.len();

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

fn evaluate_wrapping_index(current_index: isize, update: isize, len: isize) -> usize {
    ((current_index + update).rem_euclid(len)) as usize
}

impl Default for State {
    fn default() -> Self {
        Self {
            peripheral_index: 0,
            characteristic_index: 0,
            peripherals: Vec::new(),
            local_names: Vec::new(),
            characteristics: vec![vec![]; 1],
        }
    }
}
