use iot_sdk::{Peripheral, PlatformPeripheral, Uuid};
use std::collections::{BTreeMap, HashMap};

use crate::utils::peripherals::KnownCharacteristic;

#[derive(Debug)]
pub struct State {
    peripheral_index: usize,
    characteristic_index: usize,
    peripherals: Vec<PlatformPeripheral>,
    local_names: Vec<String>,
    characteristics: Vec<Vec<KnownCharacteristic>>,
    characteristic_responses: HashMap<Uuid, CharacteristicResponse>,
    characteristic_map: BTreeMap<Uuid, KnownCharacteristic>,
}

impl State {
    // This function cannot be async, its used in rendering which cant do async
    pub fn get_local_names(&self) -> &Vec<String> {
        &self.local_names
    }

    pub fn get_indexed_peripheral(&self) -> &PlatformPeripheral {
        &self.peripherals[self.peripheral_index]
    }

    pub fn get_indexed_characteristic(&self) -> Option<&KnownCharacteristic> {
        if let Some(characteristic) = self.get_characteristics() {
            Some(&characteristic[self.characteristic_index])
        } else {
            None
        }
    }

    pub fn get_characteristics(&self) -> Option<&Vec<KnownCharacteristic>> {
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

    pub fn get_characteristic(&self, characteristic_id: &Uuid) -> Option<&KnownCharacteristic> {
        self.characteristic_map.get(characteristic_id)
    }

    pub fn get_characteristic_response(
        &self,
        characteristic_id: &Uuid,
    ) -> Option<&CharacteristicResponse> {
        self.characteristic_responses.get(characteristic_id)
    }

    pub fn clear_peripheral_local_names(&mut self) {
        self.local_names.clear();
    }

    pub async fn update_peripherals(&mut self, peripherals: Vec<PlatformPeripheral>) {
        for p in &peripherals {
            if let Ok(Some(peripheral_properties)) = p.properties().await
                && let Some(local_name) = peripheral_properties.local_name
            {
                self.local_names.push(local_name)
            }
        }

        self.peripherals = peripherals
    }

    pub fn update_characteristics(&mut self, characteristics: Vec<KnownCharacteristic>) {
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

    pub fn update_characteristic_response(
        &mut self,
        characteristic: KnownCharacteristic,
        response: Vec<u8>,
    ) {
        let characteristic_id = characteristic.id();
        let characteristic_response = CharacteristicResponse::new(response);
        self.characteristic_responses
            .insert(characteristic_id, characteristic_response);
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
            characteristic_responses: HashMap::new(),
            characteristic_map: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacteristicResponse {
    raw_data: Vec<u8>,
}

impl CharacteristicResponse {
    pub fn new(raw_data: Vec<u8>) -> Self {
        Self { raw_data }
    }

    pub fn data(&self) -> &[u8] {
        &self.raw_data
    }
}
