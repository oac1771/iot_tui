use std::collections::HashSet;

use crate::utils::evaluate_wrapping_index;

#[derive(Default, Clone, Debug)]
pub struct State {
    scan_items: (usize, HashSet<(String, String)>),
}

impl State {
    pub fn get_scan_items(&self) -> (usize, impl Iterator<Item = (&str, &str)>) {
        (
            self.scan_items.0,
            self.scan_items
                .1
                .iter()
                .map(|i| (i.0.as_str(), i.1.as_str())),
        )
    }
    pub fn update_scan_items(&mut self, scan_items: HashSet<(String, String)>) {
        self.scan_items.1 = scan_items
    }

    pub fn update_scan_items_index(&mut self, update: i8) {
        let len = self.scan_items.1.len();

        let scan_item_index = if len == 0 {
            0
        } else {
            evaluate_wrapping_index(self.scan_items.0 as isize, update as isize, len as isize)
        };
        self.scan_items.0 = scan_item_index;
    }
}
