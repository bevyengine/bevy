//! Demonstrates the normal map, metallic-roughness map, and emissive features
//! of clustered decals.

use std::{f32::consts::PI, time::Duration};

use bevy::{
    asset::io::web::WebAssetPlugin,
    color::palettes::css::{CRIMSON, GOLD},
    image::ImageLoaderSettings,
    light::ClusteredDecal,
    prelude::*,
    render::view::Hdr,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::widgets::{RadioButton, RadioButtonText, WidgetClickEvent, WidgetClickSender};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// The demonstration textures that we use.
///
/// We cache these for efficiency.
#[derive(Resource)]
struct AppTextures {
    /// The base color that all our decals have (the Bevy logo).
    decal_base_color_texture: Handle<Image>,

    /// A normal map that all our decals have.
    ///
    /// This provides a nice raised embossed look.
    decal_normal_map_texture: Handle<Image>,

    /// The metallic-roughness map that all our decals have.
    ///
    /// Metallic is in the blue channel and roughness is in the green channel,
    /// like glTF requires.
    decal_metallic_roughness_map_texture: Handle<Image>,

    /// The emissive texture that can optionally be enabled.
    ///
    /// This causes the white bird to glow.
    decal_emissive_texture: Handle<Image>,
}

impl FromWorld for AppTextures {
    fn from_world(world: &mut World) -> Self {
        // Load all the decal textures.
        let asset_server = world.resource::<AssetServer>();
        AppTextures {
            decal_base_color_texture: asset_server.load("branding/bevy_bird_dark.png"),
            decal_normal_map_texture: asset_server.load_with_settings(
                get_web_asset_url("BevyLogo-Normal.png"),
                |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
            ),
            decal_metallic_roughness_map_texture: asset_server.load_with_settings(
                get_web_asset_url("BevyLogo-MetallicRoughness.png"),
                |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
            ),
            decal_emissive_texture: asset_server.load(get_web_asset_url("BevyLogo-Emissive.png")),
        }
    }
}

/// A component that we place on our decals to track them for animation
/// purposes.
#[derive(Component)]
struct ExampleDecal {
    /// The width and height of the square decal in meters.
    size: f32,
    /// What state the decal is in (animating in, idling, or animating out).
    state: ExampleDecalState,
}

/// The animation state of a decal.
///
/// When each [`Timer`] goes off, the decal advances to the next state.
enum ExampleDecalState {
    /// The decal has just been spawned and is animating in.
    AnimatingIn(Timer),
    /// The decal has animated in and is waiting to animate out.
    Idling(Timer),
    /// The decal is animating out.
    ///
    /// When this timer expires, the decal is despawned.
    AnimatingOut(Timer),
}

/// All settings that the user can change.
///
/// This app only has one: whether newly-spawned decals are emissive.
#[derive(Clone, Copy, PartialEq)]
enum AppSetting {
    /// True if newly-spawned decals have an emissive channel (i.e. they glow),
    /// or false otherwise.
    EmissiveDecals(bool),
}

/// The current values of the settings that the user can change.
///
/// This app only has one: whether newly-spawned decals are emissive.
#[derive(Default, Resource)]
struct AppStatus {
    /// True if newly-spawned decals have an emissive channel (i.e. they glow),
    /// or false otherwise.
    emissive_decals: bool,
}

/// Half of the width and height of the plane onto which the decals are
/// projected.
const PLANE_HALF_SIZE: f32 = 2.0;
/// The minimum width and height that a decal may have.
///
/// The actual size is determined randomly, using this value as a lower bound.
const DECAL_MIN_SIZE: f32 = 0.5;
/// The maximum width and height that a decal may have.
///
/// The actual size is determined randomly, using this value as an upper bound.
const DECAL_MAX_SIZE: f32 = 1.5;

/// How long it takes the decal to grow to its full size when animating in.
const DECAL_ANIMATE_IN_DURATION: Duration = Duration::from_millis(300);
/// How long a decal stays in the idle state before starting to animate out.
const DECAL_IDLE_DURATION: Duration = Duration::from_secs(10);
/// How long it takes the decal to shrink down to nothing when animating out.
const DECAL_ANIMATE_OUT_DURATION: Duration = Duration::from_millis(300);

/// The demo entry point.
fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WebAssetPlugin {
                    silence_startup_warning: true,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Clustered Decal Maps Example".into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_message::<WidgetClickEvent<AppSetting>>()
        .init_resource::<AppStatus>()
        .init_resource::<AppTextures>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos)
        .add_systems(Update, spawn_decal)
        .add_systems(Update, animate_decals)
        .add_systems(
            Update,
            (
                widgets::handle_ui_interactions::<AppSetting>,
                update_radio_buttons,
            ),
        )
        .add_systems(
            Update,
            handle_emission_type_change.after(widgets::handle_ui_interactions::<AppSetting>),
        )
        .insert_resource(SeededRng(ChaCha8Rng::seed_from_u64(19878367467712)))
        .run();
}

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

