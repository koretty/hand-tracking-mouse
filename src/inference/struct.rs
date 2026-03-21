use ort::session::Session;

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

pub struct HandLandmarkSession {
    pub(super) session: Session,
    pub(super) logged_output_info: bool,
}