use crate::state::StateClient;

use rand::{self, Rng};

pub struct ScanCmd;

impl ScanCmd {
    pub fn handle(state_client: &StateClient) -> Result<(), String> {
        let num: usize = rand::thread_rng().gen_range(0..10);

        let scan_items = vec![String::from("foo"); num];
        state_client.update_scan_items(scan_items).unwrap();
        Ok(())
    }
}
