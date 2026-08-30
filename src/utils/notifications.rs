#[derive(Clone, Debug)]
pub struct Notifications {
    status: bool,
    notifications: Vec<String>,
}

impl Notifications {
    pub fn is_channel_empty(&self) -> bool {
        self.status
    }

    pub fn update_empty_status(&mut self, status: bool) {
        self.status = status
    }

    pub fn update_notifications(&mut self, notification: String) {
        self.notifications.push(notification);
    }

    pub fn notifications(&self) -> impl Iterator<Item = &str> {
        self.notifications.iter().map(|n| n.as_str())
    }
}


impl Default for Notifications {
    fn default() -> Self {
        Self {
            status: true,
            notifications: Vec::new()
        }
    }
}