use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(spawn_player)
        .add_startup_system(spawn_enemy)
        // Exclusive needed because we are yoinking the whole world
        // Needs to happen at the end of startup because a player needs to exist
        .add_startup_system(create_scene.exclusive_system().at_end())
        .run();
}

fn create_scene(world: &mut World) {
    // quick: make a scene with all entities that match a given query filter
    // (all components will be included)
    let my_scene: DynamicScene =
        DynamicScene::from_query_filter::<(With<Player>, Without<Enemy>)>(world);

    // This should print out 1 because the player was saved to the scene
    println!("{}", my_scene.entities.len());
}

#[derive(Component)]
#[allow(dead_code)]
struct Player {
    speed: f32,
}

fn spawn_player(mut commands: Commands) {
    commands.spawn().insert_bundle(Camera2dBundle::default());

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(40.0, 40.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player { speed: 300.0 });
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Enemy {
    // Speed is always positive
    speed: f32,
}

fn spawn_enemy(mut commands: Commands) {
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::MAROON,
                custom_size: Some(Vec2::new(40.0, 40.0)),
                ..Default::default()
            },
            transform: Transform::from_xyz(300.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(Enemy { speed: 200.0 });
}
