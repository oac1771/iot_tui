use crate::pages::home::HomePageEvent;
use futures_util::StreamExt;
use iot_sdk::{Characteristic, Peripheral, PlatformPeripheral, central::Central};
use tokio::{
    select,
    sync::mpsc::Sender,
    time::{Duration, sleep},
};

pub struct Peripherals(Central);

impl Peripherals {
    pub async fn new() -> Result<Self, String> {
        let central = Central::new().await.map_err(|e| e.to_string())?;
        Ok(Self(central))
    }

    pub async fn get_peripherals(
        &self,
        home_page_event_tx: &Sender<HomePageEvent>,
    ) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::PeripheralScanStarted)
            .await
            .map_err(|err| err.to_string())?;

        let tx = home_page_event_tx.clone();
        let central = self.0.clone();

        tokio::spawn(async move {
            let result = async {
                let peripherals = central
                    .peripherals()
                    .await
                    .map_err(|e| e.to_string())?
                    .take(15)
                    .collect::<Vec<PlatformPeripheral>>()
                    .await;

                Ok(peripherals)
            }
            .await;

            let event = match result {
                Ok(peripherals) => HomePageEvent::PeripheralScanComplete(peripherals),
                Err(err) => HomePageEvent::PeripheralScanError(err),
            };

            let _ = tx.send(event).await;
        });

        Ok(())
    }

    pub async fn get_characteristics(
        &self,
        home_page_event_tx: &Sender<HomePageEvent>,
        peripheral: &PlatformPeripheral,
    ) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::CharacteristicScanStarted)
            .await
            .map_err(|err| err.to_string())?;

        let tx = home_page_event_tx.clone();
        let peripheral = peripheral.clone();

        tokio::spawn(async move {
            let characteristics_result = async {
                let peripheral = get_peripheral(peripheral, &tx).await?;

                let characteristics = peripheral
                    .characteristics()
                    .iter()
                    .cloned()
                    .collect::<Vec<Characteristic>>();

                Ok(characteristics)
            };

            let event = match characteristics_result.await {
                Ok(characteristics) => HomePageEvent::CharacteristicScanComplete(characteristics),
                Err(err) => HomePageEvent::CharacteristicScanError(err),
            };

            let _ = tx.send(event).await;
        });

        Ok(())
    }

    pub async fn _call_characteristic(
        &self,
        _local_name: &str,
        home_page_event_tx: &Sender<HomePageEvent>,
        characteristic: &Characteristic,
    ) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::CharacteristicScanStarted)
            .await
            .map_err(|err| err.to_string())?;

        // let tx = home_page_event_tx.clone();
        let characteristic = characteristic.clone();
        // let local_name = local_name.to_string();
        // let central = self.0.clone();

        tokio::spawn(async move {
            let _properties = characteristic.properties;

            let _call_result = async { Ok::<(), String>(()) };
        });

        Ok(())
    }
}

async fn get_peripheral(
    peripheral: PlatformPeripheral,
    tx: &Sender<HomePageEvent>,
) -> Result<PlatformPeripheral, String> {
    let _ = tx
        .send(HomePageEvent::ScanningMessageUpdate(
            "Connecting to Peripheral".to_string(),
        ))
        .await;

    select! {
        result = peripheral.connect() => result.map_err(|e| e.to_string()),
        _ = sleep(Duration::from_secs(5)) => Err("Timed out connecting to Peripheral".to_string())
    }?;

    Ok(peripheral)
}
