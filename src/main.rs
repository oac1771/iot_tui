mod app;
mod pages;
mod utils;

use app::App;

#[tokio::main]
async fn main() {
    let mut terminal = ratatui::init();
    match App::new().await {
        Ok(app) => {
            if let Err(err) = app.run(&mut terminal).await {
                eprintln!("{err}")
            };
        }
        Err(err) => eprintln!("{err}"),
    }
    ratatui::restore();
}
