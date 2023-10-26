//! This example illustrates how to use the `apply_deferred` system
//! to flush commands added by systems that have already run,
//! but have not had their buffers applied yet.
//!
//! This is useful when you don't want to wait until the next flush set
//! automatically added by Bevy (usually `CoreSet::UpdateFlush`, for systems
//! added to `CoreSet::Update`) but want to flush commands immediately.
//!
//! It is important that systems are ordered correctly with respect to
//! `apply_deferred`, to avoid surprising non-deterministic system execution order.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Timers>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    despawn_old_and_spawn_new_fruits,
                    // We encourage adding apply_deferred to a custom set
                    // to improve diagnostics. This is optional, but useful when debugging!
                    apply_deferred.in_set(CustomFlush),
                    count_apple,
                )
                    .chain(),
                count_orange,
                bevy::window::close_on_esc,
            ),
        )
        .run();
}

#[derive(Resource)]
struct Timers {
    repeating: Timer,
}

impl Default for Timers {
    fn default() -> Self {
        Self {
            repeating: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct CustomFlush;

#[derive(Component)]
struct Apple;

#[derive(Component)]
struct Orange;

#[derive(Component)]
struct AppleCount;

#[derive(Component)]
struct OrangeCount;

// Setup the counters in the UI.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "Apple: nothing counted yet".to_string(),
                    TextStyle {
                        font_size: 80.0,
                        color: Color::ORANGE,
                        ..default()
                    },
                ),
                AppleCount,
            ));
            parent.spawn((
                TextBundle::from_section(
                    "Orange: nothing counted yet".to_string(),
                    TextStyle {
                        font_size: 80.0,
                        color: Color::ORANGE,
                        ..default()
                    },
                ),
                OrangeCount,
            ));
        });
}

// Every tick, before the CustomFlush we added, we despawn any Apple and Orange
// we have previously spawned, if any. Then we tick the timer, and if the timer
// has finished during this tick, we spawn a new Apple and a new Orange.
//
// The commands that we have added here will normally be flushed by Bevy
// after all systems in the schedule have run, but because we have ordered
// this system to run before `apply_deferred.in_set(CustomFlush)`,
// these commands added here will be flushed during our custom flush.
fn despawn_old_and_spawn_new_fruits(
    mut commands: Commands,
    time: Res<Time>,
    mut timers: ResMut<Timers>,
    apple: Query<Entity, With<Apple>>,
    orange: Query<Entity, With<Orange>>,
) {
    if let Ok(apple_entity) = apple.get_single() {
        commands.entity(apple_entity).despawn();
    }

    if let Ok(orange_entity) = orange.get_single() {
        commands.entity(orange_entity).despawn();
    }

    timers.repeating.tick(time.delta());

    if timers.repeating.just_finished() {
        commands.spawn(Apple);
        commands.spawn(Orange);
    }
}

// If the timer has finished during this tick, we see if there is an entity
// with an Apple component or not, and update the UI accordingly.
//
// Since this system is ordered `.after(CustomFlush)` it will be guaranteed
// to run after our CustomFlush set, so the Apple will always be counted.
//
// We will see the AppleCount go from "Apple: nothing counted yet" to "Apple: counted"
fn count_apple(
    timers: Res<Timers>,
    apple: Query<&Apple>,
    mut apple_count: Query<&mut Text, With<AppleCount>>,
) {
    if timers.repeating.just_finished() {
        let mut apples_text = apple_count.single_mut();
        apples_text.sections[0].value = if apple.is_empty() {
            "Apple: not counted".to_string()
        } else {
            "Apple: counted".to_string()
        };
    }
}

// If the timer has finished during this tick, we see if there is an entity
// with an Orange component or not, and update the UI accordingly.
//
// Since this system is not ordered `.after(CustomFlush)`, it may or may not run
// before the custom flush, therefore you will see the UI either show "Orange: counted"
// or "Orange: not counted" or alternate between the two.
//
// Try to re-run the example multiple times as well.
fn count_orange(
    timers: Res<Timers>,
    orange: Query<&Orange>,
    mut orange_count: Query<&mut Text, With<OrangeCount>>,
) {
    if timers.repeating.just_finished() {
        let mut oranges_text = orange_count.single_mut();
        oranges_text.sections[0].value = if orange.is_empty() {
            "Orange: not counted".to_string()
        } else {
            "Orange: counted".to_string()
        };
    }
}
