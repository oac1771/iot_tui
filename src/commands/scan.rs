use crate::state::State;
use futures_util::StreamExt;
use iot_sdk::central::Central;

pub struct ScanCmd;

impl ScanCmd {
    pub async fn handle(state: &mut State) -> Result<(), String> {
        let central = Central::new().await.map_err(|err| err.to_string())?;
        let peripherals = central
            .peripheral_properties()
            .await
            .map_err(|err| err.to_string())?
            .filter_map(|p| async { p.local_name })
            .take(5)
            .collect::<Vec<String>>()
            .await;

        state.update_scan_items(peripherals);

        Ok(())
    }
}
