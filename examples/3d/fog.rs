//! This interactive example shows how to use distance fog,
//! and allows playing around with different fog settings.
//!
//! ## Controls
//!
//! | Key Binding        | Action                              |
//! |:-------------------|:------------------------------------|
//! | `1` / `2` / `3`    | Fog Falloff Mode                    |
//! | `A` / `S`          | Move Start Distance (Linear Fog)    |
//! |                    | Change Density (Exponential Fogs)   |
//! | `Z` / `X`          | Move End Distance (Linear Fog)      |
//! | `-` / `=`          | Adjust Fog Red Channel              |
//! | `[` / `]`          | Adjust Fog Green Channel            |
//! | `;` / `'`          | Adjust Fog Blue Channel             |
//! | `.` / `?`          | Adjust Fog Alpha Channel            |

use bevy::{
    math::ops,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

fn main() {
    App::new()
        .insert_resource(AmbientLight::NONE)
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (setup_camera_fog, setup_pyramid_scene, setup_instructions),
        )
        .add_systems(Update, update_system)
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        DistanceFog {
            color: Color::srgb(0.25, 0.25, 0.25),
            falloff: FogFalloff::Linear {
                start: 5.0,
                end: 20.0,
            },
            ..default()
        },
    ));
}

fn setup_pyramid_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stone = materials.add(StandardMaterial {
        base_color: Srgba::hex("28221B").unwrap().into(),
        perceptual_roughness: 1.0,
        ..default()
    });

    // pillars
    for (x, z) in &[(-1.5, -1.5), (1.5, -1.5), (1.5, 1.5), (-1.5, 1.5)] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 3.0, 1.0))),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(*x, 1.5, *z),
        ));
    }

    // orb
    commands.spawn((
        Mesh3d(meshes.add(Sphere::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("126212CC").unwrap().into(),
            reflectance: 1.0,
            perceptual_roughness: 0.0,
            metallic: 0.5,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(1.75)).with_translation(Vec3::new(0.0, 4.0, 0.0)),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // steps
    for i in 0..50 {
        let half_size = i as f32 / 2.0 + 3.0;
        let y = -i as f32 / 2.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.0 * half_size, 0.5, 2.0 * half_size))),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(0.0, y + 0.25, 0.0),
        ));
    }

    // sky
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("888888").unwrap().into(),
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(1_000_000.0)),
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn update_system(
    camera: Single<(&mut DistanceFog, &mut Transform)>,
    mut text: Single<&mut Text>,
    time: Res<Time>,
    keycode: Res<ButtonInput<KeyCode>>,
) {
    let now = time.elapsed_secs();
    let delta = time.delta_secs();

    let (mut fog, mut transform) = camera.into_inner();

    // Orbit camera around pyramid
    let orbit_scale = 8.0 + ops::sin(now / 10.0) * 7.0;
    *transform = Transform::from_xyz(
        ops::cos(now / 5.0) * orbit_scale,
        12.0 - orbit_scale / 2.0,
        ops::sin(now / 5.0) * orbit_scale,
    )
    .looking_at(Vec3::ZERO, Vec3::Y);

    // Fog Information
    text.0 = format!("Fog Falloff: {:?}\nFog Color: {:?}", fog.falloff, fog.color);

    // Fog Falloff Mode Switching
    text.push_str("\n\n1 / 2 / 3 - Fog Falloff Mode");

    if keycode.pressed(KeyCode::Digit1) {
        if let FogFalloff::Linear { .. } = fog.falloff {
            // No change
        } else {
            fog.falloff = FogFalloff::Linear {
                start: 5.0,
                end: 20.0,
            };
        };
    }

    if keycode.pressed(KeyCode::Digit2) {
        if let FogFalloff::Exponential { .. } = fog.falloff {
            // No change
        } else if let FogFalloff::ExponentialSquared { density } = fog.falloff {
            fog.falloff = FogFalloff::Exponential { density };
        } else {
            fog.falloff = FogFalloff::Exponential { density: 0.07 };
        };
    }

    if keycode.pressed(KeyCode::Digit3) {
        if let FogFalloff::Exponential { density } = fog.falloff {
            fog.falloff = FogFalloff::ExponentialSquared { density };
        } else if let FogFalloff::ExponentialSquared { .. } = fog.falloff {
            // No change
        } else {
            fog.falloff = FogFalloff::Exponential { density: 0.07 };
        };
    }

    // Linear Fog Controls
    if let FogFalloff::Linear {
        ref mut start,
        ref mut end,
    } = &mut fog.falloff
    {
        text.push_str("\nA / S - Move Start Distance\nZ / X - Move End Distance");

        if keycode.pressed(KeyCode::KeyA) {
            *start -= delta * 3.0;
        }
        if keycode.pressed(KeyCode::KeyS) {
            *start += delta * 3.0;
        }
        if keycode.pressed(KeyCode::KeyZ) {
            *end -= delta * 3.0;
        }
        if keycode.pressed(KeyCode::KeyX) {
            *end += delta * 3.0;
        }
    }

    // Exponential Fog Controls
    if let FogFalloff::Exponential { ref mut density } = &mut fog.falloff {
        text.push_str("\nA / S - Change Density");

        if keycode.pressed(KeyCode::KeyA) {
            *density -= delta * 0.5 * *density;
            if *density < 0.0 {
                *density = 0.0;
            }
        }
        if keycode.pressed(KeyCode::KeyS) {
            *density += delta * 0.5 * *density;
        }
    }

    // ExponentialSquared Fog Controls
    if let FogFalloff::ExponentialSquared { ref mut density } = &mut fog.falloff {
        text.push_str("\nA / S - Change Density");

        if keycode.pressed(KeyCode::KeyA) {
            *density -= delta * 0.5 * *density;
            if *density < 0.0 {
                *density = 0.0;
            }
        }
        if keycode.pressed(KeyCode::KeyS) {
            *density += delta * 0.5 * *density;
        }
    }

    // RGBA Controls
    text.push_str("\n\n- / = - Red\n[ / ] - Green\n; / ' - Blue\n. / ? - Alpha");

    // We're performing various operations in the sRGB color space,
    // so we convert the fog color to sRGB here, then modify it,
    // and finally when we're done we can convert it back and set it.
    let mut fog_color = Srgba::from(fog.color);
    if keycode.pressed(KeyCode::Minus) {
        fog_color.red = (fog_color.red - 0.1 * delta).max(0.0);
    }

    if keycode.any_pressed([KeyCode::Equal, KeyCode::NumpadEqual]) {
        fog_color.red = (fog_color.red + 0.1 * delta).min(1.0);
    }

    if keycode.pressed(KeyCode::BracketLeft) {
        fog_color.green = (fog_color.green - 0.1 * delta).max(0.0);
    }

    if keycode.pressed(KeyCode::BracketRight) {
        fog_color.green = (fog_color.green + 0.1 * delta).min(1.0);
    }

    if keycode.pressed(KeyCode::Semicolon) {
        fog_color.blue = (fog_color.blue - 0.1 * delta).max(0.0);
    }

    if keycode.pressed(KeyCode::Quote) {
        fog_color.blue = (fog_color.blue + 0.1 * delta).min(1.0);
    }

    if keycode.pressed(KeyCode::Period) {
        fog_color.alpha = (fog_color.alpha - 0.1 * delta).max(0.0);
    }

    if keycode.pressed(KeyCode::Slash) {
        fog_color.alpha = (fog_color.alpha + 0.1 * delta).min(1.0);
    }

    fog.color = Color::from(fog_color);
}
