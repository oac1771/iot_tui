use crate::pages::home::HomePageEvent;
use futures_util::StreamExt;
use iot_sdk::{Characteristic, Peripheral, central::Central};
use tokio::{
    select,
    sync::mpsc::Sender,
    time::{Duration, sleep},
};

pub struct Peripherals;

impl Peripherals {
    pub async fn get_peripherals(home_page_event_tx: &Sender<HomePageEvent>) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::PeripheralScanPending)
            .await
            .map_err(|err| err.to_string())?;

        let tx = home_page_event_tx.clone();

        tokio::spawn(async move {
            let result = async {
                let central = Central::new().await.map_err(|e| e.to_string())?;

                let peripherals = central
                    .peripheral_properties()
                    .await
                    .map_err(|e| e.to_string())?
                    .filter_map(|p| async move { p.local_name })
                    .take(5)
                    .collect::<Vec<String>>()
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
        home_page_event_tx: &Sender<HomePageEvent>,
        local_name: &str,
    ) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::CharacteristicScanPending)
            .await
            .map_err(|err| err.to_string())?;

        let tx = home_page_event_tx.clone();
        let local_name = local_name.to_string();

        tokio::spawn(async move {
            let characteristics_result = async {
                let central = Central::new().await.map_err(|e| e.to_string())?;
                let peripheral = central
                    .find_peripheral(&local_name)
                    .await
                    .map_err(|e| e.to_string())?;

                let _ = tx
                    .send(HomePageEvent::CharacteristicScanPeripheralFound)
                    .await;

                let connection_result = select! {
                    result = peripheral.connect() => result.map_err(|e| e.to_string()),
                    _ = sleep(Duration::from_secs(5)) => Err(format!("Timed out connecting to {local_name} Peripheral"))
                };

                if let Err(err) = connection_result {
                    Err(err)
                } else {
                    let characteristics = peripheral
                        .characteristics()
                        .iter()
                        .cloned()
                        .collect::<Vec<Characteristic>>();

                    Ok(characteristics)
                }
            };

            let result = select! {
                result = characteristics_result => result,
                _ = sleep(Duration::from_secs(10)) => Err(format!("Timed out Loading characteristics metadata for {local_name} Peripheral"))
            };

            let event = match result {
                Ok(characteristics) => HomePageEvent::CharacteristicScanComplete(characteristics),
                Err(err) => HomePageEvent::CharacteristicScanError(err),
            };

            let _ = tx.send(event).await;
        });

        Ok(())
    }
}
