//! Example `no_std` compatible Bevy library.

// The first step to a `no_std` library is to add this annotation:

#![no_std]

// This does 2 things to your crate:
//  1. It prevents automatically linking the `std` crate with yours.
//  2. It switches to `core::prelude` instead of `std::prelude` for what is implicitly
//     imported in all modules in your crate.

// It is common to want to use `std` when it's available, and fall-back to an alternative
// implementation which may make compromises for the sake of compatibility.
// To do this, you can conditionally re-include the standard library:

#[cfg(feature = "std")]
extern crate std;

// This still uses the `core` prelude, so items such as `std::println` aren't implicitly included
// in all your modules, but it does make them available to import.

// Because Bevy requires access to an allocator anyway, you are free to include `alloc` regardless
// of what features are enabled.
// This gives you access to `Vec`, `String`, `Box`, and many other allocation primitives.

extern crate alloc;

// Here's our first example of using something from `core` instead of `std`.
// Since `std` re-exports `core` items, they are the same type just with a different name.
// This means any 3rd party code written for `std::time::Duration` will work identically for
// `core::time::Duration`.

use core::time::Duration;

// With the above boilerplate out of the way, everything below should look very familiar to those
// who have worked with Bevy before.

use bevy::prelude::*;

// While this example doesn't need it, a lot of fundamental types which are exclusively in `std`
// have alternatives in `bevy::platform`.
// If you find yourself needing a `HashMap`, `RwLock`, or `Instant`, check there first!

#[expect(unused_imports, reason = "demonstrating some available items")]
use bevy::platform::{
    collections::{HashMap, HashSet},
    hash::DefaultHasher,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc, Barrier, LazyLock, Mutex, Once, OnceLock, RwLock, Weak,
    },
    time::Instant,
};

// Note that `bevy::platform::sync::Arc` exists, despite `alloc::sync::Arc` being available.
// The reason is not every platform has full support for atomic operations, so `Arc`, `AtomicBool`,
// etc. aren't always available.
// You can test for their inclusion with `#[cfg(target_has_atomic = "ptr")]` and other related flags.
// You can get a more cross-platform alternative from `portable-atomic`, but Bevy handles this for you!
// Simply use `bevy::platform::sync` instead of `core::sync` and `alloc::sync` when possible,
// and Bevy will handle selecting the fallback from `portable-atomic` when it is required.

/// Plugin for working with delayed components.
///
/// You can delay the insertion of a component by using [`insert_delayed`](EntityCommandsExt::insert_delayed).
pub struct DelayedComponentPlugin;

impl Plugin for DelayedComponentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, tick_timers);
    }
}

/// Extension trait providing [`insert_delayed`](EntityCommandsExt::insert_delayed).
pub trait EntityCommandsExt {
    /// Insert the provided [`Bundle`] `B` with a provided `delay`.
    fn insert_delayed<B: Bundle>(&mut self, bundle: B, delay: Duration) -> &mut Self;
}

impl EntityCommandsExt for EntityCommands<'_> {
    fn insert_delayed<B: Bundle>(&mut self, bundle: B, delay: Duration) -> &mut Self {
        self.insert((
            DelayedComponentTimer(Timer::new(delay, TimerMode::Once)),
            DelayedComponent(bundle),
        ))
        .observe(unwrap::<B>)
    }
}

impl EntityCommandsExt for EntityWorldMut<'_> {
    fn insert_delayed<B: Bundle>(&mut self, bundle: B, delay: Duration) -> &mut Self {
        self.insert((
            DelayedComponentTimer(Timer::new(delay, TimerMode::Once)),
            DelayedComponent(bundle),
        ))
        .observe(unwrap::<B>)
    }
}

#[derive(Component, Deref, DerefMut, Reflect, Debug)]
#[reflect(Component)]
struct DelayedComponentTimer(Timer);

#[derive(Component)]
#[component(immutable)]
struct DelayedComponent<B: Bundle>(B);

#[derive(EntityEvent)]
struct Unwrap;

fn tick_timers(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DelayedComponentTimer)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            commands
                .entity(entity)
                .remove::<DelayedComponentTimer>()
                .trigger(Unwrap);
        }
    }
}

fn unwrap<B: Bundle>(trigger: On<Unwrap>, world: &mut World) {
    if let Ok(mut target) = world.get_entity_mut(trigger.target())
        && let Some(DelayedComponent(bundle)) = target.take::<DelayedComponent<B>>()
    {
        target.insert(bundle);
    }

    world.despawn(trigger.observer());
}
