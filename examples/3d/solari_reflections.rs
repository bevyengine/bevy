//! Test bed for Solari reflections.

use bevy::{
    camera::{CameraMainTextureUsages, Exposure},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::{Diagnostic, DiagnosticPath, DiagnosticsStore},
    image::{ImageAddressMode, ImageLoaderSettings},
    math::ops,
    mesh::VertexAttributeValues,
    prelude::*,
    render::{diagnostic::RenderDiagnosticsPlugin, render_resource::TextureUsages},
    solari::{
        pathtracer::{Pathtracer, PathtracingPlugin},
        prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
    },
    ui_widgets::{
        observe, slider_self_update, Slider, SliderRange, SliderThumb, SliderValue, TrackClick,
    },
};

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy::{
    anti_alias::dlss::{
        Dlss, DlssProjectId, DlssRayReconstructionFeature, DlssRayReconstructionSupported,
    },
    render::camera::{MipBias, TemporalJitter},
};

fn main() {
    let mut app = App::new();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.insert_resource(DlssProjectId(bevy_asset::uuid::uuid!(
        "3f6c1d28-9b04-4a71-bd52-7e8a5c0f1d93"
    )));

    app.add_plugins((
        DefaultPlugins,
        SolariPlugins,
        PathtracingPlugin,
        FreeCameraPlugin,
        RenderDiagnosticsPlugin,
    ))
    .init_resource::<DemoState>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            pause_scene,
            toggle_metallic,
            toggle_pathtracer,
            select_preset,
            update_roughness,
            move_objects,
            reset_pathtracer.after(move_objects),
        ),
    )
    .add_systems(PostUpdate, (update_control_text, update_performance_text));

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.add_systems(Update, toggle_dlss_rr);

    app.run();
}

const METALLIC_BASE_COLOR: Color = Color::srgb(0.95, 0.95, 0.97);
const DIELECTRIC_BASE_COLOR: Color = Color::srgb(0.15, 0.28, 0.55);

const DELTA_ROUGHNESS: f32 = 0.0314;

const GUIDE_ROUGHNESS: f32 = 0.25;

const MATERIAL_PRESETS: [(KeyCode, &str, f32); 5] = [
    (KeyCode::Digit3, "Mirror", 0.0),
    (KeyCode::Digit4, "Near-mirror", DELTA_ROUGHNESS),
    (KeyCode::Digit5, "Glossy", 0.12),
    (KeyCode::Digit6, "Satin", GUIDE_ROUGHNESS - 0.01),
    (KeyCode::Digit7, "Diffuse", 1.0),
];

const INITIAL_ROUGHNESS: f32 = MATERIAL_PRESETS[0].2;

#[derive(Resource)]
struct DemoState {
    metallic: bool,
    roughness: f32,
    paused: bool,
    phase: f32,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            metallic: true,
            roughness: INITIAL_ROUGHNESS,
            paused: false,
            phase: 0.0,
        }
    }
}

#[derive(Resource)]
struct TestMaterial(Handle<StandardMaterial>);

#[derive(Component)]
struct RoughnessSlider;

#[derive(Component)]
struct RoughnessSliderThumb;

#[derive(Component)]
struct OrbitingObject;

#[derive(Component)]
struct SlidingObject;

#[derive(Component)]
struct ControlText;

#[derive(Component)]
struct PerformanceText;

