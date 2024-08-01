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
//!
//! # This example
//!
//! In this example, we're demonstrating the APIs available by creating a simple counter
//! in four different ways:
//!
//! 1. Using a system with a `Changed` filter.
//! 2. Use the `Ref` query type and the `is_changed` method.
//! 3. Using a hook.
//! 4. Using an observer.
//!
//! The counter is incremented by pressing the corresponding button.
//! At this scale, we have neither performance nor complexity concerns:
//! see the discussion above for guidance on when to use each method.
//!
//! Hooks are not suitable for this application (as they represent intrinsic functionality for the type),
//! and cannot sensibly be added to the general-purpose [`Interaction`] component just to make these buttons work.
//! Instead, we demonstrate how to use them by adding a on-mutate hook to the [`CounterValue`] component which will
//! update the text of each button whenever the counter is incremented.

use bevy::prelude::*;
use on_mutate::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Example setup
        .add_systems(Startup, setup_ui)
        .add_systems(
            Update,
            (change_button_color_based_on_interaction, update_button_text)
                .in_set(ChangeDetectionSet),
        )
        // Change detection based methods
        .add_systems(
            Update,
            (
                update_counter_changed_filter.after(ChangeDetectionSet),
                update_counter_ref_query,
            ),
        )
        // Observer based methods
        // Checking for OnMutate events is rather expensive,
        // so unlike change detection, it is opt-in.
        .generate_on_mutate::<CounterValue>()
        .observe(update_counter_observer)
        .run();
}

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone)]
struct ChangeDetectionSet;

/// Tracks the value of the counter for each button.
#[derive(Component)]
struct CounterValue(u32);

/// A component that differentiates our buttons by their change-response strategy.
#[derive(Component, PartialEq)]
enum ChangeStrategy {
    ChangedFilter,
    RefQuery,
    Observer,
}

impl ChangeStrategy {
    fn color(&self) -> Srgba {
        use bevy::color::palettes::tailwind::*;

        match self {
            ChangeStrategy::ChangedFilter => RED_500,
            ChangeStrategy::RefQuery => ORANGE_500,
            ChangeStrategy::Observer => BLUE_500,
        }
    }

    fn button_string(&self) -> &'static str {
        match self {
            ChangeStrategy::ChangedFilter => "Changed Filter",
            ChangeStrategy::RefQuery => "Ref Query",
            ChangeStrategy::Observer => "Observer",
        }
    }
}

/// Generates an interactive button with a counter,
/// returning the entity ID of the button spawned.
fn spawn_button_with_counter(commands: &mut Commands, change_strategy: ChangeStrategy) -> Entity {
    commands
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(250.),
                    height: Val::Px(120.),
                    margin: UiRect::all(Val::Px(20.)),
                    ..default()
                },
                background_color: change_strategy.color().into(),
                border_radius: BorderRadius::all(Val::Px(20.)),
                ..default()
            },
            change_strategy,
            CounterValue(0),
        ))
        .with_children(|parent| {
            // We don't need to set the initial value of the Text component here,
            // as Changed filters are triggered whenever the value is mutated OR the component is added.
            parent.spawn(TextBundle {
                style: Style {
                    align_self: AlignSelf::Center,
                    width: Val::Percent(100.),
                    ..default()
                },
                ..default()
            });
        })
        .id()
}

// This system implicitly filters out any entities whose `Interaction` component hasn't changed.
fn update_counter_changed_filter(
    mut query: Query<(&Interaction, &ChangeStrategy, &mut CounterValue), Changed<Interaction>>,
) {
    for (interaction, change_strategy, mut counter) in query.iter_mut() {
        if change_strategy != &ChangeStrategy::ChangedFilter {
            continue;
        }

        if *interaction == Interaction::Pressed {
            counter.0 += 1;
        }
    }
}

// This system works just like the one above, except entries that are not changed will be included.
// We can check if the entity has changed by calling the `is_changed` method on the `Ref` type.
// The [`Mut`] and [`ChangeTrackers`] types also have these methods.
fn update_counter_ref_query(
    mut query: Query<(Ref<Interaction>, &ChangeStrategy, &mut CounterValue)>,
) {
    for (interaction, change_strategy, mut counter) in query.iter_mut() {
        if change_strategy != &ChangeStrategy::RefQuery {
            continue;
        }

        // Being able to check if the entity has changed inside of the system is
        // sometimes useful for more complex logic, but Changed filters are generally clearer.
        if interaction.is_changed() && *interaction == Interaction::Pressed {
            counter.0 += 1;
        }
    }
}

// This observer is added to the app using the `observe` method,
// and will run whenever the `Interaction` component is mutated.
// Like above, we're returning early if the button isn't the one we're interested in.
// FIXME: this isn't currently working
fn update_counter_observer(
    trigger: Trigger<OnMutate<Interaction>>,
    mut button_query: Query<(&mut CounterValue, &Interaction, &ChangeStrategy)>,
) {
    let Ok((mut counter, interaction, change_strategy)) = button_query.get_mut(trigger.entity())
    else {
        // Other entities may have the Interaction component, but we're only interested in these particular buttons.
        return;
    };

    if *change_strategy != ChangeStrategy::Observer {
        return;
    }

    // OnMutate events will be generated whenever *any* change occurs,
    // even if it's not to the value we're interested in.
    if *interaction == Interaction::Pressed {
        counter.0 += 1;
    }
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let root_node = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .id();

    let changed_filter_button =
        spawn_button_with_counter(&mut commands, ChangeStrategy::ChangedFilter);
    let ref_query_button = spawn_button_with_counter(&mut commands, ChangeStrategy::RefQuery);
    let observer_button = spawn_button_with_counter(&mut commands, ChangeStrategy::Observer);

    commands.entity(root_node).push_children(&[
        changed_filter_button,
        ref_query_button,
        observer_button,
    ]);
}

// This is another example of a change-detection based system,
// which only acts on buttons whose `Interaction` component has changed to save on work.
//
// Because the operation is idempotent (calling it multiple times has the same effect as calling it once),
// this is purely a performance optimization.
fn change_button_color_based_on_interaction(
    mut query: Query<(&mut BackgroundColor, &ChangeStrategy, &Interaction), Changed<Interaction>>,
) {
    for (mut background_color, change_strategy, interaction) in query.iter_mut() {
        let standard_color = change_strategy.color();

        *background_color = match interaction {
            Interaction::None => standard_color.into(),
            Interaction::Hovered => standard_color.darker(0.15).into(),
            Interaction::Pressed => standard_color.darker(0.3).into(),
        };
    }
}

// TODO: implement this using hooks
// Like other filters, `Changed` (and `Added`) filters can be composed via `Or` filters.
// The default behavior for both query data and filters is to use AND logic.
// In this case, a truly robust solution should update whenever the counter value
// or the children that point to the text entity change.
fn update_button_text(
    counter_query: Query<
        (&CounterValue, &ChangeStrategy, &Children),
        Or<(Changed<CounterValue>, Changed<Children>)>,
    >,
    mut text_query: Query<&mut Text>,
) {
    for (counter, change_strategy, children) in counter_query.iter() {
        for child in children.iter() {
            // By attempting to fetch the Text component on each child and continuing if it fails,
            // we can avoid panicking if non-text children are present.
            if let Ok(mut text) = text_query.get_mut(*child) {
                let string = format!("{}: {}", change_strategy.button_string(), counter.0);
                *text = Text {
                    sections: vec![TextSection {
                        value: string,
                        style: TextStyle::default(),
                    }],
                    justify: JustifyText::Center,
                    ..default()
                };
            }
        }
    }
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
