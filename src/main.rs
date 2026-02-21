mod app;
mod commands;
mod event;
mod pages;
mod state;

use app::App;

#[tokio::main]
async fn main() {
    let mut terminal = ratatui::init();
    if let Err(err) = App::run(&mut terminal).await {
        eprintln!("{err}")
    };
    ratatui::restore();
}
