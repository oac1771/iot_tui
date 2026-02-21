use crate::state::StateClient;
use iot_sdk::central::Central;

pub struct ScanCmd;

impl ScanCmd {
    pub async fn handle(_state_client: &StateClient) -> Result<(), String> {
        let _central = Central::new().await.map_err(|err| err.to_string())?;
        Ok(())
    }
}
