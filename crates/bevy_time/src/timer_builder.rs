use bevy_utils::Duration;

use crate::{Timer, TimerMode};

#[derive(Default)]
pub struct TimerBuilder {
    initially_paused: bool,
    duration: Duration,
    mode: TimerMode,
}

impl TimerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn duration(self, duration: Duration) -> Self {
        Self { duration, ..self }
    }

    pub fn seconds(self, duration: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(duration),
            ..self
        }
    }

    pub fn mode(self, mode: TimerMode) -> Self {
        Self { mode, ..self }
    }

    pub fn repeating(self) -> Self {
        Self {
            mode: TimerMode::Repeating,
            ..self
        }
    }

    pub fn paused(self) -> Self {
        Self {
            initially_paused: true,
            ..self
        }
    }

    pub fn build(self) -> Timer {
        let mut timer = Timer::new(self.duration, self.mode);

        if self.initially_paused {
            timer.pause();
        }

        timer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_builder() {
        let t = TimerBuilder::new().seconds(10.0).build();
        assert_eq!(t.duration(), Duration::from_secs_f32(10.0));
        assert_eq!(t.mode(), TimerMode::Once);
        assert!(!t.paused());

        let t = TimerBuilder::new()
            .duration(Duration::from_millis(500))
            .repeating()
            .paused()
            .build();

        assert_eq!(t.duration(), Duration::from_secs_f32(0.5));
        assert_eq!(t.mode(), TimerMode::Repeating);
        assert!(t.paused());
    }
}
