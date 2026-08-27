use crossbeam::channel::{self, bounded};
use futures::FutureExt;
use futures_util::StreamExt;
use iot_sdk::{
    CharPropFlags, Characteristic, Peripheral, PlatformPeripheral, Uuid, ValueNotification,
    central::Central,
};
use services::{
    Foo,
    health::{
        HEALTH_PING_CHAR_UUID, HEALTH_PING_DESCRIPTOR_UUID, HEALTH_STATUS_CHAR_UUID,
        HEALTH_STATUS_DESCRIPTOR_UUID, HealthServicePingDescriptor, HealthServiceStatusDescriptor,
    },
    storage::{STORAGE_DATA_CHAR_UUID, STORAGE_DATA_DESCRIPTOR_UUID, StorageServiceDataDescriptor},
    trouble_host::types::gatt_traits::AsGatt,
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
    Write((PlatformPeripheral, Uuid, Vec<u8>)),
    Notify((PlatformPeripheral, Uuid, channel::Sender<ValueNotification>)),
}

#[derive(Debug)]
pub enum PeripheralResponse {
    PeripheralScanStarted,
    GetPheripherals(Vec<PlatformPeripheral>),
    CharacteristicScanStarted,
    ScanningMessageUpdate(String),
    GetCharacteristics(Vec<KnownCharacteristic>),
    ReadCharacteristicCallStarted,
    ReadCharacteristic((Uuid, Vec<u8>)),
    WriteCharacteristicCallStarted,
    WriteCharacteristic,
    Error((ResponseType, String)),
}

#[derive(Debug)]
pub enum ResponseType {
    Peripheral,
    Characteristic,
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
                PeripheralRequest::Read((peripheral, characteristic_id)) => {
                    Self::read_characteristic(central, tx, peripheral, characteristic_id).boxed()
                }
                PeripheralRequest::Write((peripheral, characteristic_id, data)) => {
                    Self::write_characteristic(central, tx, peripheral, characteristic_id, data)
                        .boxed()
                }
                PeripheralRequest::Notify((peripheral, characteristic_id, notify_tx)) => {
                    Self::notify_characteristic(
                        central,
                        tx,
                        peripheral,
                        characteristic_id,
                        notify_tx,
                    )
                    .boxed()
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

        if let Err(err) = result.map_err(|e| (ResponseType::Peripheral, e)) {
            tx.send(PeripheralResponse::Error(err))
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
                let c = characteristic.clone();
                let descriptors = c
                    .descriptors
                    .iter()
                    .filter_map(|d| KnownDescriptor::try_from(d.uuid).ok());

                let known_characteristic = KnownCharacteristic::new(characteristic, descriptors);
                known_characteristics.push(known_characteristic)
            }

            let response = PeripheralResponse::GetCharacteristics(known_characteristics);
            tx.send(response).await.map_err(|e| e.to_string())?;

            Ok(())
        }
        .await;

