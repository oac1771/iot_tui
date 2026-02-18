mod page;
mod state;

use page::Page;

fn main() -> std::io::Result<()> {
    ratatui::run(|terminal| Page::new().run(terminal))
}
