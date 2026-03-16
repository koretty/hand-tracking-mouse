use std::time::Instant;

pub struct FpsCounter {
    report_started: Instant,
    frame_count: u32,
    current_fps: f64,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            report_started: Instant::now(),
            frame_count: 0,
            current_fps: 0.0,
        }
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;
        let elapsed = self.report_started.elapsed().as_secs_f64();

        // 0.25秒ごとに更新して表示の追従性と安定性を両立する
        if elapsed >= 0.25 {
            self.current_fps = self.frame_count as f64 / elapsed;
            self.frame_count = 0;
            self.report_started = Instant::now();
        }
    }

    pub fn current_fps(&self) -> f64 {
        self.current_fps
    }
}
