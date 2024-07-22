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
    let system_id = commands.register_one_shot_system(system_a);
    commands.spawn((Callback(system_id), A));
}

fn setup_with_world(world: &mut World) {
    // We can run it once manually
    world.run_system_once(system_b);
    // Or with a Callback
    let system_id = world.register_system(system_b);
    world.spawn((Callback(system_id), B));
}

/// Tag entities that have callbacks we want to run with the `Triggered` component.
fn trigger_system(
    mut commands: Commands,
    query_a: Query<Entity, With<A>>,
    query_b: Query<Entity, With<B>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyA) {
        let entity = query_a.single();
        commands.entity(entity).insert(Triggered);
    }
    if input.just_pressed(KeyCode::KeyB) {
        let entity = query_b.single();
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

fn system_a(mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[2].value = String::from("A");
    info!("A: One shot system registered with Commands was triggered");
}

fn system_b(mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[2].value = String::from("B");
    info!("B: One shot system registered with World was triggered");
}

fn setup_ui(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_sections([
            TextSection::new(
                "Press A or B to trigger a one-shot system\n",
                TextStyle::default(),
            ),
            TextSection::new("Last Triggered: ", TextStyle::default()),
            TextSection::new(
                "-",
                TextStyle {
                    color: bevy::color::palettes::css::ORANGE.into(),
                    ..default()
                },
            ),
        ])
        .with_text_justify(JustifyText::Center)
        .with_style(Style {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        }),
    );
}
