//! Tests loading the same glTF multiple times but with different values for
//! `GltfLoaderSettings::override_sampler` and `default_sampler`.
//!
//! CAUTION: This test currently fails due to <https://github.com/bevyengine/bevy/issues/18267> -
//! subsequent loads of the same gltf do not respect the loader settings.

use bevy::{
    gltf::GltfLoaderSettings,
    image::{ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor},
    prelude::*,
};

fn main() {
    App::new()
        .insert_resource(VisibleCombo(KeyCode::Digit1))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_controls, update_camera))
        .run();
}

#[derive(Component, Clone)]
struct Combo {
    key: KeyCode,
    label: &'static str,
    sampler: ImageSamplerDescriptor,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let default_sampler = ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..Default::default()
    };

    // Declare combinations of sampler settings linked to a key code.

    let combos: &[Combo] = &[
        Combo {
            key: KeyCode::Digit1,
            label: "1: None",
            sampler: ImageSamplerDescriptor {
                lod_max_clamp: 0.0,
                ..default_sampler.clone()
            },
        },
        Combo {
            key: KeyCode::Digit2,
            label: "2: Nearest",
            sampler: default_sampler.clone(),
        },
        Combo {
            key: KeyCode::Digit3,
            label: "3: Linear",
            sampler: ImageSamplerDescriptor {
                mipmap_filter: ImageFilterMode::Linear,
                ..default_sampler.clone()
            },
        },
        Combo {
            key: KeyCode::Digit4,
            label: "4: Linear, Anisotropic",
            sampler: ImageSamplerDescriptor {
                mipmap_filter: ImageFilterMode::Linear,
                anisotropy_clamp: 16,
                ..default_sampler.clone()
            },
        },
    ];

    // Spawn each combination.

    for combo in combos.iter() {
        let asset = GltfAssetLabel::Scene(0).from_asset("models/checkerboard/checkerboard.gltf");

        let sampler = combo.sampler.clone();

        let settings = move |settings: &mut GltfLoaderSettings| {
            settings.default_sampler = Some(sampler.clone());
            settings.override_sampler = true;
        };

        commands.spawn((
            SceneRoot(asset_server.load_with_settings(asset, settings)),
            combo.clone(),
        ));
    }

    // Spawn camera and text.

    commands.spawn((
        Camera3d::default(),
        Projection::from(PerspectiveProjection {
            near: 0.001,
            ..Default::default()
        }),
    ));

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..Default::default()
        },
    ));
}

#[derive(Resource, PartialEq)]
struct VisibleCombo(KeyCode);

fn update_controls(
    mut text: Single<&mut Text>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut time: ResMut<Time<Virtual>>,
    mut combos: Query<(&Combo, &mut Visibility)>,
    mut visible_combo: ResMut<VisibleCombo>,
) {
    // Update combo visibility.

    for (combo, _) in &combos {
        if keyboard_input.just_pressed(combo.key) {
            *visible_combo = VisibleCombo(combo.key);
        }
    }

    for (combo, mut visibility) in &mut combos {
        *visibility = if *visible_combo == VisibleCombo(combo.key) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update pause.

    if keyboard_input.just_pressed(KeyCode::Space) {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    }

    // Update help text.

    text.clear();

    text.push_str(&format!(
        "Space: {}\n\n",
        if time.is_paused() { "Unpause" } else { "Pause" }
    ));

    text.push_str("Mipmap filter:\n");

    for (combo, _) in &combos {
        let visible = *visible_combo == VisibleCombo(combo.key);

        text.push_str(&format!(
            "{}{}\n",
            if visible { "> " } else { "  " },
            combo.label,
        ));
    }
}

fn update_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in &mut query {
        let height = (ops::sin(time.elapsed_secs()) * 0.07) + 0.08;

        *transform =
            Transform::from_xyz(0.2, height, 0.95).looking_at(Vec3::new(0.0, -0.1, 0.0), Vec3::Y);
    }
}
