use crate::schedule_v3::*;
use crate::world::World;

/// Temporary "stageless" `App` methods.
pub trait AppExt {
    /// Sets the [`Schedule`] that will be modified by default when you call `App::add_system`
    /// and similar methods.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    fn set_default_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self;
    /// Applies the function to the [`Schedule`] associated with `label`.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnMut(&mut Schedule),
    ) -> &mut Self;
    /// Adds [`State<S>`] and [`NextState<S>`] resources, [`OnEnter`] and [`OnExit`] schedules
    /// for each state variant, and an instance of [`apply_state_transition::<S>`] in
    /// \<insert-`bevy_core`-set-name\> so that transitions happen before `Update`.
    fn add_state<S: States>(&mut self) -> &mut Self;
}

/// Temporary "stageless" [`World`] methods.
pub trait WorldExt {
    /// Runs the [`Schedule`] associated with `label`.
    fn run_schedule(&mut self, label: impl ScheduleLabel);
}

impl WorldExt for World {
    fn run_schedule(&mut self, label: impl ScheduleLabel) {
        if let Some(mut schedule) = self.resource_mut::<Schedules>().remove(&label) {
            schedule.run(self);
            self.resource_mut::<Schedules>().insert(label, schedule);
        }
    }
}
