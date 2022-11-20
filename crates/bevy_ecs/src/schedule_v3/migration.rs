use crate::schedule_v3::*;
use crate::world::World;

/// New "stageless" [`App`](bevy_app::App) methods.
pub trait AppExt {
    /// Sets the [`Schedule`] that will be modified by default when you call `App::add_system`
    /// and similar methods.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    fn set_default_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self;
    /// Sets the [`Schedule`] that will be modified by default within the scope of `f` and calls it.
    /// Afterwards, restores the default to its previous value.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    fn edit_schedule(&mut self, label: impl ScheduleLabel, f: impl FnMut(&mut Self)) -> &mut Self;
}

/// New "stageless" [`World`] methods.
pub trait WorldExt {
    /// Runs the [`Schedule`] associated with `label`.
    fn run_schedule(&mut self, label: impl ScheduleLabel);
}

impl WorldExt for World {
    fn run_schedule(&mut self, label: impl ScheduleLabel) {
        let mut schedule = self.resource_mut::<Schedules>().remove(&label).unwrap();
        schedule.run(self);
        self.resource_mut::<Schedules>()
            .insert(label, schedule)
            .unwrap();
    }
}
