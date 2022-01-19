use bevy::{input::gamepad::GamepadSettings, prelude::*};

const WINDOW_SIZE: f32 = 300.0;
const CROSSHAIR_SIZE: f32 = 16.0;
const FONT: &str = "fonts/FiraMono-Medium.ttf";
const FONT_SIZE: f32 = 18.0;
const LIVEZONE_COLOR: Color = Color::GRAY;
const DEADZONE_COLOR: Color = Color::rgb(0.4, 0.4, 0.4);

#[derive(Component)]
struct Crosshair;

#[derive(Component)]
struct CoordinateText;

#[derive(Component)]
struct DeadzoneBox;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Gamepad Stick Input".to_owned(),
            width: WINDOW_SIZE,
            height: WINDOW_SIZE,
            ..Default::default()
        })
        .insert_resource(ClearColor(DEADZONE_COLOR))
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gamepad_settings: Res<GamepadSettings>,
) {
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
            // Make sure it is in the foreground with a Z value > 0.0
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .insert(Crosshair);

    // Get live/deadzone info
    let livezone_upperbound = gamepad_settings.default_axis_settings.positive_high;
    let livezone_lowerbound = gamepad_settings.default_axis_settings.negative_high;
    let deadzone_upperbound = gamepad_settings.default_axis_settings.positive_low;
    let deadzone_lowerbound = gamepad_settings.default_axis_settings.negative_low;
    let livezone_midpoint = (livezone_lowerbound + livezone_upperbound) / 2.0;
    let deadzone_midpoint = (deadzone_lowerbound + deadzone_upperbound) / 2.0;
    let livezone_size = livezone_upperbound - livezone_lowerbound;
    let deadzone_size = deadzone_upperbound - deadzone_lowerbound;
    let livezone_box_midpoint = livezone_midpoint * WINDOW_SIZE / 2.0;
    let deadzone_box_midpoint = deadzone_midpoint * WINDOW_SIZE / 2.0;
    let livezone_box_size = livezone_size * WINDOW_SIZE / 2.0;
    let deadzone_box_size = deadzone_size * WINDOW_SIZE / 2.0;
    // For text placement
    let livezone_lower_left_corner = (livezone_box_midpoint - livezone_box_size) / 2.0;

    // Spawn livezone box
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(livezone_box_size, livezone_box_size)),
            color: LIVEZONE_COLOR,
            ..Default::default()
        },
        transform: Transform::from_xyz(livezone_box_midpoint, livezone_box_midpoint, 0.0),
        ..Default::default()
    });
    // Spawn deadzone box
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(deadzone_box_size, deadzone_box_size)),
                color: DEADZONE_COLOR,
                ..Default::default()
            },
            transform: Transform::from_xyz(deadzone_box_midpoint, deadzone_box_midpoint, 0.1),
            ..Default::default()
        })
        .insert(DeadzoneBox);

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
            text: Text::with_section("( 0.000,  0.000)", text_style, text_alignment),
            transform: Transform::from_xyz(
                livezone_lower_left_corner,
                livezone_lower_left_corner,
                1.0,
            ),
            ..Default::default()
        })
        .insert(CoordinateText);
}
