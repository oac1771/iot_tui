mod app;
mod commands;
mod pages;
mod state;
mod util;

use app::App;

#[tokio::main]
async fn main() {
    let mut terminal = ratatui::init();
    if let Err(err) = App::new().run(&mut terminal).await {
        eprintln!("{err}")
    };
    ratatui::restore();
}
