use bevy::{prelude::*, render::camera::WindowOrigin};

/// This example shows how to manipulate the UI camera projection and position.
/// Controls:
/// * Arrow keys: move UI camera around,
/// * Left/Right mouse buttons: zoom in/out
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(cam_system)
        .add_system(button_system)
        .run();
}

#[derive(Component)]
struct IdleColor(UiColor);

// UI Camera data are modifyed through the `UiCameraConfig` component.
// Note that you must insert one to your screen camera.
fn cam_system(
    mut cam_configs: Query<&mut UiCameraConfig>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
) {
    for mut cam_config in &mut cam_configs {
        let mut offset = match () {
            () if keyboard.pressed(KeyCode::Left) => -Vec2::X,
            () if keyboard.pressed(KeyCode::Right) => Vec2::X,
            () => Vec2::ZERO,
        };
        offset += match () {
            () if keyboard.pressed(KeyCode::Down) => -Vec2::Y,
            () if keyboard.pressed(KeyCode::Up) => Vec2::Y,
            () => Vec2::ZERO,
        };
        // We only modify the transform when there is a change, so as
        // to not trigger change detection.
        if offset != Vec2::ZERO {
            let scale = cam_config.scale;
            cam_config.position += offset * scale * 30.0;
        }
        let scale_offset = match () {
            () if mouse.pressed(MouseButton::Left) => 0.9,
            () if mouse.pressed(MouseButton::Right) => 1.1,
            () => 0.0,
        };
        if scale_offset != 0.0 {
            cam_config.scale *= scale_offset;
        }
    }
}

fn button_system(
    mut interaction_query: Query<(&Interaction, &mut UiColor, &IdleColor), Changed<Interaction>>,
) {
    for (interaction, mut material, IdleColor(idle_color)) in &mut interaction_query {
        if let Interaction::Hovered = interaction {
            *material = Color::WHITE.into();
        } else {
            *material = *idle_color;
        }
    }
}

fn setup(mut commands: Commands) {
    let button_row_count = 35;
    let as_rainbow = |i: u32| Color::hsl((i as f32 / button_row_count as f32) * 360.0, 0.9, 0.5);
    commands
        .spawn_bundle(Camera2dBundle::default())
        // Insert a UiCameraConfig to customize the UI camera.
        .insert(UiCameraConfig {
            window_origin: WindowOrigin::Center,
            ..default()
        });
    for i in 0..button_row_count {
        for j in 0..button_row_count {
            let full = (i + j).max(1);
            let color = as_rainbow((i * j) % full).into();
            spawn_button(&mut commands, color, button_row_count, i, j);
        }
    }
}
fn spawn_button(commands: &mut Commands, color: UiColor, max: u32, i: u32, j: u32) {
    let size = 340.0 / max as f32;
    commands
        .spawn_bundle(ButtonBundle {
            color,
            style: Style {
                size: Size::new(Val::Percent(size), Val::Percent(size)),
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Percent((400.0 / max as f32) * i as f32),
                    left: Val::Percent((400.0 / max as f32) * j as f32),
                    ..default()
                },
                ..default()
            },
            ..default()
        })
        .insert(IdleColor(color));
}
