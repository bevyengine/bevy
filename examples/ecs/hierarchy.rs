//! Demonstrates techniques for creating a hierarchy of parent and child entities.
//!
//! When [`DefaultPlugins`] are added to your app, systems are automatically added to propagate
//! [`Transform`] and [`Visibility`] from parents to children down the hierarchy,
//! resulting in a final [`GlobalTransform`] and [`InheritedVisibility`] component for each entity.

use std::{f32::consts::*, time::Duration};

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .init_state::<Scene>()
        .insert_resource(Delta(Duration::ZERO))
        .add_systems(OnEnter(Scene::WithChildren), setup_with_children)
        .add_systems(OnEnter(Scene::ChildrenSpawn), setup_children_spawn)
        .add_systems(OnEnter(Scene::ChildrenMacro), spawn_children_macro)
        .add_systems(OnEnter(Scene::ChildrenIter), setup_children_iter)
        .add_systems(OnEnter(Scene::Related), setup_children_related)
        .add_systems(Update, (rotate, switch_scene))
        .run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
#[states(scoped_entities)]
enum Scene {
    #[default]
    WithChildren,
    ChildrenSpawn,
    ChildrenMacro,
    ChildrenIter,
    Related,
}

impl Scene {
    fn next(&self) -> Self {
        match self {
            Scene::WithChildren => Scene::ChildrenSpawn,
            Scene::ChildrenSpawn => Scene::ChildrenMacro,
            Scene::ChildrenMacro => Scene::ChildrenIter,
            Scene::ChildrenIter => Scene::Related,
            Scene::Related => Scene::WithChildren,
        }
    }
}

fn switch_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("Switching scene");
        next_scene.set(scene.get().next());
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Resource)]
struct Delta(Duration);

fn setup_common(
    commands: &mut Commands,
    time: &Res<Time>,
    delta: &mut ResMut<Delta>,
    title: &str,
    stage: Scene,
) {
    delta.0 = time.elapsed();
    commands.spawn((
        Text::new(title),
        TextFont {
            font: Default::default(),
            font_size: 36.,
            ..default()
        },
        DespawnOnExitState(stage),
    ));
}

fn setup_with_children(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut delta: ResMut<Delta>,
) {
    let texture = asset_server.load("branding/icon.png");

    setup_common(
        &mut commands,
        &time,
        &mut delta,
        "with_children()\nPress Space to continue",
        Scene::WithChildren,
    );

    // Spawn a root entity with no parent
    let parent = commands
        .spawn((
            Sprite::from_image(texture.clone()),
            Transform::from_scale(Vec3::splat(0.75)),
            DespawnOnExitState(Scene::WithChildren),
        ))
        // With that entity as a parent, run a lambda that spawns its children
        .with_children(|parent| {
            // parent is a ChildSpawnerCommands, which has a similar API to Commands
            parent.spawn((
                Transform::from_xyz(250.0, 0.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture.clone(),
                    color: BLUE.into(),
                    ..default()
                },
            ));
        })
        // Store parent entity for next sections
        .id();

    // Another way is to use the add_child function to add children after the parent
    // entity has already been spawned.
    let child = commands
        .spawn((
            Sprite {
                image: texture.clone(),
                color: LIME.into(),
                ..default()
            },
            Transform::from_xyz(0.0, 250.0, 0.0).with_scale(Vec3::splat(0.75)),
        ))
        .id();

    // Add child to the parent.
    commands.entity(parent).add_child(child);
}

fn setup_children_spawn(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut delta: ResMut<Delta>,
) {
    let texture = asset_server.load("branding/icon.png");

    setup_common(
        &mut commands,
        &time,
        &mut delta,
        "Children::spawn() \nPress Space to continue",
        Scene::ChildrenSpawn,
    );

    // Children can also be spawned using the `Children` component as part of the parent's bundle.
    commands.spawn((
        Sprite::from_image(texture.clone()),
        Transform::from_scale(Vec3::splat(0.75)),
        DespawnOnExitState(Scene::ChildrenSpawn),
        Children::spawn((
            Spawn((
                Transform::from_xyz(250.0, 0.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture.clone(),
                    color: BLUE.into(),
                    ..default()
                },
            )),
            Spawn((
                Transform::from_xyz(0.0, 250.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture,
                    color: LIME.into(),
                    ..default()
                },
            )),
        )),
    ));
}

