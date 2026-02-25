const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Default)]
pub struct Spinner {
    index: usize,
}

impl Spinner {
    pub fn tick(&mut self) {
        self.index = (self.index + 1) % SPINNER_FRAMES.len();
    }

    pub fn frame(&self) -> &'static str {
        SPINNER_FRAMES[self.index]
    }
}
