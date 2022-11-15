use crate::schedule_v3::*;
use crate::world::World;

/// New "stageless" [`App`](bevy_app::App) methods.
pub trait AppExt {
    fn set_default_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self;
    fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnMut(&mut Schedule),
    ) -> &mut Self;
}

/// New "stageless" [`World`] methods.
pub trait WorldExt {
    /// Runs the [`Schedule`] associated with the provided [`ScheduleLabel`].
    fn run_schedule(&mut self, label: impl ScheduleLabel);
}

impl WorldExt for World {
    fn run_schedule(&mut self, label: impl ScheduleLabel) {
        let mut schedule = self.resource_mut::<Schedules>().check_out(&label).unwrap();
        schedule.run(self);
        self.resource_mut::<Schedules>()
            .check_in(&label, schedule)
            .unwrap();
    }
}