fn raytraced(mesh: impl Into<Mesh>) -> Mesh {
    mesh.into().with_generated_tangents().unwrap()
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_rr_supported: Option<
        Res<DlssRayReconstructionSupported>,
    >,
) {
    let test_material = materials.add(StandardMaterial {
        base_color: METALLIC_BASE_COLOR,
        metallic: 1.0,
        perceptual_roughness: INITIAL_ROUGHNESS,
        ..default()
    });
    commands.insert_resource(TestMaterial(test_material.clone()));

    let mirror = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.97),
        metallic: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });

    spawn_room(&mut commands, &asset_server, &mut meshes, &mut materials);

    let panel = meshes.add(raytraced(Plane3d::new(Vec3::Z, Vec2::new(2.0, 1.5))));
    commands.spawn((
        RaytracingMesh3d(panel.clone()),
        Mesh3d(panel),
        MeshMaterial3d(test_material.clone()),
        Transform::from_xyz(2.4, 1.7, -3.0)
            .with_rotation(Quat::from_rotation_y(0.22) * Quat::from_rotation_x(-0.16)),
    ));

    let sphere = meshes.add(raytraced(Sphere::new(1.2).mesh().build()));
    commands.spawn((
        RaytracingMesh3d(sphere.clone()),
        Mesh3d(sphere),
        MeshMaterial3d(test_material.clone()),
        Transform::from_xyz(4.8, 1.2, -1.6),
    ));

    let cube = meshes.add(raytraced(Cuboid::from_length(0.6)));
    commands.spawn((
        RaytracingMesh3d(cube.clone()),
        Mesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.15, 0.05),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_xyz(3.5, 1.5, 2.5),
        OrbitingObject,
    ));

    let moving_sphere = meshes.add(raytraced(Sphere::new(0.8).mesh().build()));
    commands.spawn((
        RaytracingMesh3d(moving_sphere.clone()),
        Mesh3d(moving_sphere),
        MeshMaterial3d(test_material),
        Transform::from_xyz(0.0, 0.9, 3.6),
        SlidingObject,
    ));

    let nested = meshes.add(raytraced(Plane3d::new(Vec3::Z, Vec2::new(1.3, 1.9))));
    for (x, yaw) in [(-5.2, 1.04), (-1.8, -1.04)] {
        commands.spawn((
            RaytracingMesh3d(nested.clone()),
            Mesh3d(nested.clone()),
            MeshMaterial3d(mirror.clone()),
            Transform::from_xyz(x, 2.0, -3.5).with_rotation(Quat::from_rotation_y(yaw)),
        ));
    }

    commands.spawn((
        RaytracingMesh3d(cube.clone()),
        Mesh3d(cube),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.75, 0.1),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_xyz(-3.5, 2.0, -4.0).with_scale(Vec3::splat(0.6)),
    ));

    let mut camera = commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        FreeCamera {
            walk_speed: 3.0,
            run_speed: 10.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.8, 9.5).looking_at(Vec3::new(0.0, 1.6, -1.0), Vec3::Y),
        Exposure::INDOOR,
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        Msaa::Off,
        SolariLighting::default(),
    ));

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_rr_supported.is_some() {
        camera.insert(Dlss::<DlssRayReconstructionFeature> {
            perf_quality_mode: Default::default(),
            reset: Default::default(),
            _phantom_data: Default::default(),
        });
    }
    let _ = &mut camera;

    spawn_ui(&mut commands);
}

