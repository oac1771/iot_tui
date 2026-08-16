use futures::FutureExt;
use futures_util::StreamExt;
use iot_sdk::{
    CharPropFlags, Characteristic, Peripheral, PlatformPeripheral, Uuid, central::Central,
};
use services::{
    health::{
        HEALTH_PING_CHAR_UUID, HEALTH_STATUS_CHAR_UUID, HealthServicePingDescriptor,
        HealthServiceStatusDescriptor,
    },
    storage::STORAGE_STATUS_CHAR_UUID,
    trouble_host::types::gatt_traits::{AsGatt, FromGatt, FromGattError},
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
    Read((PlatformPeripheral, KnownCharacteristic)),
}

#[derive(Debug)]
pub enum PeripheralResponse {
    PeripheralScanStarted,
    GetPheripherals(Vec<PlatformPeripheral>),
    PeripheralScanError(String),
    CharacteristicScanStarted,
    ScanningMessageUpdate(String),
    GetCharacteristics(Vec<KnownCharacteristic>),
    CharacteristicScanError(String),
    ReadCharacteristicCallStarted,
    ReadCharacteristic((KnownCharacteristic, Vec<u8>)),
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
                PeripheralRequest::Read((peripheral, characteristic)) => {
                    Self::read_characteristic(central, tx, peripheral, characteristic).boxed()
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

            let mut known_characteristics = Vec::new();

            for characteristic in peripheral.characteristics().into_iter() {
                let mut raw_descriptors = Vec::new();

                for descriptor in characteristic.descriptors.iter() {
                    let raw_descriptor = select! {
                        result = peripheral.read_descriptor(descriptor) => result.map_err(|e| e.to_string()),
                        _ = sleep(Duration::from_secs(5)) => Err("Timed out reading characteristic descriptor".to_string())
                    }?;

                    raw_descriptors.push(raw_descriptor);
                }

                let known_characteristic = KnownCharacteristic::new(characteristic, raw_descriptors.into_iter());
                known_characteristics.push(known_characteristic)
            }

            let response = PeripheralResponse::GetCharacteristics(known_characteristics);
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
        characteristic: KnownCharacteristic,
    ) -> Result<(), String> {
        let result = async {
            tx.send(PeripheralResponse::ReadCharacteristicCallStarted)
                .await
                .map_err(|e| e.to_string())?;

            let result = select! {
                result = central.read(&peripheral, characteristic.id()) => result.map_err(|e| e.to_string()),
                _ = sleep(Duration::from_secs(5)) => Err("Timed out reading characteristic value".to_string())
            }?;

            let response = PeripheralResponse::ReadCharacteristic((characteristic, result));
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
        characteristic: &KnownCharacteristic,
    ) -> Result<(), String> {
        let request = PeripheralRequest::Read((peripheral, characteristic.clone()));
        self.send_request(request).await?;
        Ok(())
    }

    async fn send_request(&self, request: PeripheralRequest) -> Result<(), String> {
        self.0.send(request).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KnownCharacteristic {
    characteristic: Characteristic,
    descriptors: Vec<KnownDescriptor>,
    characteristic_type: CharacteristicType,
}

#[derive(Debug, Clone)]
enum CharacteristicType {
    Ping,
    Status,
    Storage,
    Unknown,
}

impl KnownCharacteristic {
    pub fn new(characteristic: Characteristic, descriptors: impl Iterator<Item = Vec<u8>>) -> Self {
        let descriptors = descriptors
            .map(|descriptor_data| KnownDescriptor::from_gatt(&descriptor_data).unwrap())
            .collect();

        let characteristic_type = if characteristic.uuid == HEALTH_STATUS_CHAR_UUID {
            CharacteristicType::Status
        } else if characteristic.uuid == HEALTH_PING_CHAR_UUID {
            CharacteristicType::Ping
        } else if characteristic.uuid == STORAGE_STATUS_CHAR_UUID {
            CharacteristicType::Storage
        } else {
            CharacteristicType::Unknown
        };

        Self {
            characteristic,
            descriptors,
            characteristic_type,
        }
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &KnownDescriptor> {
        self.descriptors.iter()
    }

    pub fn properties(&self) -> &CharPropFlags {
        &self.characteristic.properties
    }

    pub fn id(&self) -> Uuid {
        self.characteristic.uuid
    }

    pub fn to_inner_string(&self, data: &[u8]) -> Result<String, String> {
        // match self characteristic type with espected descriptor response handler or unknown -> unknown
        match self.characteristic_type {
            CharacteristicType::Ping => {}
            CharacteristicType::Status => {}
            CharacteristicType::Storage => {
                let foo = self.descriptors().map(|d| {});
            }
            CharacteristicType::Unknown => {
                return Ok(String::from_utf8(data.to_vec())
                    .unwrap_or(String::from("Unable to deserialize")));
            }
        }
        Ok(String::from("foo bar"))
    }

    pub fn display_characteristic_properties(&self) -> String {
        match self.characteristic_type {
            CharacteristicType::Ping => format!("Ping: {}, {:?}", self.id(), self.properties()),
            CharacteristicType::Status => format!("Status: {}, {:?}", self.id(), self.properties()),
            CharacteristicType::Storage => {
                format!("Storage: {}, {:?}", self.id(), self.properties())
            }
            CharacteristicType::Unknown => format!("ID: {}, {:?}", self.id(), self.properties()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum KnownDescriptor {
    Status(HealthServiceStatusDescriptor),
    Ping(HealthServicePingDescriptor),
    Unknown(Vec<u8>),
}

impl KnownDescriptor {
    pub fn metadata(&self) -> String {
        match self {
            KnownDescriptor::Ping(_) => String::from("Ping"),
            KnownDescriptor::Status(_) => String::from("Status"),
            KnownDescriptor::Unknown(d) => {
                String::from_utf8(d.to_vec()).unwrap_or(String::from(""))
            }
        }
    }
}

impl FromGatt for KnownDescriptor {
    fn from_gatt(data: &[u8]) -> Result<Self, FromGattError> {
        if data.is_empty() {
            Ok(KnownDescriptor::Unknown(data.to_vec()))
        } else if HealthServiceStatusDescriptor::from_gatt(data).is_ok() {
            Ok(KnownDescriptor::Status(HealthServiceStatusDescriptor))
        } else if HealthServicePingDescriptor::from_gatt(data).is_ok() {
            Ok(KnownDescriptor::Ping(HealthServicePingDescriptor))
        } else {
            Ok(KnownDescriptor::Unknown(data.to_vec()))
        }
    }
}

impl AsGatt for KnownDescriptor {
    const MIN_SIZE: usize = core::mem::size_of::<usize>();
    const MAX_SIZE: usize = core::mem::size_of::<usize>();

    fn as_gatt(&self) -> &[u8] {
        &[]
    }
}