/// Spawns all the objects in the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_plane_mesh(&mut commands, &asset_server, &mut meshes, &mut materials);
    spawn_light(&mut commands);
    spawn_camera(&mut commands);
    spawn_buttons(&mut commands);
}

/// Spawns the plane onto which the decals are projected.
fn spawn_plane_mesh(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Create a plane onto which we project decals.
    //
    // As the plane has a normal map, we must generate tangents for the
    // vertices.
    let plane_mesh = meshes.add(
        Plane3d {
            normal: Dir3::NEG_Z,
            half_size: Vec2::splat(PLANE_HALF_SIZE),
        }
        .mesh()
        .build()
        .with_extractable_data(|d| {
            d.unwrap()
                .with_duplicated_vertices()
                .with_computed_flat_normals()
                .with_generated_tangents()
                .unwrap()
        }),
    );

    // Give the plane some texture.
    //
    // Note that, as this is a normal map, we must disable sRGB when loading.
    let normal_map_texture = asset_server.load_with_settings(
        "textures/ScratchedGold-Normal.png",
        |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
    );

    // Actually spawn the plane.
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(CRIMSON),
            normal_map_texture: Some(normal_map_texture),
            ..StandardMaterial::default()
        })),
        Transform::IDENTITY,
    ));
}

/// Spawns a light to illuminate the scene.
fn spawn_light(commands: &mut Commands) {
    commands.spawn((
        PointLight {
            intensity: 10_000_000.,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, -8.0),
    ));
}

/// Spawns a camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.0, 0.0, -7.0).looking_at(Vec3::ZERO, Vec3::Y),
        Hdr,
    ));
}

/// Spawns all the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    commands.spawn((
        widgets::main_ui_node(),
        children![widgets::option_buttons(
            "Emissive Decals",
            &[
                (AppSetting::EmissiveDecals(true), "On"),
                (AppSetting::EmissiveDecals(false), "Off"),
            ],
        ),],
    ));
}

/// Draws the outlines that show the bounds of the clustered decals.
fn draw_gizmos(mut gizmos: Gizmos, decals: Query<&GlobalTransform, With<ClusteredDecal>>) {
    for global_transform in &decals {
        gizmos.primitive_3d(
            &Cuboid {
                // Since the clustered decal is a 1×1×1 cube in model space, its
                // half-size is half of the scaling part of its transform.
                half_size: global_transform.scale() * 0.5,
            },
            Isometry3d {
                rotation: global_transform.rotation(),
                translation: global_transform.translation_vec3a(),
            },
            GOLD,
        );
    }
}

/// A system that spawns new decals at fixed intervals.
fn spawn_decal(
    mut commands: Commands,
    app_status: Res<AppStatus>,
    app_textures: Res<AppTextures>,
    time: Res<Time>,
    mut decal_spawn_timer: Local<Option<Timer>>,
    mut seeded_rng: ResMut<SeededRng>,
) {
    // Tick the decal spawn timer. Check to see if we should spawn a new decal,
    // and bail out if it's not yet time to.
    let decal_spawn_timer = decal_spawn_timer
        .get_or_insert_with(|| Timer::new(Duration::from_millis(1000), TimerMode::Repeating));
    decal_spawn_timer.tick(time.delta());
    if !decal_spawn_timer.just_finished() {
        return;
    }

    // Generate a random position along the plane.
    let decal_position = vec3(
        seeded_rng.0.random_range(-PLANE_HALF_SIZE..PLANE_HALF_SIZE),
        seeded_rng.0.random_range(-PLANE_HALF_SIZE..PLANE_HALF_SIZE),
        0.0,
    );

    // Generate a random size for the decal.
    let decal_size = seeded_rng.0.random_range(DECAL_MIN_SIZE..DECAL_MAX_SIZE);

    // Generate a random rotation for the decal.
    let theta = seeded_rng.0.random_range(0.0f32..PI);

    // Now spawn the decal.
    commands.spawn((
        // Apply the textures.
        ClusteredDecal {
            base_color_texture: Some(app_textures.decal_base_color_texture.clone()),
            normal_map_texture: Some(app_textures.decal_normal_map_texture.clone()),
            metallic_roughness_texture: Some(
                app_textures.decal_metallic_roughness_map_texture.clone(),
            ),
            emissive_texture: if app_status.emissive_decals {
                Some(app_textures.decal_emissive_texture.clone())
            } else {
                None
            },
            ..ClusteredDecal::default()
        },
        // Spawn the decal at the right place. Note that the scale is initially
        // zero; we'll animate it later.
        Transform::from_translation(decal_position)
            .with_scale(Vec3::ZERO)
            .looking_to(Vec3::Z, Vec3::ZERO.with_xy(Vec2::from_angle(theta))),
        // Create the component that tracks the animation state.
        ExampleDecal {
            size: decal_size,
            state: ExampleDecalState::AnimatingIn(Timer::new(
                DECAL_ANIMATE_IN_DURATION,
                TimerMode::Once,
            )),
        },
    ));
}

