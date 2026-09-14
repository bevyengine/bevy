//! Test bed for Solari reflections.

use bevy::{
    camera::{CameraMainTextureUsages, Exposure},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::{Diagnostic, DiagnosticPath, DiagnosticsStore},
    image::{ImageAddressMode, ImageLoaderSettings},
    math::ops,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
    render::{diagnostic::RenderDiagnosticsPlugin, render_resource::TextureUsages},
    solari::{
        pathtracer::{Pathtracer, PathtracingPlugin},
        prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
    },
    ui_widgets::{
        observe, slider_self_update, Slider, SliderRange, SliderThumb, SliderValue, TrackClick,
    },
    world_serialization::WorldInstanceReady,
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
            toggle_mirror_motion,
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
    pan_mirror: bool,
    slide_mirror: bool,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            metallic: true,
            roughness: INITIAL_ROUGHNESS,
            paused: false,
            phase: 0.0,
            pan_mirror: false,
            slide_mirror: false,
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
struct PanningMirror;

#[derive(Component)]
enum ControlRow {
    Pause,
    Metallic,
    Presets,
    MirrorPan,
    MirrorSlide,
    Renderer,
    Denoising,
}

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
        Transform::from_xyz(MIRROR_BASE_X, 1.7, -3.0).with_rotation(mirror_rotation(0.0)),
        PanningMirror,
    ));

    commands
        .spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
            )),
            Transform::from_xyz(4.8, 0.0, -1.6)
                .with_scale(Vec3::splat(4.0))
                .with_rotation(Quat::from_rotation_y(-0.4)),
        ))
        .observe(add_raytracing_meshes_on_scene_load);

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
        camera.insert(Dlss::<DlssRayReconstructionFeature>::default());
    }
    let _ = &mut camera;

    spawn_ui(&mut commands);
}

fn add_raytracing_meshes_on_scene_load(
    scene_ready: On<WorldInstanceReady>,
    children: Query<&Children>,
    mesh_query: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for descendant in children.iter_descendants(scene_ready.entity) {
        if let Ok(Mesh3d(mesh_handle)) = mesh_query.get(descendant) {
            commands
                .entity(descendant)
                .insert(RaytracingMesh3d(mesh_handle.clone()));

            let mut mesh = meshes.get_mut(mesh_handle).unwrap();
            if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0) {
                let vertex_count = mesh.count_vertices();
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; vertex_count]);
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_TANGENT,
                    vec![[0.0, 0.0, 0.0, 0.0]; vertex_count],
                );
            }
            if !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) {
                mesh.generate_tangents().unwrap();
            }
            if mesh.contains_attribute(Mesh::ATTRIBUTE_UV_1) {
                mesh.remove_attribute(Mesh::ATTRIBUTE_UV_1);
            }
            if let Some(indices) = mesh.indices_mut()
                && let Indices::U16(_) = indices
            {
                *indices = Indices::U32(indices.iter().map(|i| i as u32).collect());
            }
        }
    }
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

const SLIDER_TRACK: Color = Color::srgb(0.45, 0.45, 0.48);
const SLIDER_THUMB: Color = Color::srgb(0.45, 0.95, 0.45);
const SLIDER_TICK: Color = Color::srgb(0.95, 0.78, 0.25);

const SLIDER_WIDTH: f32 = 180.0;
const SLIDER_THUMB_SIZE: f32 = 14.0;
const SLIDER_TRACK_HEIGHT: f32 = 8.0;

const HEADING_COLOR: Color = Color::srgb(0.72, 0.83, 1.0);
const KEY_COLOR: Color = Color::srgb(0.95, 0.82, 0.4);
const LABEL_COLOR: Color = Color::srgb(0.92, 0.92, 0.92);

const HEADING_SIZE: f32 = 10.0;
const LABEL_SIZE: f32 = 11.0;

const KEY_WIDTH: f32 = 42.0;
const KEY_GAP: f32 = 8.0;

fn spawn_ui(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12.0),
            left: px(12.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            padding: px(10.0).all(),
            border_radius: BorderRadius::all(px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.92)),
        children![
            heading("PLAYBACK"),
            row("Space", ControlRow::Pause),
            static_row("< >", "Scrub objects"),
            heading("TEST MATERIAL"),
            row("1", ControlRow::Metallic),
            row("3-7", ControlRow::Presets),
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(KEY_GAP),
                    margin: UiRect::left(px(KEY_WIDTH + KEY_GAP)),
                    ..default()
                },
                children![
                    (
                        RoughnessValueText,
                        Text::default(),
                        TextFont::from_font_size(LABEL_SIZE),
                        TextColor(LABEL_COLOR),
                    ),
                    roughness_slider(),
                ],
            ),
            heading("RIGHT MIRROR"),
            row("8", ControlRow::MirrorPan),
            row("9", ControlRow::MirrorSlide),
            heading("RENDERER"),
            row("P", ControlRow::Renderer),
            row("2", ControlRow::Denoising),
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

fn heading(title: &str) -> impl Bundle {
    (
        Node {
            margin: UiRect::top(px(7.0)).with_bottom(px(3.0)),
            ..default()
        },
        Text::new(title),
        TextFont::from_font_size(HEADING_SIZE),
        TextColor(HEADING_COLOR),
    )
}

