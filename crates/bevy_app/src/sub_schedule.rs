use std::fmt::{Debug, Display};

use bevy_ecs::{
    change_detection::Mut,
    schedule::{ScheduleLabel, ScheduleLabelId, Stage, StageLabel, StageLabelId},
    system::IntoExclusiveSystem,
    world::World,
};
use bevy_utils::HashMap;
use thiserror::Error;

use crate::App;

/// Methods for converting a schedule into a [sub-Schedule](SubSchedule) descriptor.
pub trait IntoSubSchedule: Sized {
    /// The wrapped schedule type.
    type Sched: Stage;
    /// The type that controls the behaviour of the exclusive system
    /// which runs [`SubSchedule`]s of this type.
    type Runner: FnMut(&mut Self::Sched, &mut World) + Send + Sync + 'static;

    /// Applies the specified label to the current schedule.
    /// This means it will be accessible in the [`SubSchedules`] resource after
    /// being added to the [`App`].
    fn label(self, label: impl ScheduleLabel) -> SubSchedule<Self::Sched, Self::Runner> {
        let mut sub = Self::into_sched(self);
        sub.label = Some(label.as_label());
        sub
    }
    /// Defines a function that runs the current schedule. It will be inserted into
    /// an exclusive system within the stage `stage`.
    ///
    /// Overwrites any previously set runner or stage.
    fn with_runner<F>(self, stage: impl StageLabel, f: F) -> SubSchedule<Self::Sched, F>
    where
        F: FnMut(&mut Self::Sched, &mut World) + Send + Sync + 'static,
    {
        let SubSchedule {
            schedule, label, ..
        } = Self::into_sched(self);
        SubSchedule {
            schedule,
            label,
            runner: Some((stage.as_label(), f)),
        }
    }

    /// Performs the conversion. You usually do not need to call this directly.
    fn into_sched(_: Self) -> SubSchedule<Self::Sched, Self::Runner>;
}

impl<S: Stage> IntoSubSchedule for S {
    type Sched = Self;
    type Runner = fn(&mut Self, &mut World);
    fn into_sched(schedule: Self) -> SubSchedule<Self, Self::Runner> {
        SubSchedule {
            schedule,
            label: None,
            runner: None,
        }
    }
}

impl<S: Stage, R> IntoSubSchedule for SubSchedule<S, R>
where
    R: FnMut(&mut S, &mut World) + Send + Sync + 'static,
{
    type Sched = S;
    type Runner = R;
    #[inline]
    fn into_sched(sched: Self) -> SubSchedule<S, R> {
        sched
    }
}

/// A schedule that may run independently of the main app schedule.
pub struct SubSchedule<S: Stage, F>
where
    F: FnMut(&mut S, &mut World) + Send + Sync + 'static,
{
    schedule: S,
    label: Option<ScheduleLabelId>,
    runner: Option<(StageLabelId, F)>,
}

/// A [resource](bevy_ecs::system::Res) that stores all labeled [`SubSchedule`]s.
#[derive(Default)]
pub struct SubSchedules {
    // INVARIANT: A `SubSlot` cannot be removed once added, and is associated with
    // a single schedule. Even if a slot gets temporarily emptied, it is guaranteed
    // that the slot will always get refilled by the same exact schedule.
    map: HashMap<ScheduleLabelId, SubSlot>,
}

struct SubSlot(Option<Box<dyn Stage>>);

/// Error type returned by [`SubSchedules::extract_from`](SubSchedules#method.extract_from).
#[derive(Error)]
#[non_exhaustive]
pub enum ExtractError {
    /// Schedule could not be found.
    #[error("there is no sub-schedule with label '{0:?}'")]
    NotFound(ScheduleLabelId),
    /// Schedule is being extracted by someone else right now.
    #[error("cannot extract sub-schedule '{0:?}', as it is currently extracted already")]
    AlreadyExtracted(ScheduleLabelId),
    /// The [`SubSchedules`] resource got removed during the scope.
    #[error("the `SubSchedules` resource got removed during the scope for schedule '{0:?}'")]
    ResourceLost(ScheduleLabelId),
}
impl Debug for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Error)]
/// Error type returned by [`SubSchedules::insert`](SubSchedules#method.insert).
#[non_exhaustive]
pub enum InsertError {
    /// A schedule with this label already exists.
    #[error("there is no sub-schedule with label '{0:?}'")]
    Duplicate(ScheduleLabelId),
}
impl Debug for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl SubSchedules {
    /// Inserts a new sub-schedule.
    ///
    /// # Errors
    /// If there is already a sub-schedule labeled `label`.
    pub fn insert(
        &mut self,
        label: impl ScheduleLabel,
        sched: Box<dyn Stage>,
    ) -> Result<(), InsertError> {
        let label = label.as_label();
        if self.map.contains_key(&label) {
            return Err(InsertError::Duplicate(label));
        }
        self.map.insert(label, SubSlot(Some(sched)));
        Ok(())
    }

