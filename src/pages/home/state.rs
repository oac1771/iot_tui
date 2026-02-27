use crate::utils::evaluate_wrapping_index;

#[derive(Default, Clone, Debug)]
pub struct State {
    peripherals: (usize, Vec<PheripheralScanItems>),
}

#[derive(Default, Clone, Debug)]
pub struct PheripheralScanItems {
    local_name: String,
    address: String,
}

impl PheripheralScanItems {
    pub fn new(local_name: String, address: String) -> Self {
        Self {
            local_name,
            address,
        }
    }

    pub fn local_name(&self) -> &str {
        &self.local_name
    }

    pub fn address(&self) -> &str {
        &self.address
    }
}

impl State {
    pub fn get_peripherals(&self) -> (usize, &Vec<PheripheralScanItems>) {
        (self.peripherals.0, &self.peripherals.1)
    }
    pub fn update_peripherals(&mut self, peripherals: Vec<PheripheralScanItems>) {
        self.peripherals.1 = peripherals
    }

    pub fn update_peripherals_index(&mut self, update: i8) {
        let len = self.peripherals.1.len();

        let scan_item_index = if len == 0 {
            0
        } else {
            evaluate_wrapping_index(self.peripherals.0 as isize, update as isize, len as isize)
        };
        self.peripherals.0 = scan_item_index;
    }
}