/// A system that animates the decals growing as they enter and shrinking as
/// they leave.
fn animate_decals(
    mut commands: Commands,
    mut decals_query: Query<(Entity, &mut ExampleDecal, &mut Transform)>,
    time: Res<Time>,
) {
    for (decal_entity, mut example_decal, mut decal_transform) in decals_query.iter_mut() {
        // Update the animation timers, and advance the animation state if the
        // timer has expired.
        match example_decal.state {
            ExampleDecalState::AnimatingIn(ref mut timer) => {
                timer.tick(time.delta());
                if timer.just_finished() {
                    example_decal.state =
                        ExampleDecalState::Idling(Timer::new(DECAL_IDLE_DURATION, TimerMode::Once));
                }
            }
            ExampleDecalState::Idling(ref mut timer) => {
                timer.tick(time.delta());
                if timer.just_finished() {
                    example_decal.state = ExampleDecalState::AnimatingOut(Timer::new(
                        DECAL_ANIMATE_OUT_DURATION,
                        TimerMode::Once,
                    ));
                }
            }
            ExampleDecalState::AnimatingOut(ref mut timer) => {
                timer.tick(time.delta());
                if timer.just_finished() {
                    commands.entity(decal_entity).despawn();
                    continue;
                }
            }
        }

        // Actually animate the decal by adjusting its transform.
        // All we have to do here is to compute the decal's scale as a fraction
        // of its full size.
        let new_decal_scale_factor = match example_decal.state {
            ExampleDecalState::AnimatingIn(ref timer) => timer.fraction(),
            ExampleDecalState::Idling(_) => 1.0,
            ExampleDecalState::AnimatingOut(ref timer) => timer.fraction_remaining(),
        };
        decal_transform.scale =
            Vec3::splat(example_decal.size * new_decal_scale_factor).with_z(1.0);
    }
}

/// Updates the appearance of the radio buttons to reflect the current
/// application status.
fn update_radio_buttons(
    mut widgets: Query<
        (
            Entity,
            Option<&mut BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<AppSetting>,
        ),
        Or<(With<RadioButton>, With<RadioButtonText>)>,
    >,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
) {
    for (entity, image, has_text, sender) in widgets.iter_mut() {
        // We only have one setting in this particular application.
        let selected = match **sender {
            AppSetting::EmissiveDecals(emissive_decals) => {
                emissive_decals == app_status.emissive_decals
            }
        };

        if let Some(mut bg_color) = image {
            // Update the colors of the button itself.
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            // Update the colors of the button text.
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

/// Handles the user's clicks on the radio button that determines whether the
/// newly-spawned decals have an emissive map.
fn handle_emission_type_change(
    mut app_status: ResMut<AppStatus>,
    mut events: MessageReader<WidgetClickEvent<AppSetting>>,
) {
    for event in events.read() {
        let AppSetting::EmissiveDecals(on) = **event;
        app_status.emissive_decals = on;
    }
}

/// Returns the GitHub download URL for the given asset.
///
/// The files are expected to be in the `clustered_decal_maps` directory in the
/// [repository].
///
/// [repository]: https://github.com/bevyengine/bevy_asset_files
fn get_web_asset_url(name: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/bevyengine/bevy_asset_files/refs/heads/main/\
clustered_decal_maps/{}",
        name
    )
}
