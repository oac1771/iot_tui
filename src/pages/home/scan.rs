use std::collections::HashSet;

use crate::pages::home::HomePageEvent;
use futures_util::StreamExt;
use iot_sdk::central::Central;
use tokio::sync::mpsc::Sender;

pub struct ScanCmd;

impl ScanCmd {
    pub async fn handle(home_page_event_tx: &Sender<HomePageEvent>) -> Result<(), String> {
        home_page_event_tx
            .send(HomePageEvent::Pending)
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
                    .filter_map(|p| async move {
                        match p.local_name {
                            Some(local_name) => Some((local_name, p.address.to_string())),
                            None => None,
                        }
                    })
                    .take(5)
                    .collect::<HashSet<(String, String)>>()
                    .await;

                Ok(peripherals)
            }
            .await;

            let event = match result {
                Ok(peripherals) => HomePageEvent::Complete(peripherals),
                Err(err) => HomePageEvent::Error(err),
            };

            let _ = tx.send(event).await;
        });

        Ok(())
    }
}
