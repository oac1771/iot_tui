use crate::state::StateClient;
use futures_util::StreamExt;
use iot_sdk::central::Central;

pub struct ScanCmd;

impl ScanCmd {
    pub async fn handle(state_client: &StateClient) -> Result<(), String> {
        let central = Central::new().await.map_err(|err| err.to_string())?;
        let peripherals = central
            .peripheral_properties()
            .await
            .map_err(|err| err.to_string())?
            .filter_map(|p| async { p.local_name })
            .take(5)
            .collect::<Vec<String>>()
            .await;

        state_client
            .update_scan_items(peripherals)
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }
}
