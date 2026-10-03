use crossbeam::channel::{self, bounded};
use futures::FutureExt;
use futures_util::StreamExt;
use iot_sdk::{
    CharPropFlags, Characteristic, Peripheral, PlatformPeripheral, Uuid, ValueNotification,
    central::Central,
};
use services::{
    NotificationHandler, NotificationResponse, ReadHandler, ReadResponse, WriteHandler,
    WriteResponse,
    health::{HEALTH_PING_CHAR_UUID, HEALTH_STATUS_CHAR_UUID},
    storage::STORAGE_DATA_CHAR_UUID,
    trouble_host::types::gatt_traits::AsGatt,
};
use std::{fmt::Debug, pin::Pin};
use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver, Sender},
        watch,
    },
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
    Notify(
        (
            PlatformPeripheral,
            Uuid,
            channel::Sender<ValueNotification>,
            watch::Receiver<bool>,
        ),
    ),
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
                PeripheralRequest::Notify((peripheral, characteristic_id, notify_tx, kill_rx)) => {
                    Self::notify_characteristic(
                        central,
                        tx,
                        peripheral,
                        characteristic_id,
                        notify_tx,
                        kill_rx,
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
                let read_handler = ReadHandler::new(characteristic.uuid);
                let write_handler = WriteHandler::new(characteristic.uuid);
                let notification_handler = NotificationHandler::new(characteristic.uuid);

                let known_characteristic = KnownCharacteristic::new(
                    characteristic.clone(),
                    read_handler,
                    write_handler,
                    notification_handler,
                );
                known_characteristics.push(known_characteristic);
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
        kill_rx: watch::Receiver<bool>,
    ) -> Result<(), String> {
        let result = async {

            let notification_stream = select! {
                result = central.subscribe(&peripheral, characteristic_id) => result.map_err(|e| e.to_string()),
                _ = sleep(Duration::from_secs(5)) => Err(String::from("Timed out subscribing to characteristic notifications"))
            }?;

            tokio::pin!(notification_stream);
            tokio::pin!(kill_rx);

            loop {
                select! {
                    Some(notification) = notification_stream.next() => {
                        if let Err(err) = notify_tx.send(notification) {
                            return Err(err.to_string());
                        }
                    }
                    _ = kill_rx.changed() => {
                        break;
                    }
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
    ) -> Result<(channel::Receiver<ValueNotification>, watch::Sender<bool>), String> {
        let (notify_tx, notify_rx) = bounded(100);
        let (kill_tx, kill_rx) = watch::channel(false);

        let request =
            PeripheralRequest::Notify((peripheral, characteristic_id, notify_tx, kill_rx));
        self.send_request(request).await?;

        Ok((notify_rx, kill_tx))
    }

    async fn send_request(&self, request: PeripheralRequest) -> Result<(), String> {
        self.0.send(request).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KnownCharacteristic {
    display_name: String,
    characteristic: Characteristic,
    read_handler: ReadHandler,
    write_handler: WriteHandler,
    notification_handler: NotificationHandler,
}

impl KnownCharacteristic {
    fn new(
        characteristic: Characteristic,
        read_handler: ReadHandler,
        write_handler: WriteHandler,
        notification_handler: NotificationHandler,
    ) -> Self {
        let display_name = if characteristic.uuid == HEALTH_STATUS_CHAR_UUID {
            String::from("Status")
        } else if characteristic.uuid == HEALTH_PING_CHAR_UUID {
            String::from("Ping")
        } else if characteristic.uuid == STORAGE_DATA_CHAR_UUID {
            String::from("Data")
        } else {
            String::from("")
        };

        Self {
            display_name,
            characteristic,
            read_handler,
            write_handler,
            notification_handler,
        }
    }

    pub fn properties(&self) -> &CharPropFlags {
        &self.characteristic.properties
    }

    pub fn id(&self) -> Uuid {
        self.characteristic.uuid
    }

    pub fn display_characteristic_properties(&self) -> String {
        format!(
            "{} {}, {:?}",
            self.display_name,
            self.id(),
            self.properties()
        )
    }

    pub fn handle_response(&self, data: &[u8]) -> Result<String, String> {
        match self.read_handler.deserialize(data) {
            ReadResponse::Other => String::from_utf8(data.to_vec()).map_err(|e| e.to_string()),
            ReadResponse::Status(status) => Ok(format!("{}", status)),
            ReadResponse::Data(data) => Ok(format!("{}", data)),
        }
    }

    pub fn serialize_write(&self, data: String) -> Result<Vec<u8>, String> {
        let data = data.as_bytes();

        match self.write_handler.serialize(data) {
            WriteResponse::Other => Ok(data.to_vec()),
            WriteResponse::Data(data) => Ok(data.as_gatt().to_vec()),
        }
    }

    pub fn handle_notification(&self, data: &[u8]) -> Result<String, String> {
        match self.notification_handler.deserialize(data) {
            NotificationResponse::Other => {
                String::from_utf8(data.to_vec()).map_err(|e| e.to_string())
            }
            NotificationResponse::Ping(pong) => Ok(format!("{}", pong)),
        }
    }
}
