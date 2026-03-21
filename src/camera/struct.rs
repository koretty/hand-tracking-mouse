use nokhwa::{utils::CameraIndex, Camera};

#[derive(Clone, Debug)]
pub struct CameraDevice {
    pub display_name: String,
    pub index: CameraIndex,
}

pub struct CameraSession {
    pub(super) camera: Camera,
}