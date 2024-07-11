use bevy_app::App;
use bevy_ecs::{
    component::Component,
    system::{Query, StaticSystemParam, SystemParam},
};
use bevy_utils::all_tuples;

use crate::{
    context::{Context, TimesWithContext},
    UpdatingStopwatch, UpdatingTimer,
};

/// Defines a component that can be registered to automatically track time.
///
/// Register this type into an [`App`] using [`register_time_tracker`](TimeTrackingAppExtension::register_time_tracker) to make it update automatically.
pub trait TimeTracker {
    /// The time source this time tracker tracker tracks in [`update`](TimeTracker::update).
    type Time: TimesWithContext;

    /// Updates this time tracker with the data from the [`UpdateSource`](TimeTracker::UpdateSource).
    ///
    /// If this time tracker was registered with [register_time_tracker](TimeTrackingAppExtension::register_time_tracker) this will be called in the [`First`] schedule.
    fn update(
        &mut self,
        time: &<<Self::Time as TimesWithContext>::AsSystemParam<'_> as SystemParam>::Item<'_, '_>,
    );

    /// Signals to the time tracker that we've entered [`FixedUpdate`](crate::FixedUpdate).
    ///
    /// Will only be called for time trackers that depend on [`Time<Fixed>`](crate::Fixed).
    fn enter_fixed_update(&mut self) {}

    /// Signals to the time tracker that we've exited [`FixedUpdate`](crate::FixedUpdate).
    ///
    /// Will only be called for time trackers that depend on [`Time<Fixed>`](crate::Fixed).
    fn exit_fixed_update(&mut self) {}
}

/// A generic system used to update a specific [`TimeTracker`].
pub fn update_time_tracker<T>(
    time: StaticSystemParam<<T::Time as TimesWithContext>::AsSystemParam<'_>>,
    mut us: Query<&mut T>,
) where
    T: TimeTracker + Component,
{
    let inner = time.into_inner();
    for mut us in &mut us {
        us.update(&inner)
    }
}

/// A generic system that signals to [`TimeTracker`]s that they've entered the fixed update portion of the main loop.
///
/// Called in [`FixedPreUpdate`](crate::FixedPreUpdate) before [`update_time_tracker`].
/// Never called for `TimeTracker`s that don't depend on [`Time<Fixed>`](crate::Fixed).
pub fn enter_fixed_update<T>(mut us: Query<&mut T>)
where
    T: TimeTracker + Component,
{
    for mut us in &mut us {
        us.enter_fixed_update();
    }
}

/// A generic system that signals to [`TimeTracker`]s that they've exited the fixed update portion of the main loop.
///
/// Called in [`FixedPostUpdate`](crate::FixedPostUpdate).
/// Never called for `TimeTracker`s that don't depend on [`Time<Fixed>`](crate::Fixed).
pub fn exit_fixed_update<T>(mut us: Query<&mut T>)
where
    T: TimeTracker + Component,
{
    for mut us in &mut us {
        us.exit_fixed_update();
    }
}

/// A extension to the bevy_app to allow registering time trackers.
pub trait TimeTrackingAppExtension: sealed::Sealed {
    /// Registers a time tracker to be automatically updated.
    fn register_time_tracker<T>(&mut self) -> &mut Self
    where
        T: TimeTracker + Component;

    /// When defining a new [`Context`] for [`Time`](crate::Time) you must register Bevy's time tracking components
    /// to track that `Time<Context>`. This function does it for you.
    fn register_time_context<C>(&mut self) -> &mut Self
    where
        C: Context + Default + Send + Sync + 'static;
}

mod sealed {
    /// No you don't
    pub trait Sealed {}
    impl Sealed for bevy_app::App {}
}

impl TimeTrackingAppExtension for App {
    fn register_time_tracker<T: TimeTracker + Component>(&mut self) -> &mut Self {
        T::Time::register_after_time_updates::<T>(self);

        self
    }

    fn register_time_context<C>(&mut self) -> &mut Self
    where
        C: Context + Default + Send + Sync + 'static,
    {
        self.register_time_tracker::<UpdatingStopwatch<C>>()
            .register_time_tracker::<UpdatingTimer<C>>()
    }
}

impl<T: TimeTracker> TimeTracker for [T] {
    type Time = T::Time;

    fn update(
        &mut self,
        time: &<<Self::Time as TimesWithContext>::AsSystemParam<'_> as SystemParam>::Item<'_, '_>,
    ) {
        for tracker in self {
            tracker.update(time);
        }
    }

    fn enter_fixed_update(&mut self) {
        for tracker in self {
            tracker.enter_fixed_update();
        }
    }
}

impl<T: TimeTracker, const N: usize> TimeTracker for [T; N] {
    type Time = T::Time;

    fn update(
        &mut self,
        time: &<<Self::Time as TimesWithContext>::AsSystemParam<'_> as SystemParam>::Item<'_, '_>,
    ) {
        for tracker in self {
            tracker.update(time);
        }
    }

    fn enter_fixed_update(&mut self) {
        for tracker in self {
            tracker.enter_fixed_update();
        }
    }
}

macro_rules! impl_time_tracking {
    ($(($T:ident, $time:ident, $resource:ident)),*) => {
        impl<$($T: TimeTracker),*> TimeTracker for ($($T,)*) {
            #[allow(unused_parens)]
            type Time = ($($T::Time),*);

            #[allow(unused_parens)]
            fn update(
                &mut self,
                ($($resource),*): &<<Self::Time as TimesWithContext>::AsSystemParam<'_> as SystemParam>::Item<'_, '_>,
            ) {
                let ($($time,)*) = self;
                $($time.update($resource);)*
            }

            fn enter_fixed_update(&mut self) {
                let ($($time,)*) = self;
                $($time.enter_fixed_update();)*
            }

            fn exit_fixed_update(&mut self) {
                let ($($time,)*) = self;
                $($time.exit_fixed_update();)*
            }
        }
    };
}

all_tuples!(impl_time_tracking, 1, 15, T, t, r);
