//! Tests `ImageSamplerDescription::mipmap_filter`. Loads a checkerboard model
//! and lets the user switch between various presets. This also serves as a test
//! of `GltfLoaderSettings::override_sampler`.
//!
//! CAUTION: This test currently fails due to <https://github.com/bevyengine/bevy/issues/18267> -
//! subsequent loads of the same gltf will ignore the test's custom sampler settings.

use bevy::{
    gltf::GltfLoaderSettings,
    image::{ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor},
    prelude::*,
};

#[derive(Resource)]
struct Combos(Vec<Combo>);

fn main() {
    let default_sampler = ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..Default::default()
    };

    let combos = Combos(vec![
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
    ]);

    let mut app = App::new();

    app.insert_resource(combos)
        .insert_resource(VisibleCombo(KeyCode::Digit1))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_controls,
                update_visibility,
                update_text,
                update_camera,
            ),
        );

    #[cfg(feature = "bevy_ci_testing")]
    app.insert_resource(ci::Capture::default())
        .add_systems(Startup, ci::setup)
        .add_systems(Update, ci::update);

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, combos: Res<Combos>) {
    for combo in combos.0.iter() {
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

#[derive(Component, Clone)]
struct Combo {
    key: KeyCode,
    label: &'static str,
    sampler: ImageSamplerDescriptor,
}

#[derive(Resource, PartialEq)]
struct VisibleCombo(KeyCode);

fn update_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut time: ResMut<Time<Virtual>>,
    combos: Query<&Combo>,
    mut visible_combo: ResMut<VisibleCombo>,
) {
    for combo in &combos {
        if keyboard_input.just_pressed(combo.key) {
            *visible_combo = VisibleCombo(combo.key);
        }
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    }
}

fn update_visibility(
    mut combos: Query<(&Combo, &mut Visibility)>,
    visible_combo: Res<VisibleCombo>,
) {
    for (combo, mut visibility) in &mut combos {
        *visibility = if *visible_combo == VisibleCombo(combo.key) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_text(
    mut text: Single<&mut Text>,
    time: Res<Time<Virtual>>,
    combos: Query<(&Combo, &Visibility)>,
    visible_combo: Res<VisibleCombo>,
) {
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

fn update_camera(_time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in &mut query {
        #[cfg(feature = "bevy_ci_testing")]
        let height = 0.08;

        #[cfg(not(feature = "bevy_ci_testing"))]
        let height = (ops::sin(_time.elapsed_secs()) * 0.07) + 0.08;

        *transform =
            Transform::from_xyz(0.2, height, 0.95).looking_at(Vec3::new(0.0, -0.1, 0.0), Vec3::Y);
    }
}

#[cfg(feature = "bevy_ci_testing")]
mod ci {
    use super::*;
    use bevy::{
        dev_tools::ci_testing::{CiTestingConfig, CiTestingEvent, CiTestingEventOnFrame},
        diagnostic::FrameCount,
        render::view::screenshot::Captured,
    };

    #[derive(Resource, Default)]
    pub struct Capture {
        pending: bool,
        remaining: Vec<KeyCode>,
    }

    pub fn setup(mut capture: ResMut<Capture>, combos: Res<Combos>) {
        capture.remaining = combos.0.iter().map(|c| c.key).collect();
    }

    pub fn update(
        mut ci_config: ResMut<CiTestingConfig>,
        mut capture: ResMut<Capture>,
        mut visible_combo: ResMut<VisibleCombo>,
        frame_count: Res<FrameCount>,
        removed_captures: RemovedComponents<Captured>,
    ) {
        if capture.pending {
            capture.pending = removed_captures.is_empty();
        } else if let Some(next) = capture.remaining.pop() {
            ci_config.events.push(CiTestingEventOnFrame(
                frame_count.0 + 100,
                CiTestingEvent::NamedScreenshot(format!("{:?}", next)),
            ));
            visible_combo.0 = next;
            capture.pending = true;
        } else {
            ci_config.events.push(CiTestingEventOnFrame(
                frame_count.0 + 1,
                CiTestingEvent::AppExit,
            ));
        }
    }
}
