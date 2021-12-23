use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    let texture = asset_server.load("branding/icon.png");

    // Spawn a root entity with no parent
    let parent = commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(0.75)),
            texture: texture.clone(),
            ..Default::default()
        })
        // With that entity as a parent, run a lambda that spawns its children
        .with_children(|parent| {
            // parent is a ChildBuilder, which has a similar API to Commands
            parent.spawn_bundle(SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(250.0, 0.0, 0.0),
                    scale: Vec3::splat(0.75),
                    ..Default::default()
                },
                texture: texture.clone(),
                sprite: Sprite {
                    color: Color::BLUE,
                    ..Default::default()
                },
                ..Default::default()
            });
        })
        // Store parent entity for next sections
        .id();

    // Another way to create a hierarchy is to add a Parent component to an entity,
    // which would be added automatically to parents with other methods.
    // Similarly, adding a Parent component will automatically add a Children component to the
    // parent.
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(-250.0, 0.0, 0.0),
                scale: Vec3::splat(0.75),
                ..Default::default()
            },
            texture: texture.clone(),
            sprite: Sprite {
                color: Color::RED,
                ..Default::default()
            },
            ..Default::default()
        })
        // Using the entity from the previous section as the parent:
        .insert(Parent(parent));

    // Another way is to use the push_children function to add children after the parent
    // entity has already been spawned.
    let child = commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 250.0, 0.0),
                scale: Vec3::splat(0.75),
                ..Default::default()
            },
            texture,
            sprite: Sprite {
                color: Color::GREEN,
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    // Pushing takes a slice of children to add:
    commands.entity(parent).push_children(&[child]);
}

// A simple system to rotate the root entity, and rotate all its children separately
fn rotate(
    mut commands: Commands,
    time: Res<Time>,
    mut parents_query: Query<(Entity, &Children), With<Sprite>>,
    mut transform_query: Query<&mut Transform, With<Sprite>>,
) {
    let angle = std::f32::consts::PI / 2.0;
    for (parent, children) in parents_query.iter_mut() {
        if let Ok(mut transform) = transform_query.get_mut(parent) {
            transform.rotate(Quat::from_rotation_z(-angle * time.delta_seconds()));
        }

        // To iterate through the entities children, just treat the Children component as a Vec
        // Alternatively, you could query entities that have a Parent component
        for child in children.iter() {
            if let Ok(mut transform) = transform_query.get_mut(*child) {
                transform.rotate(Quat::from_rotation_z(angle * 2.0 * time.delta_seconds()));
            }
        }

        // To demonstrate removing children, we'll start to remove the children after a couple of
        // seconds
        if time.seconds_since_startup() >= 2.0 && children.len() == 3 {
            let child = children.last().copied().unwrap();
            commands.entity(child).despawn_recursive();
        }

        if time.seconds_since_startup() >= 4.0 {
            // This will remove the entity from its parent's list of children, as well as despawn
            // any children the entity has.
            commands.entity(parent).despawn_recursive();
        }
    }
}
