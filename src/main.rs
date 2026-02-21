mod app;
mod commands;
mod event;
mod pages;
mod state;

use app::App;

fn main() -> std::io::Result<()> {
    ratatui::run(App::run)
}
