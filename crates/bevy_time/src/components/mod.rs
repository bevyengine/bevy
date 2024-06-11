mod mixed;
mod updating_stopwatch;
mod updating_timer;

pub use mixed::*;
pub use updating_stopwatch::*;
pub use updating_timer::*;

use crate::TimeTracker;
use crate::{Fixed, Real, Time, Virtual};

use bevy_ecs::change_detection::Res;

macro_rules! impl_time_tracking {
    ($ty:ident<Fixed>, $field:ident) => {
        impl TimeTracker for $ty<Fixed> {
            const DOES_UPDATE: bool = false;

            const DOES_FIXED_UPDATE: bool = true;

            type UpdateSource<'a> = ();

            type FixedUpdateSource<'a> = Res<'a, Time<Fixed>>;

            fn update<'a: 'b, 'b>(
                &mut self,
                _time: &'b <Self::UpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
            ) {
            }

            fn fixed_update<'a: 'b, 'b>(
                &mut self,
                time: &'b <Self::FixedUpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<
                    '_,
                    '_,
                >,
            ) {
                self.$field.tick(time.delta());
            }
        }
    };
    ($ty:ident<$time:ident>, $field:ident) => {
        impl TimeTracker for $ty<$time> {
            const DOES_UPDATE: bool = true;

            const DOES_FIXED_UPDATE: bool = false;

            type UpdateSource<'a> = Res<'a, Time<$time>>;

            type FixedUpdateSource<'a> = ();

            fn update<'a: 'b, 'b>(
                &mut self,
                time: &'b <Self::UpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<'_, '_>,
            ) {
                self.$field.tick(time.delta());
            }

            fn fixed_update<'a: 'b, 'b>(
                &mut self,
                _time: &'b <Self::FixedUpdateSource<'a> as bevy_ecs::system::SystemParam>::Item<
                    '_,
                    '_,
                >,
            ) {
            }
        }
    };
}

impl_time_tracking!(UpdatingTimer<Virtual>, timer);
impl_time_tracking!(UpdatingTimer<Real>, timer);
impl_time_tracking!(UpdatingTimer<Fixed>, timer);
impl_time_tracking!(UpdatingStopwatch<Virtual>, watch);
impl_time_tracking!(UpdatingStopwatch<Real>, watch);
impl_time_tracking!(UpdatingStopwatch<Fixed>, watch);
