use crate::pages::home::HomePageEvent;
use futures_util::StreamExt;
use iot_sdk::{Characteristic, Peripheral, central::Central};
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
                    .peripheral_properties()
                    .await
                    .map_err(|e| e.to_string())?
                    .filter_map(|p| async move { p.local_name })
                    .take(15)
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
        &self,
        home_page_event_tx: &Sender<HomePageEvent>,
        local_name: &str,
    ) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::CharacteristicScanStarted)
            .await
            .map_err(|err| err.to_string())?;

        let tx = home_page_event_tx.clone();
        let local_name = local_name.to_string();
        let central = self.0.clone();

        tokio::spawn(async move {
            let characteristics_result = async {
                let peripheral = central
                    .find_peripheral(&local_name)
                    .await
                    .map_err(|e| e.to_string())?;

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

    // pub async fn call_characteristic(
    //     home_page_event_tx: &Sender<HomePageEvent>,
    //     characteristic: &Characteristic,
    // ) -> Result<(), String> {
    //     home_page_event_tx
    //         .send(HomePageEvent::CharacteristicScanStarted)
    //         .await
    //         .map_err(|err| err.to_string())?;

    //     let tx = home_page_event_tx.clone();
    //     let characteristic = characteristic.clone();

    //     tokio::spawn(async move {

    //         let properties = characteristic.properties;

    //         let characteristics_result = async {
    //             let central = Central::new().await.map_err(|e| e.to_string())?;
    //             let peripheral = central
    //                 .find_peripheral(&local_name)
    //                 .await
    //                 .map_err(|e| e.to_string())?;

    //             let connection_result = select! {
    //                 result = peripheral.connect() => result.map_err(|e| e.to_string()),
    //                 _ = sleep(Duration::from_secs(5)) => Err(format!("Timed out connecting to {local_name} Peripheral"))
    //             };

    //             if let Err(err) = connection_result {
    //                 Err(err)
    //             } else {
    //                 Ok(peripheral)
    //             }
    //         };
    //     });

    //     Ok(())
    // }
}

// fn get_peripheral() {}
