use futures_util::StreamExt;
use iot_sdk::{PlatformPeripheral, central::Central};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub async fn start() -> Result<PeripheralsInit, String> {
    let peripherals = Peripherals::new().await?;
    let (peripherals_req_tx, peripherals_req_rx) = mpsc::channel(100);
    let (peripherals_resp_tx, peripherals_resp_rx) = mpsc::channel(100);
    let peripherals_client = PeripheralsClient::new(peripherals_req_tx);

    let peripherals_init = PeripheralsInit {
        peripherals,
        peripherals_client,
        peripherals_req_rx,
        peripherals_resp_tx,
        peripherals_resp_rx,
    };

    Ok(peripherals_init)
}

pub struct PeripheralsInit {
    pub peripherals: Peripherals,
    pub peripherals_client: PeripheralsClient,
    pub peripherals_req_rx: Receiver<PeripheralRequest>,
    pub peripherals_resp_tx: Sender<PeripheralResponse>,
    pub peripherals_resp_rx: Receiver<PeripheralResponse>,
}

pub struct Peripherals(Central);

pub struct PeripheralsClient(Sender<PeripheralRequest>);

pub enum PeripheralRequest {
    GetPheripherals,
}

#[derive(Debug)]
pub enum PeripheralResponse {
    PeripheralScanStarted,
    GetPheripherals(Vec<PlatformPeripheral>),
    PeripheralScanError(String),
}

impl Peripherals {
    async fn new() -> Result<Self, String> {
        let central = Central::new().await.map_err(|e| e.to_string())?;
        Ok(Self(central))
    }

    pub async fn handle_request(
        &self,
        peripheral_client_request: PeripheralRequest,
        peripherals_resp_tx: &Sender<PeripheralResponse>,
    ) {
        let central = self.0.clone();
        let tx = peripherals_resp_tx.clone();

        let request_function = match peripheral_client_request {
            PeripheralRequest::GetPheripherals => Self::get_peripherals(central, tx),
        };

        tokio::spawn(async move {
            if let Err(err) = request_function.await {
                panic!("Error handling request: {err}")
            }
        });
    }

    async fn get_peripherals(
        central: Central,
        tx: Sender<PeripheralResponse>,
    ) -> Result<(), String> {
        let result = async {
            tx.send(PeripheralResponse::PeripheralScanStarted)
                .await
                .map_err(|e| e.to_string())?;
            let peripherals = central
                .peripherals()
                .await
                .map_err(|e| e.to_string())?
                .take(15)
                .collect::<Vec<PlatformPeripheral>>()
                .await;

            let response = PeripheralResponse::GetPheripherals(peripherals);
            tx.send(response).await.map_err(|e| e.to_string())?;

            Ok(())
        }
        .await;

        if let Err(err) = result {
            tx.send(PeripheralResponse::PeripheralScanError(err))
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
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
