mod app;
mod pages;
mod utils;

use app::App;

#[tokio::main]
async fn main() {
    let mut terminal = ratatui::init();
    if let Err(err) = App::new().await.unwrap().run(&mut terminal).await {
        eprintln!("{err}")
    };
    ratatui::restore();
}
