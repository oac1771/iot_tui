tutorials: https://ratatui.rs/tutorials/

todo:

use this pattern in impl App run:

tokio::select! {
    // Keyboard events
    maybe_event = reader.next() => {
        if let Some(Ok(CrosstermEvent::Key(key_event))) = maybe_event {
            self.handle_key_event(&key_event).await;
        }
    }

    // Custom app events
    maybe_app_event = self.rx.recv() => {
        if let Some(app_event) = maybe_app_event {
            self.handle_app_event(app_event);
        }
    }
}

impl App
pub fn new() -> Self {
    let (tx, rx) = mpsc::channel(32);

    Self {
        home_page: HomePage::new(tx),
        active_page: PageKind::Home,
        exit: false,
        rx,
    }
}
