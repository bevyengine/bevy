//! Demonstrates the use of "one-shot systems", which run once when triggered.
//!
//! These can be useful to help structure your logic in a push-based fashion,
//! reducing the overhead of running extremely rarely run systems
//! and improving schedule flexibility.
//!
//! See the [`World::run_system`](World::run_system) or
//! [`World::run_system_once`](World#method.run_system_once_with)
//! docs for more details.

use bevy::{
    ecs::system::{RunSystemOnce, SystemId},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (
                setup_ui,
                setup_with_commands,
                setup_with_world.after(setup_ui), // since we run `system_b` once in world it needs to run after `setup_ui`
            ),
        )
        .add_systems(Update, (trigger_system, evaluate_callbacks).chain())
        .run();
}

#[derive(Component)]
struct Callback(SystemId);

#[derive(Component)]
struct Triggered;

#[derive(Component)]
struct A;
#[derive(Component)]
struct B;

fn setup_with_commands(mut commands: Commands) {
    let system_id = commands.register_system(system_a);
    commands.spawn((Callback(system_id), A));
}

fn setup_with_world(world: &mut World) {
    // We can run it once manually
    world.run_system_once(system_b).unwrap();
    // Or with a Callback
    let system_id = world.register_system(system_b);
    world.spawn((Callback(system_id), B));
}

/// Tag entities that have callbacks we want to run with the `Triggered` component.
fn trigger_system(
    mut commands: Commands,
    query_a: Single<Entity, With<A>>,
    query_b: Single<Entity, With<B>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyA) {
        let entity = *query_a;
        commands.entity(entity).insert(Triggered);
    }
    if input.just_pressed(KeyCode::KeyB) {
        let entity = *query_b;
        commands.entity(entity).insert(Triggered);
    }
}

/// Runs the systems associated with each `Callback` component if the entity also has a `Triggered` component.
///
/// This could be done in an exclusive system rather than using `Commands` if preferred.
fn evaluate_callbacks(query: Query<(Entity, &Callback), With<Triggered>>, mut commands: Commands) {
    for (entity, callback) in query.iter() {
        commands.run_system(callback.0);
        commands.entity(entity).remove::<Triggered>();
    }
}

fn system_a(entity_a: Single<Entity, With<Text>>, mut writer: TextUiWriter) {
    *writer.text(*entity_a, 3) = String::from("A");
    info!("A: One shot system registered with Commands was triggered");
}

fn system_b(entity_b: Single<Entity, With<Text>>, mut writer: TextUiWriter) {
    *writer.text(*entity_b, 3) = String::from("B");
    info!("B: One shot system registered with World was triggered");
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::default(),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        },
        children![
            (TextSpan::new("Press A or B to trigger a one-shot system\n")),
            (TextSpan::new("Last Triggered: ")),
            (
                TextSpan::new("-"),
                TextColor(bevy::color::palettes::css::ORANGE.into()),
            )
        ],
    ));
}
