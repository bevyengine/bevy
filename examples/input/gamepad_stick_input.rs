use bevy::prelude::*;

const WINDOW_SIZE: f32 = 300.0;
const CROSSHAIR_SIZE: f32 = 32.0;

#[derive(Component)]
struct Crosshair;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Gamepad Stick Input".to_owned(),
            width: WINDOW_SIZE,
            height: WINDOW_SIZE,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::GRAY))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_crosshair)
        .run();
}

fn move_crosshair(
    mut query: Query<&mut Transform, With<Crosshair>>,
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
) {
    let mut xform = query.single_mut();
    for gamepad in gamepads.iter() {
        let left_stick_x = axes
            .get(GamepadAxis(*gamepad, GamepadAxisType::LeftStickX))
            .unwrap();
        let left_stick_y = axes
            .get(GamepadAxis(*gamepad, GamepadAxisType::LeftStickY))
            .unwrap();
        xform.translation.x = left_stick_x * WINDOW_SIZE / 2.0;
        xform.translation.y = left_stick_y * WINDOW_SIZE / 2.0;
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // Spawn crosshair
    let texture = asset_server.load("textures/crosshair.png");
    commands
        .spawn_bundle(SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::new(CROSSHAIR_SIZE, CROSSHAIR_SIZE)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Crosshair);
}
