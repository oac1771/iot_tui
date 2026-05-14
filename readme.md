tutorials: https://ratatui.rs/tutorials/

        tokio::spawn(async move {
            let result = async {
                let peripheral = get_peripheral(local_name, central).await?;

                let characteristics = peripheral
                    .characteristics()
                    .iter()
                    .cloned()
                    .collect::<Vec<Characteristic>>();

                Ok(characteristics)
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

create peripheral that runs in tokio spawn with Peripheral client that the pages take as argument