fn spawn_room(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mut floor_mesh = raytraced(Plane3d::new(Vec3::Y, Vec2::new(9.0, 10.5)));
    match floor_mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap() {
        VertexAttributeValues::Float32x2(items) => {
            items.iter_mut().flatten().for_each(|x| *x *= 9.0);
        }
        _ => unreachable!(),
    }
    let floor = meshes.add(floor_mesh);
    commands.spawn((
        RaytracingMesh3d(floor.clone()),
        Mesh3d(floor),
        MeshMaterial3d(
            materials.add(StandardMaterial {
                base_color_texture: Some(
                    asset_server
                        .load_builder()
                        .with_settings::<ImageLoaderSettings>(|settings| {
                            settings
                                .sampler
                                .get_or_init_descriptor()
                                .set_address_mode(ImageAddressMode::Repeat);
                        })
                        .load("textures/uv_checker_bw.png"),
                ),
                perceptual_roughness: 0.9,
                ..default()
            }),
        ),
        Transform::from_xyz(0.0, 0.0, 1.5),
    ));

    let neutral = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.54, 0.52),
        perceptual_roughness: 0.9,
        ..default()
    });

    for (z, normal) in [(-9.0, Vec3::Z), (12.0, Vec3::NEG_Z)] {
        let wall = meshes.add(raytraced(Plane3d::new(normal, Vec2::new(9.0, 4.0))));
        commands.spawn((
            RaytracingMesh3d(wall.clone()),
            Mesh3d(wall),
            MeshMaterial3d(neutral.clone()),
            Transform::from_xyz(0.0, 4.0, z),
        ));
    }

    for (x, normal, tint) in [
        (-9.0, Vec3::X, Color::srgb(0.6, 0.25, 0.22)),
        (9.0, Vec3::NEG_X, Color::srgb(0.22, 0.3, 0.6)),
    ] {
        let wall = meshes.add(raytraced(Plane3d::new(normal, Vec2::new(4.0, 10.5))));
        commands.spawn((
            RaytracingMesh3d(wall.clone()),
            Mesh3d(wall),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: tint,
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(x, 4.0, 1.5),
        ));
    }

    let ceiling = meshes.add(raytraced(Plane3d::new(Vec3::NEG_Y, Vec2::splat(10.5))));
    commands.spawn((
        RaytracingMesh3d(ceiling.clone()),
        Mesh3d(ceiling),
        MeshMaterial3d(neutral),
        Transform::from_xyz(0.0, 8.0, 1.5),
    ));

    let lamp = meshes.add(raytraced(Plane3d::new(Vec3::NEG_Y, Vec2::splat(1.75))));
    commands.spawn((
        RaytracingMesh3d(lamp.clone()),
        Mesh3d(lamp),
        MeshMaterial3d(materials.add(StandardMaterial {
            emissive: LinearRgba::rgb(40000.0, 38000.0, 34000.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 7.8, 0.5),
    ));
}

const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);
const SLIDER_TICK: Color = Color::srgb(0.85, 0.7, 0.2);

fn spawn_ui(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12.0),
            left: px(12.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: px(8.0),
            ..default()
        },
        children![
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(10.0),
                    ..default()
                },
                children![
                    (RoughnessValueText, Text::default(), TextColor(Color::BLACK)),
                    roughness_slider(),
                ],
            ),
            (ControlText, Text::default(), TextColor(Color::BLACK)),
        ],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(0.0),
            padding: px(4.0).all(),
            border_radius: BorderRadius::bottom_left(px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.8)),
        children![(
            PerformanceText,
            Text::default(),
            TextFont {
                font_size: FontSize::Px(8.0),
                ..default()
            },
        )],
    ));
}

fn roughness_slider() -> impl Bundle {
    (
        Node {
            width: px(180.0),
            height: px(12.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Center,
            ..default()
        },
        RoughnessSlider,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(INITIAL_ROUGHNESS),
        SliderRange::new(0.0, 1.0),
        observe(slider_self_update),
        Children::spawn((
            Spawn((
                Node {
                    height: px(6.0),
                    border_radius: BorderRadius::all(px(3.0)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK),
            )),
            Spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    right: px(12.0),
                    top: px(0.0),
                    bottom: px(0.0),
                    ..default()
                },
                children![
                    threshold_tick(DELTA_ROUGHNESS),
                    threshold_tick(GUIDE_ROUGHNESS),
                    (
                        RoughnessSliderThumb,
                        SliderThumb,
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(12.0),
                            height: px(12.0),
                            left: percent(INITIAL_ROUGHNESS * 100.0),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(SLIDER_THUMB),
                    )
                ],
            )),
        )),
    )
}

#[derive(Component)]
struct RoughnessValueText;

fn threshold_tick(roughness: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: percent(roughness * 100.0),
            width: px(12.0),
            height: px(12.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                width: px(2.0),
                height: px(12.0),
                ..default()
            },
            BackgroundColor(SLIDER_TICK),
        )],
    )
}

fn pause_scene(key_input: Res<ButtonInput<KeyCode>>, mut state: ResMut<DemoState>) {
    if key_input.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
    }
}