        if let Err(err) = result.map_err(|e| (ResponseType::Characteristic, e)) {
            tx.send(PeripheralResponse::Error(err))
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
        characteristic_id: Uuid,
    ) -> Result<(), String> {
        let result = async {
            tx.send(PeripheralResponse::ReadCharacteristicCallStarted)
                .await
                .map_err(|e| e.to_string())?;

            let result = select! {
                result = central.read(&peripheral, characteristic_id) => result.map_err(|e| e.to_string()),
                _ = sleep(Duration::from_secs(5)) => Err("Timed out reading characteristic value".to_string())
            }?;

            let response = PeripheralResponse::ReadCharacteristic((characteristic_id, result));
            tx.send(response).await.map_err(|e| e.to_string())?;

            Ok(())
        }
        .await;

        if let Err(err) = result.map_err(|e| (ResponseType::Characteristic, e)) {
            tx.send(PeripheralResponse::Error(err))
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn write_characteristic(
        central: Central,
        tx: Sender<PeripheralResponse>,
        peripheral: PlatformPeripheral,
        characteristic_id: Uuid,
        data: Vec<u8>,
    ) -> Result<(), String> {
        let result = async {
            tx.send(PeripheralResponse::WriteCharacteristicCallStarted)
                .await
                .map_err(|e| e.to_string())?;

            select! {
                result = central.write(&peripheral, characteristic_id, &data) => result.map_err(|e| e.to_string()),
                _ = sleep(Duration::from_secs(5)) => Err(format!("Timed out writing: {:?} to characteristic", data))
            }?;

            let response = PeripheralResponse::WriteCharacteristic;
            tx.send(response).await.map_err(|e| e.to_string())?;

            Ok(())
        }
        .await;

        if let Err(err) = result.map_err(|e| (ResponseType::Characteristic, e)) {
            tx.send(PeripheralResponse::Error(err))
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn notify_characteristic(
        central: Central,
        tx: Sender<PeripheralResponse>,
        peripheral: PlatformPeripheral,
        characteristic_id: Uuid,
        notify_tx: channel::Sender<ValueNotification>,
    ) -> Result<(), String> {
        let result = async {

            let notification_stream = select! {
                result = central.subscribe(&peripheral, characteristic_id) => result.map_err(|e| e.to_string()),
                _ = sleep(Duration::from_secs(5)) => Err(String::from("Timed out subscribing to characteristic notifications"))
            }?;

            tokio::pin!(notification_stream);

            while let Some(notification) = notification_stream.next().await {
                if let Err(err) = notify_tx.send(notification) {
                    return Err(err.to_string())
                }
            }

            Ok(())
        }
        .await;

        if let Err(err) = result.map_err(|e| (ResponseType::Characteristic, e)) {
            tx.send(PeripheralResponse::Error(err))
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
        characteristic_id: Uuid,
    ) -> Result<(), String> {
        let request = PeripheralRequest::Read((peripheral, characteristic_id));
        self.send_request(request).await?;
        Ok(())
    }

    pub async fn write(
        &self,
        peripheral: PlatformPeripheral,
        characteristic_id: Uuid,
        data: &[u8],
    ) -> Result<(), String> {
        let request = PeripheralRequest::Write((peripheral, characteristic_id, data.to_vec()));
        self.send_request(request).await?;
        Ok(())
    }

    pub async fn notify(
        &self,
        peripheral: PlatformPeripheral,
        characteristic_id: Uuid,
    ) -> Result<channel::Receiver<ValueNotification>, String> {
        let (notify_tx, notify_rx) = bounded(100);

        let request = PeripheralRequest::Notify((peripheral, characteristic_id, notify_tx));
        self.send_request(request).await?;

        Ok(notify_rx)
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
pub enum CharacteristicType {
    Ping,
    Status,
    Storage,
    Unknown,
}

impl KnownCharacteristic {
    pub fn new(
        characteristic: Characteristic,
        descriptors: impl Iterator<Item = KnownDescriptor>,
    ) -> Self {
        let characteristic_type = if characteristic.uuid == HEALTH_STATUS_CHAR_UUID {
            CharacteristicType::Status
        } else if characteristic.uuid == HEALTH_PING_CHAR_UUID {
            CharacteristicType::Ping
        } else if characteristic.uuid == STORAGE_DATA_CHAR_UUID {
            CharacteristicType::Storage
        } else {
            CharacteristicType::Unknown
        };

        Self {
            characteristic,
            descriptors: descriptors.collect(),
            characteristic_type,
        }
    }

    pub fn characteristic_type(&self) -> &CharacteristicType {
        &self.characteristic_type
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

    pub fn handle_response(&self, data: &[u8]) -> Result<String, String> {
        if let Some(response) = self
            .descriptors
            .iter()
            .filter_map(|d| d.handle_response(data).ok())
            .next()
        {
            Ok(response)
        } else if let Ok(response) = String::from_utf8(data.to_vec()) {
            Ok(response)
        } else {
            Err(format!("Unable to deserialize response from: {:?}", data))
        }
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

    pub fn validate_write_data(&self, data: String) -> Result<Vec<u8>, String> {
        for descriptor in self.descriptors() {
            if let Ok(write_data) = descriptor.validate_write_data(&data) {
                return Ok(write_data);
            }
        }

        Err(format!(
            "Could not validate write data: {:?}\nDescriptor: {:?}",
            data,
            self.descriptors().collect::<Vec<&KnownDescriptor>>()
        ))
    }
}

#[derive(Debug, Clone)]
pub enum KnownDescriptor {
    Status(HealthServiceStatusDescriptor),
    Ping(HealthServicePingDescriptor),
    Storage(StorageServiceDataDescriptor),
}

impl KnownDescriptor {
    pub fn metadata(&self) -> String {
        match self {
            KnownDescriptor::Ping(d) => format!("Ping: {:?}", d.id()),
            KnownDescriptor::Status(d) => format!("Status: {:?}", d.id()),
            KnownDescriptor::Storage(d) => format!("Storage: {:?}", d.id()),
        }
    }

    fn validate_write_data(&self, data: &str) -> Result<Vec<u8>, String> {
        match self {
            KnownDescriptor::Ping(d) => Ok(d.serialize_write_data(()).as_gatt().to_vec()),
            KnownDescriptor::Status(d) => Ok(d.serialize_write_data(()).as_gatt().to_vec()),
            KnownDescriptor::Storage(d) => {
                let write_data = string_to_u8_bytes(data).map_err(|e| e.to_string())?;

                Ok(d.serialize_write_data(write_data).as_gatt().to_vec())
            }
        }
    }

    pub fn handle_response(&self, data: &[u8]) -> Result<String, String> {
        match self {
            KnownDescriptor::Ping(d) => d
                .deserialize_read_response(data)
                .map(|i| i.to_string())
                .map_err(|e| format!("{:?}", e)),
            KnownDescriptor::Status(d) => d
                .deserialize_read_response(data)
                .map(|i| i.to_string())
                .map_err(|e| format!("{:?}", e)),
            KnownDescriptor::Storage(d) => d
                .deserialize_read_response(data)
                .map(|i| i.to_string())
                .map_err(|e| format!("{:?}", e)),
        }
    }
}

impl TryFrom<Uuid> for KnownDescriptor {
    type Error = String;

    fn try_from(value: Uuid) -> Result<Self, Self::Error> {
        if value == STORAGE_DATA_DESCRIPTOR_UUID {
            Ok(Self::Storage(StorageServiceDataDescriptor))
        } else if value == HEALTH_PING_DESCRIPTOR_UUID {
            Ok(Self::Ping(HealthServicePingDescriptor))
        } else if value == HEALTH_STATUS_DESCRIPTOR_UUID {
            Ok(Self::Status(HealthServiceStatusDescriptor))
        } else {
            Err(String::from("Not known descriptor"))
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

fn string_to_u8_bytes(input: &str) -> Result<[u8; 1], std::num::ParseIntError> {
    let value: u8 = input.parse()?;
    Ok(value.to_le_bytes())
}
