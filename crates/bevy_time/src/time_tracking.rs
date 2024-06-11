use bevy_app::{App, First, FixedPreUpdate};
use bevy_ecs::{
    component::Component,
    schedule::IntoSystemConfigs,
    system::{Query, ReadOnlySystemParam, StaticSystemParam, SystemParam},
};
use bevy_utils::all_tuples;

use crate::UpdateTimeTrackers;

/// Defines a component that can be registered to automatically track time.
///
/// Register this type into an [`App`] using [`register_time_tracker`](TimeTrackingAppExtension::register_time_tracker) to make it update automatically.
pub trait TimeTracker {
    /// If set to `true` this component will be registered to update after [`Time<Virtual>`](crate::virt::Virtual) and [`Time<Real>`](crate::real::Real) in the [`First`] schedule.
    const DOES_UPDATE: bool;
    /// If set to `true` this component will be registered to update after [`Time<Fixed>`](crate::fixed::Fixed) in the [`FixedPreUpdate`] schedule.
    const DOES_FIXED_UPDATE: bool;

    /// The time source this time tracker tracker tracks in [`update`](TimeTracker::update).
    type UpdateSource<'w>: ReadOnlySystemParam;

    /// The time source this time tracker tracker tracks in [`fixed_update`](TimeTracker::fixed_update).
    type FixedUpdateSource<'w>: ReadOnlySystemParam;

    /// Updates this time tracker with the data from the [`UpdateSource`](TimeTracker::UpdateSource).
    ///
    /// If this time tracker was registered with [register_time_tracker](TimeTrackingAppExtension::register_time_tracker) this will be called in the [`First`] schedule.
    fn update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::UpdateSource<'a> as SystemParam>::Item<'_, '_>,
    );

    /// Updates this time tracker with the data from the [`FixedUpdateSource`](TimeTracker::FixedUpdateSource).
    ///
    /// This function is intended for trackers which will be read in [`FixedUpdate`](bevy_app::FixedUpdate).
    /// /// If this time tracker was registered with [register_time_tracker](TimeTrackingAppExtension::register_time_tracker) this will be called in the [`FixedPreUpdate`] schedule.
    fn fixed_update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::FixedUpdateSource<'a> as SystemParam>::Item<'_, '_>,
    );
}

/// A extension to the bevy_app to allow registering time trackers.
pub trait TimeTrackingAppExtension: sealed::Sealed {
    /// Registers a time tracker to be automatically updated
    fn register_time_tracker<T>(&mut self)
    where
        T: TimeTracker + Component;
}

mod sealed {
    /// No you don't
    pub trait Sealed {}
    impl Sealed for bevy_app::App {}
}

impl TimeTrackingAppExtension for App {
    fn register_time_tracker<T: TimeTracker + Component>(&mut self) {
        bevy_ecs::system::assert_is_system(update_time_tracker::<T>);
        if T::DOES_UPDATE {
            self.add_systems(First, update_time_tracker::<T>.in_set(UpdateTimeTrackers));
        }

        if T::DOES_FIXED_UPDATE {
            self.add_systems(
                FixedPreUpdate,
                update_time_tracker_fixed_update::<T>.in_set(UpdateTimeTrackers),
            );
        }
    }
}

/// A generic system used to update a specific [`TimeTracker`].
pub fn update_time_tracker<'a, T>(
    time: StaticSystemParam<T::UpdateSource<'a>>,
    mut us: Query<&mut T>,
) where
    T: TimeTracker + Component,
    T::UpdateSource<'a>: ReadOnlySystemParam,
{
    let inner = time.into_inner();
    for mut us in &mut us {
        us.update(&inner)
    }
}

/// A generic system used to update a specific [`TimeTracker`] in [`FixedUpdate`](bevy_app::FixedUpdate).
fn update_time_tracker_fixed_update<'a, T>(
    time: StaticSystemParam<T::FixedUpdateSource<'a>>,
    mut us: Query<&mut T>,
) where
    T: TimeTracker + Component,
{
    let inner = time.into_inner();
    for mut us in &mut us {
        us.fixed_update(&inner)
    }
}

impl<T: TimeTracker> TimeTracker for [T] {
    const DOES_UPDATE: bool = T::DOES_UPDATE;

    const DOES_FIXED_UPDATE: bool = T::DOES_FIXED_UPDATE;

    type UpdateSource<'w> = T::UpdateSource<'w>;

    type FixedUpdateSource<'w> = T::UpdateSource<'w>;

    fn update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::UpdateSource<'a> as SystemParam>::Item<'_, '_>,
    ) {
        for tracker in self {
            tracker.update(time);
        }
    }

    fn fixed_update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::FixedUpdateSource<'a> as SystemParam>::Item<'_, '_>,
    ) {
        for tracker in self {
            tracker.update(time);
        }
    }
}

impl<T: TimeTracker, const N: usize> TimeTracker for [T; N] {
    const DOES_UPDATE: bool = T::DOES_UPDATE;

    const DOES_FIXED_UPDATE: bool = T::DOES_FIXED_UPDATE;

    type UpdateSource<'w> = T::UpdateSource<'w>;

    type FixedUpdateSource<'w> = T::UpdateSource<'w>;

    fn update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::UpdateSource<'a> as SystemParam>::Item<'_, '_>,
    ) {
        for tracker in self {
            tracker.update(time);
        }
    }

    fn fixed_update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::FixedUpdateSource<'a> as SystemParam>::Item<'_, '_>,
    ) {
        for tracker in self {
            tracker.update(time);
        }
    }
}

macro_rules! impl_time_tracking {
    ($(($T:ident, $time:ident, $resource:ident)),*) => {
        impl<$($T: TimeTracker),*> TimeTracker for ($($T,)*) {
            const DOES_UPDATE: bool = $(<$T as TimeTracker>::DOES_UPDATE)||*;
            const DOES_FIXED_UPDATE: bool = $(<$T as TimeTracker>::DOES_FIXED_UPDATE)||*;

            #[allow(unused_parens)]
            type UpdateSource<'w> = ($(<$T as TimeTracker>::UpdateSource<'w>),*);

            #[allow(unused_parens)]
            type FixedUpdateSource<'w> = ($(<$T as TimeTracker>::FixedUpdateSource<'w>),*);

            #[allow(unused_parens)]
            fn update<'a: 'b, 'b>(
                &mut self,
                ($($resource),*): &'b <Self::UpdateSource<'a> as SystemParam>::Item<'_, '_>,
            ) {
                let ($($time,)*) = self;
                $($time.update($resource));*;
            }

            #[allow(unused_parens)]
            fn fixed_update<'a: 'b, 'b>(
                &mut self,
                ($($resource),*): &'b <Self::FixedUpdateSource<'a> as SystemParam>::Item<'_, '_>,
            ) {
                let ($($time,)*) = self;
                $($time.fixed_update($resource));*;
            }
        }
    };
}

all_tuples!(impl_time_tracking, 1, 15, T, t, r);
