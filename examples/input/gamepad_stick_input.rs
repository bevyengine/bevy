use bevy::{input::gamepad::GamepadSettings, prelude::*};

const CROSSHAIR_SIZE: f32 = 16.0;
const FONT: &str = "fonts/FiraMono-Medium.ttf";
const FONT_SIZE: f32 = 18.0;
const LIVEZONE_COLOR: Color = Color::GRAY;
const DEADZONE_COLOR: Color = Color::rgb(0.4, 0.4, 0.4);

#[derive(Component)]
struct Crosshair;

#[derive(Component)]
struct CoordinateText;

#[derive(Resource)]
struct VisualizationSize {
    size: f32,
}

impl Default for VisualizationSize {
    fn default() -> Self {
        VisualizationSize { size: 300.0 }
    }
}

fn main() {
    App::new()
        .insert_resource(VisualizationSize::default())
        .insert_resource(ClearColor(DEADZONE_COLOR))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_position)
        .run();
}

fn update_position(
    mut crosshair_query: Query<&mut Transform, With<Crosshair>>,
    mut text_query: Query<&mut Text, With<CoordinateText>>,
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
    visualization_size: Res<VisualizationSize>,
) {
    let mut transform = crosshair_query.single_mut();
    let mut text = text_query.single_mut();
    for gamepad in gamepads.iter() {
        // We only use input from the left stick.
        let x = axes
            .get(GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickX,
            })
            .unwrap();
        let y = axes
            .get(GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::LeftStickY,
            })
            .unwrap();
        transform.translation.x = x * visualization_size.size / 2.0;
        transform.translation.y = y * visualization_size.size / 2.0;
        text.sections[0].value = format!("({:6.3}, {:6.3})", x, y);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gamepad_settings: Res<GamepadSettings>,
    mut visualization_window: ResMut<VisualizationSize>,
    mut windows: ResMut<Windows>,
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_title("Gamepad Stick Input".to_owned());

    // The monitor's size
    let physical_width = window.physical_width();
    let physical_height = window.physical_height();

    // The scaling factor the OS uses to account for pixel density or user preference
    let scale_factor = window.scale_factor();

    // The resizing constraints that limit the resolution we can set
    let constraints = window.resize_constraints();

    let min_physical_dimension: f32 =
        (physical_height.min(physical_width) as f64 * scale_factor) as f32;
    let min_constraint_dimension = constraints.max_height.min(constraints.max_width);

    // Keep the size of the window within constraints
    let min_dimension = min_physical_dimension.min(min_constraint_dimension);

    window.set_resolution(min_dimension, min_dimension);
    visualization_window.size = min_dimension;

    // Spawn camera
    commands.spawn_bundle(Camera2dBundle::default());

    // Spawn crosshair
    let texture = asset_server.load("textures/crosshair.png");
    commands
        .spawn_bundle(SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(CROSSHAIR_SIZE)),
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
    let livezone_box_midpoint = livezone_midpoint * min_dimension / 2.0;
    let deadzone_box_midpoint = deadzone_midpoint * min_dimension / 2.0;
    let livezone_box_size = livezone_size * min_dimension / 2.0;
    let deadzone_box_size = deadzone_size * min_dimension / 2.0;
    // For text placement
    let livezone_lower_left_corner = (livezone_box_midpoint - livezone_box_size) / 2.0;

    // Spawn livezone box
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::splat(livezone_box_size)),
            color: LIVEZONE_COLOR,
            ..Default::default()
        },
        transform: Transform::from_xyz(livezone_box_midpoint, livezone_box_midpoint, 0.0),
        ..Default::default()
    });
    // Spawn deadzone box
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::splat(deadzone_box_size)),
            color: DEADZONE_COLOR,
            ..Default::default()
        },
        transform: Transform::from_xyz(deadzone_box_midpoint, deadzone_box_midpoint, 0.1),
        ..Default::default()
    });

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
            text: Text::from_section("( 0.000,  0.000)", text_style).with_alignment(text_alignment),
            transform: Transform::from_xyz(
                livezone_lower_left_corner,
                livezone_lower_left_corner,
                1.0,
            ),
            ..Default::default()
        })
        .insert(CoordinateText);
}
