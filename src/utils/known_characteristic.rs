use iot_sdk::{CharPropFlags, Characteristic, Uuid};
use services::{
    NotificationHandler, NotificationResponse, ReadHandler, ReadResponse, WriteHandler,
    WriteResponse,
    health::{HEALTH_PING_CHAR_UUID, HEALTH_STATUS_CHAR_UUID},
    storage::STORAGE_DATA_CHAR_UUID,
    trouble_host::types::gatt_traits::AsGatt,
};

#[derive(Debug, Clone)]
pub struct KnownCharacteristic {
    display_name: String,
    characteristic: Characteristic,
    read_handler: ReadHandler,
    write_handler: WriteHandler,
    notification_handler: NotificationHandler,
}

impl KnownCharacteristic {
    pub fn new(
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
        match self
            .read_handler
            .deserialize(data)
            .map_err(|e| format!("{:?}", e))?
        {
            ReadResponse::Other => String::from_utf8(data.to_vec()).map_err(|e| e.to_string()),
            ReadResponse::Status(status) => Ok(format!("{}", status)),
            ReadResponse::Data(data) => Ok(format!("{}", data)),
        }
    }

    pub fn serialize_write(&self, data: String) -> Result<Vec<u8>, String> {
        let data = data.as_bytes();

        match self
            .write_handler
            .serialize(data)
            .map_err(|_| String::from("WriteError"))?
        {
            WriteResponse::Other => Ok(data.to_vec()),
            WriteResponse::Data(data) => Ok(data.as_gatt().to_vec()),
        }
    }

    pub fn handle_notification(&self, data: &[u8]) -> Result<String, String> {
        match self
            .notification_handler
            .deserialize(data)
            .map_err(|e| format!("{:?}", e))?
        {
            NotificationResponse::Other => {
                String::from_utf8(data.to_vec()).map_err(|e| e.to_string())
            }
            NotificationResponse::Ping(pong) => Ok(format!("{}", pong)),
        }
    }
}
