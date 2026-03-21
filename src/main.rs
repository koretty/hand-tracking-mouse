mod app;
mod camera;
mod preferences;
mod inference;
mod pipeline;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
