//! This is about hooks for the [`Schedule`](super::Schedule) execution phases,
//! aiming to handle instructions triggered either before entering the `Schedule` or after exiting it.

use crate::world::World;
use crate::{intern::Interned, prelude::Resource, schedule::ScheduleLabel, system::SystemId};
use bevy_platform::collections::HashMap;
use bevy_platform::prelude::vec::Vec;
use log::error;

/// Used to control whether to retain or remove the [`ScheduleHook`] after it is triggered.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub enum ScheduleHookPlan {
    /// Remove after executing the [`ScheduleHook`]
    Clear,
    /// Keep after executing the [`ScheduleHook`]
    Keep,
}

/// Every valid [`Schedule`](super::Schedule) hook is a system that returns a [`ScheduleHookPlan`].
pub type ScheduleHook = SystemId<(), ScheduleHookPlan>;

/// The hub for managing [`ScheduleHook`], used to control when hooks are triggered.
/// 
/// When the world runs this, it first briefly removes itself from the World before executing the hooks.
/// Therefore, adding or removing hooks during hook execution will not take effect in the current phase.
/// 
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::schedule::{ScheduleHookPlan, ScheduleLabel};
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
/// let enter_hook = world.register_system(|mut res: ResMut<Count>| {
///     res.0 += 1;
///     ScheduleHookPlan::Clear
/// });
///
/// world.add_schedule(Schedule::new(HookLabel));
///
/// world.schedule_hooks().add_enter_hook(HookLabel, enter_hook);
///
/// world.run_schedule(HookLabel);
///
/// let count = world.resource::<Count>();
/// assert_eq!(1, count.0);
/// ```
#[derive(Debug, Resource, Default, Clone)]
pub struct ScheduleHooks {
    pub(crate) enter: HashMap<Interned<dyn ScheduleLabel>, Vec<ScheduleHook>>,
    pub(crate) exit: HashMap<Interned<dyn ScheduleLabel>, Vec<ScheduleHook>>,
}

impl ScheduleHooks {
    /// Used to merge the current [`ScheduleHooks`] with another.
    pub fn reinsert(&mut self, other: Self) -> &mut Self {
        self.enter.extend(other.enter);
        self.exit.extend(other.exit);
        self
    }

    /// Add a [`ScheduleHook`] to a [`ScheduleLabel`] that triggers before entering the [`Schedule`](super::Schedule).
    pub fn add_enter_hook(&mut self, label: impl ScheduleLabel, hook: ScheduleHook) -> &mut Self {
        self.enter
            .entry(label.intern())
            .and_modify(|hooks| {
                hooks.push(hook);
            })
            .or_insert(Vec::from([hook]));
        self
    }

    /// Add a [`ScheduleHook`] to a [`ScheduleLabel`] that triggers after exiting the [`Schedule`](super::Schedule).
    pub fn add_exit_hook(&mut self, label: impl ScheduleLabel, hook: ScheduleHook) -> &mut Self {
        self.exit
            .entry(label.intern())
            .and_modify(|hooks| {
                hooks.push(hook);
            })
            .or_insert(Vec::from([hook]));
        self
    }

    /// Execute the [`ScheduleHook`] that runs before a [`ScheduleLabel`].
    pub fn run_enter(&mut self, world: &mut World, label: impl ScheduleLabel) {
        if let Some(hooks) = self.enter.get_mut(&label.intern()) {
            hooks.retain(|hook| {
                world
                    .run_system(*hook)
                    .unwrap_or_else(|err| {
                        error!(
                            "a enter schedule hook fail,schedule label: {:?}, system id:{:?},error context:{:?}",
                            label, hook, err
                        );
                        ScheduleHookPlan::Clear
                    })
                    .eq(&ScheduleHookPlan::Keep)
            });
        }
    }

    /// Execute the [`ScheduleHook`] that runs after a [`ScheduleLabel`].
    pub fn run_exit(&mut self, world: &mut World, label: impl ScheduleLabel) {
        if let Some(hooks) = self.exit.get_mut(&label.intern()) {
            hooks.retain(|hook| {
                world
                    .run_system(*hook)
                    .unwrap_or_else(|err| {
                        error!(
                            "a exit schedule hook fail,schedule label: {:?}, system id:{:?},error context:{:?}",
                            label, hook, err
                        );
                        ScheduleHookPlan::Clear
                    })
                    .eq(&ScheduleHookPlan::Keep)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::Component,
        system::{Commands, Local},
    };

    use super::*;

    #[derive(Debug, ScheduleLabel, Hash, Clone, PartialEq, Eq)]
    pub struct HookLabel;

    #[derive(Debug, Component, PartialEq, Eq)]
    pub struct TestComponent;

    pub const SPAWN_COUNT: usize = 4;

    #[test]
    fn hook_success_run() {
        let mut world = World::new();

        let system = world.register_system(|mut commands: Commands, mut count: Local<usize>| {
            commands.spawn(TestComponent);
            if *count < SPAWN_COUNT {
                *count += 1;
                ScheduleHookPlan::Keep
            } else {
                ScheduleHookPlan::Clear
            }
        });

        let mut hooks = ScheduleHooks::default();

        hooks.add_enter_hook(HookLabel, system);

        for _ in 0..SPAWN_COUNT {
            hooks.run_enter(&mut world, HookLabel);
        }

        let mut query = world.query::<&TestComponent>();

        let iter = query.iter(&world);

        assert_eq!(SPAWN_COUNT, iter.count());

        hooks.run_enter(&mut world, HookLabel);

        assert!(hooks
            .enter
            .get(&HookLabel.intern())
            .is_some_and(|hooks| hooks.is_empty()));
    }

    #[test]
    fn hook_fail_run() {
        let mut world = World::new();

        let system = world.register_system(|mut commands: Commands| {
            commands.spawn(TestComponent);
            ScheduleHookPlan::Clear
        });

        let mut hooks = ScheduleHooks::default();

        hooks.add_enter_hook(HookLabel, system);

        assert_eq!(
            Some(1),
            hooks
                .enter
                .get(&HookLabel.intern())
                .map(|hooks| hooks.len())
        );

        world.despawn(system.entity);

        hooks.run_enter(&mut world, HookLabel);

        assert_eq!(
            Some(0),
            hooks
                .enter
                .get(&HookLabel.intern())
                .map(|hooks| hooks.len())
        );
    }
}
