use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_utils::Duration;

use crate::{context::Context, Stopwatch, Time, TimeTracker};

#[derive(Component)]
pub struct UpdatingStopwatch<T> {
    pub watch: Stopwatch,
    tracking: PhantomData<T>,
}

impl<T> UpdatingStopwatch<T> {
    pub fn new(watch: Stopwatch) -> Self {
        Self {
            watch,
            tracking: PhantomData,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.watch.elapsed()
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.watch.elapsed_secs()
    }

    pub fn elapsed_secs_f64(&self) -> f64 {
        self.watch.elapsed_secs_f64()
    }

    pub fn pause(&mut self) {
        self.watch.pause()
    }

    pub fn unpause(&mut self) {
        self.watch.unpause()
    }

    pub fn paused(&self) -> bool {
        self.watch.paused()
    }

    pub fn reset(&mut self) {
        self.watch.reset()
    }
}

impl<C: Context + Default + Send + Sync + 'static> TimeTracker for UpdatingStopwatch<C> {
    type Time = Time<C>;

    fn update(
        &mut self,
        time: &<<Self::Time as crate::context::TimesWithContext>::AsSystemParam<'_> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
    ) {
        self.watch.tick(time.delta());
    }
}

impl<T> Clone for UpdatingStopwatch<T> {
    fn clone(&self) -> Self {
        Self {
            watch: self.watch.clone(),
            tracking: self.tracking.clone(),
        }
    }
}

impl<T> Debug for UpdatingStopwatch<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdatingStopWatch")
            .field("watch", &self.watch)
            .field("tracking", &self.tracking)
            .finish()
    }
}

impl<T> Default for UpdatingStopwatch<T> {
    fn default() -> Self {
        Self {
            watch: Default::default(),
            tracking: Default::default(),
        }
    }
}

impl<T> PartialEq for UpdatingStopwatch<T> {
    fn eq(&self, other: &Self) -> bool {
        self.watch == other.watch && self.tracking == other.tracking
    }
}

impl<T> Eq for UpdatingStopwatch<T> {}
