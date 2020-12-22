use bevy::{prelude::*, scene::InstanceId};

fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_resource(SceneInstance::default())
        .add_startup_system(setup.system())
        .add_system(scene_update.system())
        .add_system(list_scene_entities.system())
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
            transform: Transform::from_translation(Vec3::new(0.7, 0.7, 1.0))
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
            if scene_spawner.instance_is_ready(instance_id) {
                scene_spawner.for_entity_in_scene_instance(instance_id, |entity| {
                    commands.insert_one(entity, EntityInMyScene);
                });
                *done = true;
            }
        }
    }
}

// This system will list all entities with component `EntityInMyScene`, so all
// entities from the second scene
fn list_scene_entities(scene_entities: Query<Entity, With<EntityInMyScene>>) {
    for entity in scene_entities.iter() {
        eprintln!("{:?}", entity);
    }
}
