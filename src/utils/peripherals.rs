use futures::FutureExt;
use futures_util::StreamExt;
use iot_sdk::{Characteristic, Peripheral, PlatformPeripheral, Uuid, central::Central};
use services::{
    health::{HEALTH_PING_CHAR_UUID, HEALTH_STATUS_CHAR_UUID, Pong},
    trouble_host::types::gatt_traits::FromGatt,
};
use std::pin::Pin;
use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, sleep},
};

pub async fn init() -> Result<(PeripheralsInit, PeripheralsClient), String> {
    let peripherals = Peripherals::new().await?;
    let (peripherals_req_tx, peripherals_req_rx) = mpsc::channel(100);
    let (peripherals_resp_tx, peripherals_resp_rx) = mpsc::channel(100);
    let peripherals_client = PeripheralsClient::new(peripherals_req_tx);

    let peripherals_init = PeripheralsInit {
        peripherals,
        peripherals_req_rx,
        peripherals_resp_tx,
        peripherals_resp_rx,
    };

    Ok((peripherals_init, peripherals_client))
}

pub struct PeripheralsInit {
    pub peripherals: Peripherals,
    pub peripherals_req_rx: Receiver<PeripheralRequest>,
    pub peripherals_resp_tx: Sender<PeripheralResponse>,
    pub peripherals_resp_rx: Receiver<PeripheralResponse>,
}

pub struct Peripherals(Central);

pub struct PeripheralsClient(Sender<PeripheralRequest>);

pub enum PeripheralRequest {
    GetPheripherals,
    GetCharacteristics(PlatformPeripheral),
    Read((PlatformPeripheral, Uuid)),
}

#[derive(Debug)]
pub enum PeripheralResponse {
    PeripheralScanStarted,
    GetPheripherals(Vec<PlatformPeripheral>),
    PeripheralScanError(String),
    CharacteristicScanStarted,
    ScanningMessageUpdate(String),
    GetCharacteristics(Vec<Characteristic>),
    CharacteristicScanError(String),
    ReadCharacteristicCallStarted,
    ReadCharacteristic((Uuid, Vec<u8>)),
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

        let request_function: Pin<Box<dyn Future<Output = Result<(), String>> + Send>> =
            match peripheral_client_request {
                PeripheralRequest::GetPheripherals => Self::get_peripherals(central, tx).boxed(),
                PeripheralRequest::GetCharacteristics(peripheral) => {
                    Self::get_characteristics(tx, peripheral).boxed()
                }
                PeripheralRequest::Read((peripheral, characteristic_uuid)) => {
                    Self::read_characteristic(central, tx, peripheral, characteristic_uuid).boxed()
                }
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

    async fn get_characteristics(
        tx: Sender<PeripheralResponse>,
        peripheral: PlatformPeripheral,
    ) -> Result<(), String> {
        let result = async {
            tx.send(PeripheralResponse::CharacteristicScanStarted)
                .await
                .map_err(|e| e.to_string())?;

            Self::connect_to_peripheral(&peripheral, &tx).await?;

            let characteristics = peripheral
                .characteristics()
                .iter()
                .cloned()
                .collect::<Vec<Characteristic>>();

            let response = PeripheralResponse::GetCharacteristics(characteristics);
            tx.send(response).await.map_err(|e| e.to_string())?;

            Ok(())
        }
        .await;

        if let Err(err) = result {
            tx.send(PeripheralResponse::CharacteristicScanError(err))
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn connect_to_peripheral(
        peripheral: &PlatformPeripheral,
        tx: &Sender<PeripheralResponse>,
    ) -> Result<(), String> {
        let _ = tx
            .send(PeripheralResponse::ScanningMessageUpdate(
                "Connecting to Peripheral".to_string(),
            ))
            .await;

        select! {
            result = peripheral.connect() => result.map_err(|e| e.to_string()),
            _ = sleep(Duration::from_secs(5)) => Err("Timed out connecting to Peripheral".to_string())
        }?;

        Ok(())
    }

    async fn read_characteristic(
        central: Central,
        tx: Sender<PeripheralResponse>,
        peripheral: PlatformPeripheral,
        characteristic_uuid: Uuid,
    ) -> Result<(), String> {
        let result = async {
            tx.send(PeripheralResponse::ReadCharacteristicCallStarted)
                .await
                .map_err(|e| e.to_string())?;

            let result = central
                .read(&peripheral, characteristic_uuid)
                .await
                .map_err(|e| e.to_string())?;

            let response = PeripheralResponse::ReadCharacteristic((characteristic_uuid, result));
            tx.send(response).await.map_err(|e| e.to_string())?;

            Ok(())
        }
        .await;

        if let Err(err) = result {
            tx.send(PeripheralResponse::CharacteristicScanError(err))
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

    pub async fn get_characteristics(&self, peripheral: &PlatformPeripheral) -> Result<(), String> {
        let request = PeripheralRequest::GetCharacteristics(peripheral.clone());
        self.send_request(request).await?;

        Ok(())
    }

    pub async fn read(
        &self,
        peripheral: PlatformPeripheral,
        characteristic_uuid: Uuid,
    ) -> Result<(), String> {
        let request = PeripheralRequest::Read((peripheral, characteristic_uuid));
        self.send_request(request).await?;
        Ok(())
    }

    async fn send_request(&self, request: PeripheralRequest) -> Result<(), String> {
        self.0.send(request).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}

pub enum KnownCharacteristic {
    Ping(Pong),
    Status(bool),
}

pub fn check_known_characteristic(
    characteristic_id: Uuid,
    data: &[u8],
) -> Result<Option<KnownCharacteristic>, String> {
    if characteristic_id == HEALTH_PING_CHAR_UUID {
        Ok(Some(KnownCharacteristic::Ping(
            <Pong as FromGatt>::from_gatt(data).map_err(|e| format!("{e:?}"))?,
        )))
    } else if characteristic_id == HEALTH_STATUS_CHAR_UUID {
        Ok(Some(KnownCharacteristic::Status(
            <bool as FromGatt>::from_gatt(data).map_err(|e| format!("{e:?}"))?,
        )))
    } else {
        Ok(None)
    }
}