fn row(key: &str, label: ControlRow) -> impl Bundle {
    (
        row_node(),
        children![
            key_label(key),
            (
                label,
                Text::default(),
                TextFont::from_font_size(LABEL_SIZE),
                TextColor(LABEL_COLOR),
            ),
        ],
    )
}

fn static_row(key: &str, label: &str) -> impl Bundle {
    (
        row_node(),
        children![
            key_label(key),
            (
                Text::new(label),
                TextFont::from_font_size(LABEL_SIZE),
                TextColor(LABEL_COLOR),
            ),
        ],
    )
}

fn row_node() -> Node {
    Node {
        align_items: AlignItems::Center,
        column_gap: px(KEY_GAP),
        ..default()
    }
}

fn key_label(key: &str) -> impl Bundle {
    (
        Node {
            width: px(KEY_WIDTH),
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },
        children![(
            Text::new(key),
            TextFont::from_font_size(LABEL_SIZE),
            TextColor(KEY_COLOR),
        )],
    )
}

fn roughness_slider() -> impl Bundle {
    (
        Node {
            width: px(SLIDER_WIDTH),
            height: px(SLIDER_THUMB_SIZE),
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
                    height: px(SLIDER_TRACK_HEIGHT),
                    border_radius: BorderRadius::all(px(SLIDER_TRACK_HEIGHT / 2.0)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK),
            )),
            Spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    right: px(SLIDER_THUMB_SIZE),
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
                            width: px(SLIDER_THUMB_SIZE),
                            height: px(SLIDER_THUMB_SIZE),
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
            width: px(SLIDER_THUMB_SIZE),
            height: px(SLIDER_THUMB_SIZE),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                width: px(2.0),
                height: px(SLIDER_THUMB_SIZE),
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

fn toggle_mirror_motion(key_input: Res<ButtonInput<KeyCode>>, mut state: ResMut<DemoState>) {
    if key_input.just_pressed(KeyCode::Digit8) {
        state.pan_mirror = !state.pan_mirror;
    }
    if key_input.just_pressed(KeyCode::Digit9) {
        state.slide_mirror = !state.slide_mirror;
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

const MIRROR_YAW: f32 = 0.22;
const MIRROR_PITCH: f32 = -0.16;
const MIRROR_PAN_RANGE: f32 = 0.55;
const MIRROR_BASE_X: f32 = 2.4;
const MIRROR_SLIDE_RANGE: f32 = 1.5;

fn mirror_rotation(pan: f32) -> Quat {
    Quat::from_rotation_y(MIRROR_YAW + pan) * Quat::from_rotation_x(MIRROR_PITCH)
}

fn move_objects(
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<DemoState>,
    mut orbiting: Single<&mut Transform, With<OrbitingObject>>,
    mut sliding: Single<&mut Transform, (With<SlidingObject>, Without<OrbitingObject>)>,
    mut mirror: Single<
        &mut Transform,
        (
            With<PanningMirror>,
            Without<OrbitingObject>,
            Without<SlidingObject>,
        ),
    >,
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

    let pan = if state.pan_mirror {
        ops::sin(state.phase * 1.1) * MIRROR_PAN_RANGE
    } else {
        0.0
    };
    mirror.rotation = mirror_rotation(pan);

    mirror.translation.x = if state.slide_mirror {
        MIRROR_BASE_X + ops::sin(state.phase * 0.9) * MIRROR_SLIDE_RANGE
    } else {
        MIRROR_BASE_X
    };
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
                .insert(Dlss::<DlssRayReconstructionFeature>::default());
        }
    }
}

fn update_control_text(
    mut rows: Query<(&ControlRow, &mut Text)>,
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
    for (row, mut text) in &mut rows {
        text.0.clear();

        match row {
            ControlRow::Pause => text
                .0
                .push_str(if state.paused { "Resume" } else { "Pause" }),
            ControlRow::Metallic => text.0.push_str(if state.metallic {
                "Switch to dielectric"
            } else {
                "Switch to metallic"
            }),
            ControlRow::Presets => {
                for (_, name, roughness) in MATERIAL_PRESETS {
                    if (roughness - state.roughness).abs() < 1e-4 {
                        text.0.push_str(&format!("[{name}] "));
                    } else {
                        text.0.push_str(&format!("{name} "));
                    }
                }
            }
            ControlRow::MirrorPan => text.0.push_str(if state.pan_mirror {
                "Stop panning"
            } else {
                "Pan back and forth"
            }),
            ControlRow::MirrorSlide => text.0.push_str(if state.slide_mirror {
                "Stop sliding"
            } else {
                "Slide back and forth"
            }),
            ControlRow::Renderer => text.0.push_str(if pathtracing.is_empty() {
                "Switch to reference pathtracer"
            } else {
                "Switch to realtime lighting  -  pause to let the pathtracer converge"
            }),
            ControlRow::Denoising => {
                #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
                if dlss_rr_supported.is_some() {
                    if matches!(dlss_camera.single(), Ok(true)) {
                        text.0.push_str("Disable DLSS Ray Reconstruction");
                    } else {
                        text.0.push_str("Enable DLSS Ray Reconstruction");
                    }
                } else {
                    text.0.push_str("DLSS Ray Reconstruction not supported");
                }

                #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
                text.0.push_str("App not compiled with DLSS support");
            }
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
