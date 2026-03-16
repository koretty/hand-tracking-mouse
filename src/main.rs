mod app;
mod camera;
mod config;
mod pipeline;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
