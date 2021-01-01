use bevy::{prelude::*, scene::InstanceId};

fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_resource(SceneInstance::default())
        .add_startup_system(setup.system())
        .add_system(scene_update.system())
        .add_system(move_scene_entities.system())
        .run();
}

// Resource to hold the scene `instance_id` until it is loaded
#[derive(Default)]
struct SceneInstance(Option<InstanceId>);

// Component that will be used to tag entities in the scene
struct EntityInMyScene;

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut scene_spawner: ResMut<SceneSpawner>,
    mut scene_instance: ResMut<SceneInstance>,
) {
    commands
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, 4.0)),
            ..Default::default()
        })
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(1.05, 0.9, 1.5))
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::unit_y()),
            ..Default::default()
        });

    // Spawn the scene as a child of another entity. This first scene will be translated backward
    // with its parent
    commands
        .spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn_scene(asset_server.load("models/FlightHelmet/FlightHelmet.gltf"));
        });

    // Spawn a second scene, and keep its `instance_id`
    let instance_id =
        scene_spawner.spawn(asset_server.load("models/FlightHelmet/FlightHelmet.gltf"));
    scene_instance.0 = Some(instance_id);
}

// This system will wait for the scene to be ready, and then tag entities from
// the scene with `EntityInMyScene`. All entities from the second scene will be
// tagged
fn scene_update(
    commands: &mut Commands,
    scene_spawner: Res<SceneSpawner>,
    scene_instance: Res<SceneInstance>,
    mut done: Local<bool>,
) {
    if !*done {
        if let Some(instance_id) = scene_instance.0 {
            if let Some(entity_iter) = scene_spawner.iter_instance_entities(instance_id) {
                entity_iter.for_each(|entity| {
                    commands.insert_one(entity, EntityInMyScene);
                });
                *done = true;
            }
        }
    }
}

// This system will move all entities with component `EntityInMyScene`, so all
// entities from the second scene
fn move_scene_entities(
    time: Res<Time>,
    mut scene_entities: Query<&mut Transform, With<EntityInMyScene>>,
) {
    let mut direction = 1.;
    let mut scale = 1.;
    for mut transform in scene_entities.iter_mut() {
        transform.translation = Vec3::new(
            scale * direction * time.seconds_since_startup().sin() as f32 / 20.,
            0.,
            time.seconds_since_startup().cos() as f32 / 20.,
        );
        direction *= -1.;
        scale += 0.5;
    }
}
