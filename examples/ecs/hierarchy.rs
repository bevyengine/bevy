use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(rotate.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dComponents::default());
    let texture = asset_server.load("branding/icon.png");

    // Spawn a root entity with no parent
    let parent = commands
        .spawn(SpriteComponents {
            transform: Transform::from_scale(Vec3::splat(0.75)),
            material: materials.add(ColorMaterial {
                color: Color::WHITE,
                texture: Some(texture.clone()),
            }),
            ..Default::default()
        })
        // With that entity as a parent, run a lambda that spawns its children
        .with_children(|parent| {
            // parent is a ChildBuilder, which has a similar API to Commands
            parent.spawn(SpriteComponents {
                transform: Transform {
                    translation: Vec3::new(250.0, 0.0, 0.0),
                    scale: Vec3::splat(0.75),
                    ..Default::default()
                },
                material: materials.add(ColorMaterial {
                    color: Color::BLUE,
                    texture: Some(texture.clone()),
                }),
                ..Default::default()
            });
        })
        // Store parent entity for next sections
        .current_entity()
        .unwrap();

    // Another way to create a hierarchy is to add a Parent component to an entity,
    // which would be added automatically to parents with other methods.
    // Similarly, adding a Parent component will automatically add a Children component to the parent.
    commands
        .spawn(SpriteComponents {
            transform: Transform {
                translation: Vec3::new(-250.0, 0.0, 0.0),
                scale: Vec3::splat(0.75),
                ..Default::default()
            },
            material: materials.add(ColorMaterial {
                color: Color::RED,
                texture: Some(texture.clone()),
            }),
            ..Default::default()
        })
        // Using the entity from the previous section as the parent:
        .with(Parent(parent));

    // Another way is to use the push_children function to add children after the parent
    // entity has already been spawned.
    let child = commands
        .spawn(SpriteComponents {
            transform: Transform {
                translation: Vec3::new(0.0, 250.0, 0.0),
                scale: Vec3::splat(0.75),
                ..Default::default()
            },
            material: materials.add(ColorMaterial {
                color: Color::GREEN,
                texture: Some(texture),
            }),
            ..Default::default()
        })
        .current_entity()
        .unwrap();

    // Pushing takes a slice of children to add:
    commands.push_children(parent, &[child]);
}

// A simple system to rotate the root entity, and rotate all its children separately
fn rotate(
    mut commands: Commands,
    time: Res<Time>,
    mut parents_query: Query<(Entity, &mut Children, &Sprite)>,
    mut transform_query: Query<With<Sprite, &mut Transform>>,
) {
    let angle = std::f32::consts::PI / 2.0;
    for (parent, mut children, _) in parents_query.iter_mut() {
        if let Ok(mut transform) = transform_query.entity_mut(parent) {
            transform.rotate(Quat::from_rotation_z(-angle * time.delta_seconds));
        }

        // To iterate through the entities children, just treat the Children component as a Vec
        // Alternatively, you could query entities that have a Parent component
        for child in children.iter() {
            if let Ok(mut transform) = transform_query.entity_mut(*child) {
                transform.rotate(Quat::from_rotation_z(angle * 2.0 * time.delta_seconds));
            }
        }

        // To demonstrate removing children, we'll start to remove the children after a couple of seconds
        if time.seconds_since_startup >= 2.0 && children.len() == 3 {
            // Using .despawn() on an entity does not remove it from its parent's list of children!
            // It must be done manually if using .despawn()
            // NOTE: This is a bug. Eventually Bevy will update the children list automatically
            let child = children.pop().unwrap();
            commands.despawn(child);
        }

        if time.seconds_since_startup >= 4.0 {
            // This will remove the entity from its parent's list of children, as well as despawn
            // any children the entity has.
            commands.despawn_recursive(parent);
        }
    }
}