fn spawn_children_macro(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut delta: ResMut<Delta>,
) {
    let texture = asset_server.load("branding/icon.png");

    setup_common(
        &mut commands,
        &time,
        &mut delta,
        "children!() \nPress Space to continue",
        Scene::ChildrenMacro,
    );

    // The `children!` macro provides a convenient way to define children inline with their parent.
    commands.spawn((
        Sprite::from_image(texture.clone()),
        Transform::from_scale(Vec3::splat(0.75)),
        DespawnOnExitState(Scene::ChildrenMacro),
        children![
            (
                Transform::from_xyz(250.0, 0.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture.clone(),
                    color: BLUE.into(),
                    ..default()
                },
            ),
            (
                Transform::from_xyz(0.0, 250.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture,
                    color: LIME.into(),
                    ..default()
                },
            )
        ],
    ));
}

fn setup_children_iter(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut delta: ResMut<Delta>,
) {
    let texture = asset_server.load("branding/icon.png");

    setup_common(
        &mut commands,
        &time,
        &mut delta,
        "SpawnIter() \nPress Space to continue",
        Scene::ChildrenIter,
    );

    // You can also spawn children from an iterator yielding bundles.
    let child_components = [
        (
            Transform::from_xyz(250.0, 0.0, 0.0).with_scale(Vec3::splat(0.75)),
            BLUE,
        ),
        (
            Transform::from_xyz(0.0, 250.0, 0.0).with_scale(Vec3::splat(0.75)),
            LIME,
        ),
    ];

    commands.spawn((
        Sprite::from_image(texture.clone()),
        Transform::from_scale(Vec3::splat(0.75)),
        DespawnOnExitState(Scene::ChildrenIter),
        Children::spawn(SpawnIter(child_components.into_iter().map(
            move |(transform, color)| {
                (
                    transform,
                    Sprite {
                        image: texture.clone(),
                        color: color.into(),
                        ..default()
                    },
                )
            },
        ))),
    ));
}

fn setup_children_related(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut delta: ResMut<Delta>,
) {
    let texture = asset_server.load("branding/icon.png");

    setup_common(
        &mut commands,
        &time,
        &mut delta,
        "related!() \nPress Space to continue",
        Scene::Related,
    );

    // You can also spawn entities with relationships other than parent/child.
    commands.spawn((
        Sprite::from_image(texture.clone()),
        Transform::from_scale(Vec3::splat(0.75)),
        DespawnOnExitState(Scene::Related),
        // the `related!` macro will spawn entities according to the `Children: RelationshipTarget` trait, but other types implementing `RelationshipTarget` can be used as well.
        related!(Children[
            (
                Transform::from_xyz(250.0, 0.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture.clone(),
                    color: BLUE.into(),
                    ..default()
                },
            ),
            (
                Transform::from_xyz(0.0, 250.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture,
                    color: LIME.into(),
                    ..default()
                },
            )
        ]),
    ));
}

// A simple system to rotate the root entity, and rotate all its children separately
fn rotate(
    mut commands: Commands,
    time: Res<Time>,
    delta: Res<Delta>,
    mut parents_query: Query<(Entity, &Children), With<Sprite>>,
    mut transform_query: Query<&mut Transform, With<Sprite>>,
) {
    for (parent, children) in &mut parents_query {
        if let Ok(mut transform) = transform_query.get_mut(parent) {
            transform.rotate_z(-PI / 2. * time.delta_secs());
        }

        // To iterate through the entities children, just treat the Children component as a Vec
        // Alternatively, you could query entities that have a ChildOf component
        for child in children {
            if let Ok(mut transform) = transform_query.get_mut(*child) {
                transform.rotate_z(PI * time.delta_secs());
            }
        }

        // To demonstrate removing children, we'll remove a child after a couple of seconds.
        let elapsed = time.elapsed() - delta.0;
        if elapsed.as_secs_f32() >= 2.0 && children.len() == 2 {
            let child = children.last().unwrap();
            commands.entity(*child).despawn();
        }

        if elapsed.as_secs_f32() >= 4.0 {
            // This will remove the entity from its parent's list of children, as well as despawn
            // any children the entity has.
            commands.entity(parent).despawn();
        }
    }
}
