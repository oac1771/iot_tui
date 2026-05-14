use futures_util::StreamExt;
use iot_sdk::{PlatformPeripheral, central::Central};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub async fn start() -> Result<(Peripherals, PeripheralsClient, Receiver<PeripheralRequest>), String>
{
    let peripherals = Peripherals::new().await?;
    let (peripherals_req_tx, peripherals_req_rx) = mpsc::channel(100);
    let client = PeripheralsClient::new(peripherals_req_tx);

    Ok((peripherals, client, peripherals_req_rx))
}

pub struct Peripherals(Central);

pub struct PeripheralsClient(Sender<PeripheralRequest>);

pub enum PeripheralRequest {
    GetPheripherals,
}

#[derive(Debug)]
pub enum PeripheralResponse {
    GetPheripherals(Vec<PlatformPeripheral>),
}

impl Peripherals {
    async fn new() -> Result<Self, String> {
        let central = Central::new().await.map_err(|e| e.to_string())?;
        Ok(Self(central))
    }

    pub async fn handle_request(
        &self,
        peripheral_client_request: PeripheralRequest,
    ) -> Result<PeripheralResponse, String> {
        let result = match peripheral_client_request {
            PeripheralRequest::GetPheripherals => self.get_peripherals().await,
        };

        result
    }

    async fn get_peripherals(&self) -> Result<PeripheralResponse, String> {
        let peripherals = self
            .0
            .peripherals()
            .await
            .map_err(|e| e.to_string())?
            .take(15)
            .collect::<Vec<PlatformPeripheral>>()
            .await;

        let response = PeripheralResponse::GetPheripherals(peripherals);

        Ok(response)
    }
}

impl PeripheralsClient {
    fn new(peripherals_req_tx: Sender<PeripheralRequest>) -> Self {
        Self(peripherals_req_tx)
    }

    pub async fn get_peripherals(&self) -> Result<(), String> {
        let request = PeripheralRequest::GetPheripherals;
        self.send_request(request).await?;

        Ok(())
    }

    async fn send_request(&self, request: PeripheralRequest) -> Result<(), String> {
        self.0.send(request).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
