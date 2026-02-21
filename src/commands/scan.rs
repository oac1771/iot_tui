use crate::state::StateClient;
use iot_sdk::central::Central;

const IOT_LOCAL_NAME: &str = "TrouBLE [Trouble Example]";

pub struct ScanCmd;

impl ScanCmd {
    pub async fn handle(_state_client: &StateClient) -> Result<(), String> {
        let central = Central::new().await.map_err(|err| err.to_string())?;
        let _peripheral = central.find_peripheral(IOT_LOCAL_NAME).await.map_err(|err| err.to_string())?;

        Ok(())
    }
}