    /// Temporarily extracts a [`SubSchedule`] from the world, and provides a scope
    /// that has mutable access to both the schedule and the [`World`].
    /// At the end of this scope, the sub-schedule is automatically reinserted.
    ///
    /// # Errors
    /// If there is no schedule associated with `label`, or if that schedule
    /// is currently already extracted.
    pub fn extract_scope<F, T>(
        world: &mut World,
        label: impl ScheduleLabel,
        f: F,
    ) -> Result<T, ExtractError>
    where
        F: FnOnce(&mut World, &mut dyn Stage) -> T,
    {
        let label = label.as_label();

        // Extract.
        let mut schedules = world.resource_mut::<Self>();
        let mut sched = schedules
            .map
            .get_mut(&label)
            .ok_or(ExtractError::NotFound(label))?
            .0
            .take()
            .ok_or(ExtractError::AlreadyExtracted(label))?;

        // Execute.
        let val = f(world, sched.as_mut());

        // Re-insert.
        world
            .get_resource_mut::<Self>()
            .ok_or(ExtractError::ResourceLost(label))?
            .map
            .get_mut(&label)
            .expect("schedule must exist in the map since we found it previously")
            .0 = Some(sched);

        Ok(val)
    }

    /// Gets a mutable reference to the sub-schedule identified by `label`.
    ///
    /// # Panics
    /// If the schedule is currently [extracted](#method.extract_scope).
    pub fn get_mut<S: Stage>(&mut self, label: impl ScheduleLabel) -> Option<&mut S> {
        #[cold]
        fn panic(label: impl Debug) -> ! {
            panic!("cannot get sub-schedule '{label:?}', as it is currently extracted")
        }

        let label = label.as_label();
        let sched = match self.map.get_mut(&label)?.0.as_deref_mut() {
            Some(x) => x,
            None => panic(label),
        };
        sched.downcast_mut()
    }
}

#[track_caller]
pub(crate) fn add_to_app<S: Stage>(app: &mut App, schedule: impl IntoSubSchedule<Sched = S>) {
    let SubSchedule {
        mut schedule,
        label,
        runner,
    } = IntoSubSchedule::into_sched(schedule);

    // If it has a label, insert it to the public resource.
    if let Some(label) = label {
        let mut res: Mut<SubSchedules> = app.world.get_resource_or_insert_with(Default::default);
        res.insert(label, Box::new(schedule)).unwrap();

        if let Some((stage, mut runner)) = runner {
            // Driver which extracts the schedule from the world and runs it.
            let driver = move |w: &mut World| {
                SubSchedules::extract_scope(w, label, |w, sched| {
                    let sched = if let Some(s) = sched.downcast_mut::<S>() {
                        s
                    } else {
                        #[cfg(debug_assertions)]
                        unreachable!(
                            r#"
                            The sub-schedule '{label:?}' changed types after being inserted:
                            it should be of type `{expect}`, but it's really of type `{actual}`.
                            This should be impossible.
                            If run into this error, please file an issue at https://github.com/bevyengine/bevy/issues/new/choose"#,
                            expect = std::any::type_name::<S>(),
                            actual = AnyTypeName::name(sched),
                        );
                        // SAFETY: Due to the invariant on `SubSchedules`, we can be sure that
                        // `sched` is the same instance that we inserted.
                        // Thus, we can rely on its type matching `S`.
                        #[cfg(not(debug_assertions))]
                        unsafe {
                            std::hint::unreachable_unchecked()
                        }
                    };
                    runner(sched, w);
                })
                .unwrap();
            };
            app.add_system_to_stage(stage, driver.exclusive_system());
        }
    } else if let Some((stage, mut runner)) = runner {
        // If there's no label, then the schedule isn't visible publicly.
        // We can just store it locally
        let driver = move |w: &mut World| {
            runner(&mut schedule, w);
        };
        app.add_system_to_stage(stage, driver.exclusive_system());
    } else {
        panic!("inserted sub-schedule can never be accessed, as it has neither a label nor a runner function")
    }
}

/// Trait that lets us get the type name of a trait object for debugging purposes.
trait AnyTypeName: 'static {
    fn name(&self) -> &'static str;
}
impl<T: ?Sized + 'static> AnyTypeName for T {
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
