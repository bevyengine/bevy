use bevy::prelude::*;

const WINDOW_SIZE: f32 = 300.0;
const CROSSHAIR_SIZE: f32 = 24.0;
const FONT: &str = "fonts/FiraMono-Medium.ttf";
const FONT_SIZE: f32 = 18.0;

#[derive(Component)]
struct Crosshair;

#[derive(Component)]
struct CoordinateText;

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
        .add_system(update_position)
        .run();
}

fn update_position(
    mut xform_query: Query<&mut Transform, With<Crosshair>>,
    mut text_query: Query<&mut Text, With<CoordinateText>>,
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
) {
    let mut xform = xform_query.single_mut();
    let mut text = text_query.single_mut();
    for gamepad in gamepads.iter() {
        // We only use input from the left stick.
        let x = axes
            .get(GamepadAxis(*gamepad, GamepadAxisType::LeftStickX))
            .unwrap();
        let y = axes
            .get(GamepadAxis(*gamepad, GamepadAxisType::LeftStickY))
            .unwrap();
        xform.translation.x = x * WINDOW_SIZE / 2.0;
        xform.translation.y = y * WINDOW_SIZE / 2.0;
        text.sections[0].value = format!("({:6.3}, {:6.3})", x, y);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn cameras
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
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

    // Spawn text
    let font = asset_server.load(FONT);
    let text_style = TextStyle {
        font,
        font_size: FONT_SIZE,
        color: Color::BLACK,
    };
    let text_alignment = TextAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: HorizontalAlign::Left,
    };
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::with_section("(0.000, 0.000)", text_style, text_alignment),
            transform: Transform::from_xyz(-WINDOW_SIZE / 2.0, -WINDOW_SIZE / 2.0, 0.0),
            ..Default::default()
        })
        .insert(CoordinateText);
}
