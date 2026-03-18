use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

pub trait FrameProcessor {
    fn process(&mut self, frame: Frame) -> Result<Frame>;
}
