//! Reactivity is a technique that allows your UI to automatically update when the data that defines its state changes.
//!
//! This example demonstrates how to use reactivity in Bevy with observers.
//!
//! There are a few key benefits to using reactivity in your UI:
//!
//! - **Deduplication of Spawning and Updating Logic**: When you spawn an entity, you can declare what its value should be.
//! - **Automatic Updates**: When the data that defines your UI state changes, the UI will automatically update to reflect those changes.
//! - **Widget-bound Behavior**: By defining the behavior of a widget in the same place as its data, you can simply spawn the widget and let the spawned observers handle the rest.
//!
//! # Observers
//!
//! Observers are a way to listen for and respond to entity-targeted events.
//! In Bevy, they have several key properties:
//!
//! - You can access both the event and the entity that the event is targeted at.
//! - Observers can only be triggered via commands: any triggers will be deferred until the next synchronization point where exclusive world access is available.
//! - Observers occur immediately after the event is triggered.
//! - Observers can be used to trigger other events, creating a cascade of reactive updates.
//! - Observers can be set to watch for events targeting a specific entity, or for any event of that type.
//!
//! # Incrementalization
//!
//! In order to avoid recomputing the entire UI every frame, Bevy uses a technique called incrementalization.
//! This means that Bevy will only update the parts of the UI that have changed.
//!
//! The key techniques here are **change detection**, which is tracked and triggered by the `Mut` and `ResMut` smart pointers,
//! and **lifecycle hooks**, which are events emitted whenever components are added or removed (including when entities are spawned or despawned).
//!
//! This gives us a very powerful set of standardized events that we can listen for and respond to:
//!
//! - [`OnAdd`]: triggers when a matching component is added to an entity.
//! - [`OnInsert`]: triggers when a component is added to or overwritten on an entity.
//! - [`OnReplace`]: triggers when a component is removed from or overwritten on on an entity.
//! - [`OnRemove`]: triggers when a component is removed from an entity.
//!
//! Note that "overwritten" has a specific meaning here: these are only triggered if the components value is changed via a new insertion operation.
//! Ordinary mutations to the component's value will not trigger these events.
//!
//! However, we can opt into change-detection powered observers by calling `app.generate_on_mutate::<MyComponent>()`.
//! This will watch for changes to the component and trigger a [`OnMutate`] event targeting the entity whose component has changed.
//! It's important to note that mutations are observed whenever components are *added* to the entity as well,
//! ensuring that reactive behavior is triggered even when the widget is first spawned.
//!
//! In addition, arbitrary events can be defined and triggered, which is an excellent pattern for behavior that requires a more complex or specialized response.
//!
//! # This example
//!
//! To demonstrate these concepts, we're going to create a simple UI that displays a counter.
//! We'll then create a button that increments the counter when clicked.

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
