mod app;
mod camera;
mod config;
mod inference;
mod pipeline;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
