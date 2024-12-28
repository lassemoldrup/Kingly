use std::time::Duration;

#[derive(Clone, Copy, Default)]
pub struct TimeControl {
    pub time_remaining: Duration,
    pub increment: Duration,
}

impl TimeControl {
    pub fn time_man(&self) -> Duration {
        self.time_remaining / 20 + self.increment / 2
    }
}