fn set_metallic(material: &mut StandardMaterial, metallic: bool) {
    material.metallic = if metallic { 1.0 } else { 0.0 };
    material.base_color = if metallic {
        METALLIC_BASE_COLOR
    } else {
        DIELECTRIC_BASE_COLOR
    };
}

fn toggle_metallic(
    key_input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DemoState>,
    test_material: Res<TestMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if key_input.just_pressed(KeyCode::Digit1) {
        state.metallic = !state.metallic;
        set_metallic(
            &mut materials.get_mut(&test_material.0).unwrap(),
            state.metallic,
        );
    }
}

fn toggle_pathtracer(
    key_input: Res<ButtonInput<KeyCode>>,
    camera: Single<(Entity, Has<Pathtracer>), With<Camera3d>>,
    mut commands: Commands,
) {
    if key_input.just_pressed(KeyCode::KeyP) {
        let (entity, pathtracing) = *camera;
        let mut camera = commands.entity(entity);

        if pathtracing {
            camera.remove_with_requires::<Pathtracer>();
            camera.insert(SolariLighting::default());
        } else {
            camera.remove_with_requires::<SolariLighting>();
            camera.insert(Pathtracer::default());

            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            camera.remove::<(Dlss<DlssRayReconstructionFeature>, TemporalJitter, MipBias)>();
        }
    }
}

fn reset_pathtracer(state: Res<DemoState>, pathtracer: Option<Single<&mut Pathtracer>>) {
    if let Some(mut pathtracer) = pathtracer
        && state.is_changed()
    {
        pathtracer.reset = true;
    }
}

fn select_preset(
    key_input: Res<ButtonInput<KeyCode>>,
    slider: Single<Entity, With<RoughnessSlider>>,
    test_material: Res<TestMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for (key, _, roughness) in MATERIAL_PRESETS {
        if key_input.just_pressed(key) {
            materials
                .get_mut(&test_material.0)
                .unwrap()
                .perceptual_roughness = roughness;

            commands.entity(*slider).insert(SliderValue(roughness));
        }
    }
}

fn update_roughness(
    slider: Query<
        (Entity, &SliderValue, &SliderRange),
        (Changed<SliderValue>, With<RoughnessSlider>),
    >,
    children: Query<&Children>,
    mut thumb: Query<&mut Node, With<RoughnessSliderThumb>>,
    mut value_text: Single<&mut Text, With<RoughnessValueText>>,
    mut state: ResMut<DemoState>,
    test_material: Res<TestMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (slider_entity, value, range) in &slider {
        state.roughness = value.0;
        materials
            .get_mut(&test_material.0)
            .unwrap()
            .perceptual_roughness = value.0;

        value_text.0 = format!("Roughness {:.2}", value.0);

        for child in children.iter_descendants(slider_entity) {
            if let Ok(mut node) = thumb.get_mut(child) {
                node.left = percent(range.thumb_position(value.0) * 100.0);
            }
        }
    }
}

const PHASE_RATE: f32 = 0.8;

fn move_objects(
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DemoState>,
    mut orbiting: Single<&mut Transform, With<OrbitingObject>>,
    mut sliding: Single<&mut Transform, (With<SlidingObject>, Without<OrbitingObject>)>,
) {
    let scrub = key_input.pressed(KeyCode::ArrowRight) as i32 as f32
        - key_input.pressed(KeyCode::ArrowLeft) as i32 as f32;
    let playback = if state.paused { 0.0 } else { 1.0 };
    let delta = (playback + scrub) * time.delta_secs() * PHASE_RATE;

    if delta != 0.0 {
        state.phase += delta;
    }

    orbiting.translation = Vec3::new(
        3.5 + ops::sin(state.phase) * 2.8,
        1.5,
        0.5 + ops::cos(state.phase) * 2.6,
    );
    orbiting.rotation = Quat::from_rotation_y(state.phase * 0.7);

    sliding.translation.x = ops::sin(state.phase * 0.7) * 5.5;
}

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
fn toggle_dlss_rr(
    key_input: Res<ButtonInput<KeyCode>>,
    camera: Single<(Entity, Has<Dlss<DlssRayReconstructionFeature>>), With<SolariLighting>>,
    dlss_rr_supported: Option<Res<DlssRayReconstructionSupported>>,
    mut commands: Commands,
) {
    if key_input.just_pressed(KeyCode::Digit2) && dlss_rr_supported.is_some() {
        let (entity, dlss) = *camera;
        if dlss {
            commands
                .entity(entity)
                .remove::<(Dlss<DlssRayReconstructionFeature>, TemporalJitter, MipBias)>();
        } else {
            commands
                .entity(entity)
                .insert(Dlss::<DlssRayReconstructionFeature> {
                    perf_quality_mode: Default::default(),
                    reset: Default::default(),
                    _phantom_data: Default::default(),
                });
        }
    }
}

