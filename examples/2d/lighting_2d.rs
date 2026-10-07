//! Illustrates lighting in 2d.

use bevy::{color::palettes::css::*, math::ops, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GlobalAmbientLight2d {
            brightness: 0.1,
            ..default()
        })
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, (toggle_ambient_light, movement, orbit))
        .run();
}

#[derive(Component, Default, Clone)]
struct Movable;

#[derive(Component, Default, Clone)]
struct Orbit {
    center: Vec2,
    radius: Vec2,
    speed: f32,
}

fn scene() -> impl SceneList {
    bsn_list! {
        // We need a background, as the clear color is not lit.
        Mesh2d(asset_value(Rectangle::new(4000.0, 4000.0)))
        MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(Color::WHITE)))
        --
        Sprite { image: "branding/icon.png" }
        Transform::from_xyz(0.0, 0.0, 1.0)
        Movable
        --
        PointLight2d { color: RED, range: 300.0 }
        Transform::from_xyz(-200.0, 100.0, 0.0)
        --
        PointLight2d { color: LIME, range: 300.0 }
        Orbit {
            center: Vec2::new(50.0, -30.0),
            radius: Vec2::new(280.0, 160.0),
            speed: 0.8,
        }
        --
        PointLight2d { color: BLUE, range: 300.0 }
        Orbit {
            center: Vec2::new(150.0, 40.0),
            radius: Vec2::new(180.0, 120.0),
            speed: -1.3,
        }
        --
        Text
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
        }
        Children [
            TextSpan("Ambient light is on\n")
            --
            TextSpan("\n")
            --
            TextSpan("Controls\n")
            --
            TextSpan("---------------\n")
            --
            TextSpan("Arrow keys - Move objects\n")
            --
            TextSpan("Space - Toggle ambient light")
        ]
        --
        Camera2d
    }
}

fn toggle_ambient_light(
    key_input: Res<ButtonInput<KeyCode>>,
    mut ambient_light: ResMut<GlobalAmbientLight2d>,
    text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    if key_input.just_pressed(KeyCode::Space) {
        if ambient_light.brightness > 0. {
            ambient_light.brightness = 0.;
        } else {
            ambient_light.brightness = 0.1;
        }

        let entity = *text;
        let ambient_light_state_text: &str = match ambient_light.brightness {
            0. => "off",
            _ => "on",
        };
        *writer.text(entity, 1) = format!("Ambient light is {ambient_light_state_text}\n");
    }
}

fn orbit(time: Res<Time>, mut query: Query<(&mut Transform, &Orbit)>) {
    for (mut transform, orbit) in &mut query {
        let angle = time.elapsed_secs() * orbit.speed;
        let offset = Vec2::new(ops::cos(angle), ops::sin(angle)) * orbit.radius;
        transform.translation = (orbit.center + offset).extend(0.0);
    }
}

fn movement(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Movable>>,
) {
    let mut direction = Vec3::ZERO;
    if input.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if input.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if input.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if input.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    for mut transform in &mut query {
        transform.translation += time.delta_secs() * 300.0 * direction;
    }
}
