use bevy_ecs::system::{ReadOnlySystemParam, SystemParam};
use bevy_utils::all_tuples;

/// Defines a component that can be registered to automatically track time.
///
/// Register this type into an [`App`] using [`register_time_tracker`](TimeTrackingAppExtension::register_time_tracker) to make it update automatically.
pub trait TimeTracker {
    /// If set to `true` this component will be registered to update after [`Time<Virtual>`](crate::virt::Virtual) and [`Time<Real>`](crate::real::Real) in the [First] schedule.
    const DOES_UPDATE: bool;
    /// If set to `true` this component will be registered to update after [`Time<Fixed>`](crate::fixed::Fixed) in the [`FixedPreUpdate`](crate::FixedPreUpdate) schedule.
    const DOES_FIXED_UPDATE: bool;

    /// The time source this time tracker tracker tracks in [`update`](TimeTracker::update).
    type UpdateSource<'w>: ReadOnlySystemParam;

    /// The time source this time tracker tracker tracks in [`fixed_update`](TimeTracker::fixed_update).
    type FixedUpdateSource<'w>: ReadOnlySystemParam;

    /// Updates this time tracker with the data from the [`UpdateSource`](TimeTracker::UpdateSource).
    fn update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::UpdateSource<'a> as SystemParam>::Item<'_, '_>,
    );

    /// Updates this time tracker with the data from the [`FixedUpdateSource`](TimeTracker::FixedUpdateSource).
    ///
    /// This function is intended for trackers which will be read in [`FixedUpdate`](bevy_app::FixedUpdate).
    fn fixed_update<'a: 'b, 'b>(
        &mut self,
        time: &'b <Self::FixedUpdateSource<'a> as SystemParam>::Item<'_, '_>,
    );
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
