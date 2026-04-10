//! This is about hooks for the [`Schedule`](super::Schedule) execution phases,
//! aiming to handle instructions triggered either before entering the `Schedule` or after exiting it.

use crate::{intern::Interned, schedule::ScheduleLabel};
use bevy_ecs_macros::Event;

/// Trigger schedule hook related to entering a specific [`Schedule`](super::Schedule).
///
/// # Examples
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::schedule::{ScheduleEnter, ScheduleLabel};
///
/// #[derive(Debug, ScheduleLabel, Hash, Clone, PartialEq, Eq)]
/// pub struct HookLabel;
///
/// #[derive(Debug, Resource, Default)]
/// struct Count(i32);
///
/// let mut world = World::new();
///
/// world.init_resource::<Count>();
///
/// let enter_hook = world
///     .add_observer(|trigger: On<ScheduleEnter>, mut res: ResMut<Count>| {
///         res.0 += 1;
///     })
///     .id();
///
/// world.add_schedule(Schedule::new(HookLabel));
///
/// world.run_schedule(HookLabel);
/// let count = world.resource::<Count>();
/// assert_eq!(1, count.0);
///
/// world.run_schedule(HookLabel);
/// let count = world.resource::<Count>();
/// assert_eq!(2, count.0);
///
/// world.despawn(enter_hook);
///
/// world.run_schedule(HookLabel);
/// let count = world.resource::<Count>();
/// assert_eq!(2, count.0);
///
/// ```
#[derive(Debug, Event, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleEnter(pub Interned<dyn ScheduleLabel>);

/// Trigger schedule hook related to exiting a specific [`Schedule`](super::Schedule).
///
/// # Examples
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::schedule::{ScheduleExit, ScheduleLabel};
///
/// #[derive(Debug, ScheduleLabel, Hash, Clone, PartialEq, Eq)]
/// pub struct HookLabel;
///
/// #[derive(Debug, Resource, Default)]
/// struct Count(i32);
///
/// let mut world = World::new();
///
/// world.init_resource::<Count>();
///
/// let enter_hook = world
///     .add_observer(|trigger: On<ScheduleExit>, mut res: ResMut<Count>| {
///         res.0 += 1;
///     })
///     .id();
///
/// world.add_schedule(Schedule::new(HookLabel));
///
/// world.run_schedule(HookLabel);
/// let count = world.resource::<Count>();
/// assert_eq!(1, count.0);
///
/// world.run_schedule(HookLabel);
/// let count = world.resource::<Count>();
/// assert_eq!(2, count.0);
///
/// world.despawn(enter_hook);
///
/// world.run_schedule(HookLabel);
/// let count = world.resource::<Count>();
/// assert_eq!(2, count.0);
///
/// ```
#[derive(Debug, Event, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleExit(pub Interned<dyn ScheduleLabel>);
