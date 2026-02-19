use crate::state::StateClient;

pub struct ScanCmd;

impl ScanCmd {
    pub fn handle(state_client: &StateClient) -> Result<(), String> {
        let scan_items = vec![
            String::from("foo"),
            String::from("foo"),
            String::from("foo"),
            String::from("foo"),
            String::from("foo"),
        ];
        state_client.update_scan_items(scan_items).unwrap();
        Ok(())
    }
}
