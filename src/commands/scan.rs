use crate::state::StateClient;
use iot_sdk::central::Central;

pub struct ScanCmd;

impl ScanCmd {
    pub async fn handle(state_client: &StateClient) -> Result<(), String> {
        // let _central = Central::new().await.map_err(|err| err.to_string())?;
        state_client
            .update_scan_items(vec![String::from("fooo"); 10])
            .unwrap();
        Ok(())
    }
}
