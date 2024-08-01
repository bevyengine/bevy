//! Bevy has two primary ways to respond to changes in your ECS data:
//!
//! 1. **Change detection:** whenever a component or resource is mutated, it will be flagged as changed.
//! 2. **Hooks and observers:** whenever changes or lifecycle events occur, functions will be called to respond to them.
//!
//! While similar, these two methods have different use cases and performance characteristics.
//! Change detection is fundamentally a polling-based mechanism: changes need to be looked for proactively,
//! and so the cost of change detection is paid every time the system runs (generally every frame),
//! regardless of whether or not any changes have occurred.
//!
//! By contrast, hooks and observers are event-driven: they only run when the event they're watching for occurs.
//! However, each event is processed individually, increasing the overhead when many changes occur.
//!
//! As a result, change detection is better suited to use cases where large volumes of data are being processed,
//! while hooks and observers are better suited to use cases where the data is relatively stable and changes are infrequent.
//!
//! There are two more important differences. Firstly, change detection is triggered immediately when the change occurs,
//! while hooks and observers are deferred until the next synchronization point where exclusive world access is available.
//! In Bevy, systems are run in parallel by default, so synchronizing forces the scheduler to wait until
//! all systems have finished running before proceeding and prevent systems before and after the sync point from running concurrently.
//!
//! Second, while change detection systems only run periodically,
//! hooks and observers are checked after every mutation to the world during sync points.
//!
//! Taken together, this means that change detection is good for periodic updates (but it's harder to avoid invalid state),
//! while hooks and observers are good for immediate updates and can chain into other hooks/observers indefinitely,
//! creating a cascade of reactions (but they need to wait until a sync point).
//!
//! You might use change detection for:
//!
//! - physics simulation
//! - AI action planning
//! - performance optimization in existing systems
//!
//! You might use hooks and observers for:
//!
//! - complex logic in turn-based games
//! - responding to user inputs
//! - adding behavior to UI elements
//! - upholding critical invariants, like hierarchical relationships (hooks are better suited than observers for this)

use bevy::prelude::*;
use on_mutate::{GenOnMutate, OnMutate};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .generate_on_mutate::<CounterValue>()
        .generate_on_mutate::<Interaction>()
        .add_systems(Startup, setup_ui)
        .run();
}

#[derive(Component)]
struct CounterValue(u32);

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Counter
    let counter_entity = commands
        .spawn(TextBundle { ..default() })
        .insert(CounterValue(0))
        .observe(
            |trigger: Trigger<OnMutate<CounterValue>>,
             mut query: Query<(&CounterValue, &mut Text)>| {
                let (counter_value, mut text) = query.get_mut(trigger.entity()).unwrap();
                *text = Text::from_section(counter_value.0.to_string(), TextStyle::default());
            },
        )
        .id();

    // Button
    commands
        .spawn(ButtonBundle {
            style: Style {
                width: Val::Px(100.),
                height: Val::Px(100.),
                justify_self: JustifySelf::End,
                ..default()
            },
            background_color: Color::WHITE.into(),
            ..default()
        })
        .observe(
            move |trigger: Trigger<OnMutate<Interaction>>,
                  interaction_query: Query<&Interaction>,
                  mut counter_query: Query<&mut CounterValue>| {
                let interaction = interaction_query.get(trigger.entity()).unwrap();
                if matches!(interaction, Interaction::Pressed) {
                    // We can move this value into the closure that we define,
                    // allowing us to create custom behavior for the button.
                    let mut counter = counter_query.get_mut(counter_entity).unwrap();
                    counter.0 += 1;
                }
            },
        );
}

/// This temporary module prototypes a user-space implementation of the [`OnMutate`] event.
///
/// This comes with two key caveats:
///
/// 1. Rather than being continually generated on every change between observers,
/// the list of [`OnMutate`] events is generated once at the start of the frame.
/// This restricts our ability to react indefinitely within a single frame, but is a good starting point.
/// 2. [`OnMutate`] will not have a generic parameter: instead, that will be handled via the second [`Trigger`] generic
/// and a static component ID, like the rest of the lifecycle events. This is just cosmetic.
///
/// To make this pattern hold up in practice, we likely need:
///
/// 0. Deep integration for the [`OnMutate`] event, so we can check for it in the same way as the other lifecycle events.
/// 1. Resource equivalents to all of the lifecycle hooks.
/// 2. Asset equivalents to all of the lifecycle hooks.
/// 3. Asset change detection.
///
/// As follow-up, we definitely want:
///
/// 1. Archetype-level change tracking.
/// 2. A way to automatically detect whether or not change detection triggers are needed.
/// 3. Better tools to gracefully exit observers when standard operations fail.
/// 4. Relations to make defining entity-links more robust and simpler.
/// 5. Nicer picking events to avoid having to use the naive OnMutate<Interaction> pattern.
///
/// We might also want:
///
/// 1. Syntax sugar to fetch matching components from the triggered entity in observers
mod on_mutate {
    use super::*;
    use std::marker::PhantomData;

    /// A trigger emitted when a component is mutated on an entity.
    ///
    /// This must be explicitly generated using [`GenOnMutate::generate_on_mutate`].
    #[derive(Event, Debug, Clone, Copy)]
    pub struct OnMutate<C: Component>(PhantomData<C>);

    impl<C: Component> Default for OnMutate<C> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }

    /// A temporary extension trait used to prototype this functionality.
    pub trait GenOnMutate {
        fn generate_on_mutate<C: Component>(&mut self) -> &mut Self;
    }

    impl GenOnMutate for App {
        fn generate_on_mutate<C: Component>(&mut self) -> &mut Self {
            self.add_systems(First, watch_for_mutations::<C>);

            self
        }
    }

    fn watch_for_mutations<C: Component>(mut commands: Commands, query: Query<Entity, Changed<C>>) {
        // Note that this is a linear time check, even when no mutations have occurred.
        // To accelerate this properly, we need to implement archetype-level change tracking.
        commands.trigger_targets(OnMutate::<C>::default(), query.iter().collect::<Vec<_>>());
    }
}
