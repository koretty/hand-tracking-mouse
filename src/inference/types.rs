pub const MODEL_INPUT_WIDTH: usize = 256;
pub const MODEL_INPUT_HEIGHT: usize = 256;
pub const LANDMARK_COUNT: usize = 21;

#[derive(Debug, Clone, Copy)]
pub struct RoiRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Landmark3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
