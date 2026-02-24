pub mod home;

trait Page {
    type Event;

    fn handle_event(&mut self, event: Self::Event);
    // fn handle_key_event(&mut self, key_event: KeyEvent)
}