fn update_control_text(
    mut text: Single<&mut Text, With<ControlText>>,
    state: Res<DemoState>,
    pathtracing: Query<(), With<Pathtracer>>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_rr_supported: Option<
        Res<DlssRayReconstructionSupported>,
    >,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_camera: Query<
        Has<Dlss<DlssRayReconstructionFeature>>,
        With<SolariLighting>,
    >,
) {
    text.0.clear();

    if state.paused {
        text.0.push_str("(Space): Resume");
    } else {
        text.0.push_str("(Space): Pause");
    }

    if state.metallic {
        text.0.push_str("\n(1): Switch to dielectric");
    } else {
        text.0.push_str("\n(1): Switch to metallic");
    }

    text.0.push_str("\n(Left/Right): Scrub objects");

    if pathtracing.is_empty() {
        text.0.push_str("\n(P): Switch to reference pathtracer");
    } else {
        text.0.push_str(
            "\n(P): Switch to realtime lighting  -  pause to let the pathtracer converge",
        );
    }

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_rr_supported.is_some() {
        if matches!(dlss_camera.single(), Ok(true)) {
            text.0.push_str("\n(2): Disable DLSS Ray Reconstruction");
        } else {
            text.0.push_str("\n(2): Enable DLSS Ray Reconstruction");
        }
    } else {
        text.0
            .push_str("\nDenoising: DLSS Ray Reconstruction not supported");
    }

    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    text.0
        .push_str("\nDenoising: App not compiled with DLSS support");

    text.0.push_str("\n(3-7):");
    for (_, name, roughness) in MATERIAL_PRESETS {
        if (roughness - state.roughness).abs() < 1e-4 {
            text.0.push_str(&format!(" [{name}]"));
        } else {
            text.0.push_str(&format!(" {name}"));
        }
    }
}

fn update_performance_text(
    mut text: Single<&mut Text, With<PerformanceText>>,
    diagnostics: Res<DiagnosticsStore>,
    pathtracing: Query<(), With<Pathtracer>>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_camera: Query<
        Has<Dlss<DlssRayReconstructionFeature>>,
        With<SolariLighting>,
    >,
) {
    text.0.clear();

    if !pathtracing.is_empty() {
        text.push_str("Pathtracer (untimed)");
        return;
    }

    let mut total = 0.0;
    let mut add_diagnostic = |name: &str, path: &'static str| {
        let path = DiagnosticPath::new(path);
        if let Some(value) = diagnostics.get(&path).and_then(Diagnostic::smoothed) {
            text.push_str(&format!("{name:17}  {value:.2} ms\n"));
            total += value;
        }
    };

    (add_diagnostic)(
        "Light tiles",
        "render/solari_lighting/presample_light_tiles/elapsed_gpu",
    );
    (add_diagnostic)(
        "World cache",
        "render/solari_lighting/world_cache/elapsed_gpu",
    );
    (add_diagnostic)("Lighting", "render/solari_lighting/lighting/elapsed_gpu");
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if matches!(dlss_camera.single(), Ok(true)) {
        (add_diagnostic)("DLSS-RR", "render/dlss_ray_reconstruction/elapsed_gpu");
    }
    text.push_str(&format!("{:17}  {total:.2} ms\n", "Total"));
}
