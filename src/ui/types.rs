use minifb::Window;

pub struct PreviewWindow {
    pub title: String,
    pub window: Option<Window>,
    pub pixels: Vec<u32>,
}
