use crate::{
    enter_fixed_update, exit_fixed_update, update_time_tracker, Time, TimeTracker,
    UpdateTimeTrackers,
};
use bevy_app::{App, First, FixedPostUpdate, FixedPreUpdate};
use bevy_ecs::{
    component::Component,
    schedule::IntoSystemConfigs,
    system::{ReadOnlySystemParam, Res},
};
use bevy_utils::all_tuples;

/// A context for a [`Time`].
///
/// For time trackers depending on you context to work correctly. The time with your context
/// should update before [`UpdateTimeTrackers`] in the [`First`] schedule.
pub trait Context {
    #[doc(hidden)]
    // This must be set to true if `ScheduleLabelType` is [`FixedPreUpdate`].
    const IS_FIXED_UPDATE: bool = false;
}

pub trait TimesWithContext: sealed::Sealed {
    type AsSystemParam<'w>: ReadOnlySystemParam;

    #[doc(hidden)]
    const USES_FIXED_UPDATE: bool;

    #[doc(hidden)]
    // Ideally we would have a bound <T as TimeTracker>::Time: Self but we can't currently express that.
    fn register_after_time_updates<T: TimeTracker + Component>(app: &mut App);
}

impl<C> TimesWithContext for Time<C>
where
    C: Context + Send + Sync + Default + 'static,
{
    type AsSystemParam<'w> = Res<'w, Self>;
    const USES_FIXED_UPDATE: bool = C::IS_FIXED_UPDATE;

    fn register_after_time_updates<T: TimeTracker + Component>(app: &mut App) {
        if C::IS_FIXED_UPDATE {
            app.add_systems(
                FixedPreUpdate,
                enter_fixed_update::<T>.in_set(UpdateTimeTrackers),
            )
            .add_systems(
                FixedPostUpdate,
                exit_fixed_update::<T>.in_set(UpdateTimeTrackers),
            )
            .add_systems(
                FixedPreUpdate,
                update_time_tracker::<T>
                    .in_set(UpdateTimeTrackers)
                    .after(enter_fixed_update::<T>),
            );
        } else {
            app.add_systems(First, update_time_tracker::<T>.in_set(UpdateTimeTrackers));
        }
    }
}

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl<C: Default + Context> Sealed for Time<C> {}
}

macro_rules! impl_times_with_context {
    ($($T:ident),*) => {
        impl<$($T: TimesWithContext),*> TimesWithContext for ($($T,)*) {
            #[allow(unused_parens)]
            type AsSystemParam<'w> = (
                $(<$T as TimesWithContext>::AsSystemParam<'w>),*
            );

            // The trailing false is to deal with the impl for ()
            const USES_FIXED_UPDATE: bool = $(<$T as TimesWithContext>::USES_FIXED_UPDATE ||)* false;


            fn register_after_time_updates<R: TimeTracker + Component>(_app: &mut App) {
                $(
                    <$T as TimesWithContext>::register_after_time_updates::<R>(_app);
                )*
            }
        }

        impl<$($T: TimesWithContext),*> sealed::Sealed for ($($T,)*) {}
    };
}

all_tuples!(impl_times_with_context, 0, 15, C);
