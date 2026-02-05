//! A series of simple 3D scenes showing how alpha blending can break and how order independent transparency (OIT) can fix it.
//!
//! See [`OrderIndependentTransparencyPlugin`] for the trade-offs of using OIT.
//!
//! [`OrderIndependentTransparencyPlugin`]: bevy::core_pipeline::oit::OrderIndependentTransparencyPlugin

use crate::widgets::{RadioButton, RadioButtonText, WidgetClickEvent, WidgetClickSender};
use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::css::{BLUE, GREEN, RED, YELLOW},
    core_pipeline::{oit::OrderIndependentTransparencySettings, prepass::DepthPrepass},
    pbr::{ExtendedMaterial, MaterialExtension},
    picking::window::update_window_hits,
    prelude::*,
    shader::ShaderRef,
    window::{CursorIcon, PresentMode, PrimaryWindow, SystemCursorIcon},
};
use bevy_ecs::system::SystemParam;
use bevy_render::render_resource::AsBindGroup;

#[path = "../helpers/widgets.rs"]
mod widgets;

/// Scene construction functions
const SCENES: &[(&str, &str, fn(&mut Commands, &mut SceneResources))] = &[
    ("1", "Three balls", spawn_spheres),
    ("2", "Stacked quads", spawn_quads),
    ("3", "Opaque occlusion test", spawn_occlusion_test),
    ("4", "Auto instancing test", spawn_auto_instancing_test),
    ("5", "Custom material demo", spawn_custom_material),
];

/// Application state
#[derive(Resource)]
struct AppState {
    /// Current OIT settings
    oit_settings: OrderIndependentTransparencySettings,
    /// Whether to use OIT or standard mesh sorting
    use_oit: bool,
    /// Using a depth prepass helps cull transparent fragment against opaque ones earlier
    use_depth_prepass: bool,
    /// Disable VSync to better assess performance
    enable_vsync: bool,
    /// The current scene being displayed
    current_scene_id: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            oit_settings: Default::default(),
            use_oit: true,
            use_depth_prepass: true,
            enable_vsync: true,
            current_scene_id: 0,
        }
    }
}

/// Tweakable settings
#[derive(Clone)]
enum AppSetting {
    /// Change whether OIT is used or not
    EnableOIT(bool),
    /// Change whether DepthPrepass is used or not
    UseDepthPrepass(bool),
    /// Enable or disable VSync on the window
    EnableVsync(bool),
    /// Change the displayed scene
    ChangeScene(usize),
}

/// This struct bundles up the resources used by the scene creation functions.
/// Derives SystemParam to be able to pass it in systems.
#[derive(SystemParam)]
struct SceneResources<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    extended_materials:
        ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, CheckeredMaterialExtension>>>,
    custom_materials: ResMut<'w, Assets<NoisyOpacityMaterial>>,
    asset_server: Res<'w, AssetServer>,
}

/// This message is similar to WidgetClickEvent<AppSetting>, only for events generated
/// by the app.
#[derive(Clone, Message, Deref, DerefMut)]
struct AppEvent(AppSetting);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: if AppState::default().enable_vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<NoisyOpacityMaterial>::default())
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, CheckeredMaterialExtension>,
        >::default())
        .init_resource::<AppState>()
        .add_message::<WidgetClickEvent<AppSetting>>()
        .add_message::<AppEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, handle_keyboard_shortcuts)
        .add_systems(Update, scene_change_watcher)
        .add_systems(Update, update_window_hits)
        .add_systems(
            Update,
            (
                widgets::handle_ui_interactions::<AppSetting>,
                update_radio_buttons.after(widgets::handle_ui_interactions::<AppSetting>),
                handle_setting_change.after(widgets::handle_ui_interactions::<AppSetting>),
            ),
        )
        .run();
}

/// Component used to memorize the camera transform when initiating a Drag event
#[derive(Component)]
struct BaseTransform(Transform);

/// Sets up the base scene
fn setup(
    mut commands: Commands,
    mut resources: SceneResources,
    app_state: Res<AppState>,
    window: Single<Entity, With<PrimaryWindow>>,
) {
    // Spawn the main UI
    spawn_ui(&mut commands);

    // Drag events handling on the window, tied to the camera rotation
    commands
        .entity(*window)
        .observe(
            |event: On<Pointer<Drag>>,
             mut commands: Commands,
             mut camera_transforms: Single<
                (&mut Transform, &BaseTransform),
                With<Camera3d>,
            >| {
                commands
                    .entity(event.entity)
                    .insert(CursorIcon::System(SystemCursorIcon::Grabbing));

                // During drag the additional rotation is relative to the transform stored when Drag was initiated
                let (ref mut transform, base_transform) = *camera_transforms;
                **transform = base_transform.0;

                const RADIANS_PER_PIXEL: f32 = -std::f32::consts::PI / 600.0;
                let angle = event.distance.x * RADIANS_PER_PIXEL;
                transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(angle));
            },
        )
        .observe(
            |_: On<Pointer<DragStart>>,
             mut commands: Commands,
             camera: Single<(Entity, &Transform), With<Camera3d>>| {
                let (camera, transform) = *camera;
                // Memorize the current transform 
                commands
                    .entity(camera)
                    .insert(BaseTransform(*transform));
            },
        )
        .observe(
            |_: On<Pointer<DragEnd>>,
             mut commands: Commands,
             window: Single<Entity, With<PrimaryWindow>>| {
                commands
                    .entity(*window)
                    .insert(CursorIcon::System(SystemCursorIcon::Default));
            },
        );

    // Camera configuration
    let mut camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(1),
        // Msaa currently doesn't work with OIT
        Msaa::Off,
    ));

    if app_state.use_oit {
        // Add this component to the camera to render transparent meshes using OIT
        camera.insert(OrderIndependentTransparencySettings {
            ..app_state.oit_settings
        });
    }

    if app_state.use_depth_prepass {
        // Optional: depth prepass can help OIT filter out fragments occluded by opaque objects
        camera.insert(DepthPrepass);
    }

    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        RenderLayers::layer(1),
    ));

    // Spawn the default scene
    SCENES[0].2(&mut commands, &mut resources); //&mut meshes, &mut materials);
}

/// Watches for key presses and queues corresponding AppEvent's
fn handle_keyboard_shortcuts(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_state: ResMut<AppState>,
    mut messages: MessageWriter<AppEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Tab) {
        let n = app_state.current_scene_id + SCENES.len();
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            app_state.current_scene_id = (n - 1) % SCENES.len();
        } else {
            app_state.current_scene_id = (n + 1) % SCENES.len();
        }
        // There is a dedicated scene change watcher, so no need to push an AppEvent
    }

    if keyboard_input.just_pressed(KeyCode::KeyT) {
        messages.write(AppEvent(AppSetting::EnableOIT(!app_state.use_oit)));
    }

    if keyboard_input.just_pressed(KeyCode::KeyD) {
        messages.write(AppEvent(AppSetting::UseDepthPrepass(
            !app_state.use_depth_prepass,
        )));
    }

    if keyboard_input.just_pressed(KeyCode::KeyV) {
        messages.write(AppEvent(AppSetting::EnableVsync(!app_state.enable_vsync)));
    }
}

fn spawn_ui(commands: &mut Commands) {
    // Invite to interact
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            left: px(0.0),
            right: px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Text::new("Drag view to spin around"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
        )],
    ));

    // Buttons
    commands
        .spawn((
            widgets::main_ui_node(),
            children![
                widgets::option_buttons::<AppSetting>(
                    "Scene",
                    &(SCENES
                        .iter()
                        .enumerate()
                        .map(|(i, scene)| (AppSetting::ChangeScene(i), scene.0))
                        .collect::<Vec<_>>())
                ),
                widgets::option_buttons(
                    "Order Independent [T]ransparency",
                    &[
                        (AppSetting::EnableOIT(true), "On"),
                        (AppSetting::EnableOIT(false), "Off")
                    ]
                ),
                widgets::option_buttons(
                    "[D]epth Prepass",
                    &[
                        (AppSetting::UseDepthPrepass(true), "On"),
                        (AppSetting::UseDepthPrepass(false), "Off")
                    ]
                ),
                widgets::option_buttons(
                    "Enable [V]Sync",
                    &[
                        (AppSetting::EnableVsync(true), "On"),
                        (AppSetting::EnableVsync(false), "Off")
                    ]
                ),
            ],
        ))
        // Prevent the event from bubble up so that view drag does not initiate when interacting with the UI
        .observe(|mut event: On<Pointer<Drag>>| {
            event.propagate(false);
        });
}

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
    app_state: Res<AppState>,
    mut writer: TextUiWriter,
) {
    for (entity, background_color, has_text, sender) in widgets.iter_mut() {
        let selected = match **sender {
            AppSetting::EnableOIT(value) => value == app_state.use_oit,
            AppSetting::UseDepthPrepass(value) => value == app_state.use_depth_prepass,
            AppSetting::EnableVsync(value) => value == app_state.enable_vsync,
            AppSetting::ChangeScene(scene_id) => scene_id == app_state.current_scene_id,
        };

        if let Some(mut background_color) = background_color {
            widgets::update_ui_radio_button(&mut background_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

/// Runs through the messages (either WidgetClickEvent or AppEvent), updates the AppState,
/// and performs the actual required changes
fn handle_setting_change(
    mut commands: Commands,
    mut click_events: MessageReader<WidgetClickEvent<AppSetting>>,
    mut app_events: MessageReader<AppEvent>,
    mut app_state: ResMut<AppState>,
    camera: Single<(Entity, Has<OrderIndependentTransparencySettings>), With<Camera3d>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    // Chain the two message iterators to handle both WidgetCliockEvent and AppEvent
    // This works because both can be derefed to AppSetting
    for event in click_events
        .read()
        .map(|e| &**e)
        .chain(app_events.read().map(|e| &**e))
    {
        match *event {
            AppSetting::EnableOIT(value) => {
                app_state.use_oit = value;
                if app_state.use_oit {
                    commands
                        .entity(camera.0)
                        .insert(app_state.oit_settings.clone());
                } else {
                    commands
                        .entity(camera.0)
                        .remove::<OrderIndependentTransparencySettings>();
                }
            }
            AppSetting::UseDepthPrepass(value) => {
                app_state.use_depth_prepass = value;

                if app_state.use_depth_prepass {
                    commands.entity(camera.0).insert(DepthPrepass);
                } else {
                    commands.entity(camera.0).remove::<DepthPrepass>();
                }
            }
            AppSetting::EnableVsync(value) => {
                app_state.enable_vsync = value;

                window.present_mode = if app_state.enable_vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                };
            }
            AppSetting::ChangeScene(id) => {
                if id != app_state.current_scene_id {
                    app_state.current_scene_id = id;
                }
                // The actual scene change is handled by scene_change_watcher()
            }
        }
    }
}

/// Watches changes on the AppState and loads the appropriate scene
fn scene_change_watcher(
    app_state: Res<AppState>,
    mut prev_scene_id: Local<usize>,
    entities: Query<Entity, With<Mesh3d>>,
    mut resources: SceneResources<'_>,
    mut commands: Commands,
) {
    if app_state.is_changed() && *prev_scene_id != app_state.current_scene_id {
        // Despawn the current scene
        for e in &entities {
            commands.entity(e).despawn();
        }
        SCENES[app_state.current_scene_id].2(&mut commands, &mut resources);
        *prev_scene_id = app_state.current_scene_id;
    }
}

/// Spawns 3 overlapping spheres
/// Technically, when using `alpha_to_coverage` with MSAA this particular example wouldn't break,
/// but it breaks when disabling MSAA and is enough to show the difference between OIT enabled vs disabled.
fn spawn_spheres(commands: &mut Commands, resources: &mut SceneResources) {
    let meshes = &mut resources.meshes;
    let materials = &mut resources.materials;

    let pos_a = Vec3::new(-1.0, 0.75, 0.0);
    let pos_b = Vec3::new(0.0, -0.75, 0.0);
    let pos_c = Vec3::new(1.0, 0.75, 0.0);

    let offset = Vec3::new(0.0, 0.0, 0.0);

    let sphere_handle = meshes.add(Sphere::new(2.0).mesh());

    let alpha = 0.25;

    let render_layers = RenderLayers::layer(1);

    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(alpha).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(pos_a + offset),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN.with_alpha(alpha).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(pos_b + offset),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(alpha).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(pos_c + offset),
        render_layers.clone(),
    ));
}

/// Spawns a stack of transparent quads, which better illustrates the shortcomings
/// of the standard mesh sorted transparency.
fn spawn_quads(commands: &mut Commands, resources: &mut SceneResources) {
    let meshes = &mut resources.meshes;
    let materials = &mut resources.materials;

    let quad_handle = meshes.add(Rectangle::new(3.0, 3.0).mesh());
    let render_layers = RenderLayers::layer(1);
    let xform = |x, y, z| {
        Transform::from_rotation(Quat::from_rotation_y(0.5))
            .mul_transform(Transform::from_xyz(x, y, z))
    };

    // Make the quads double-sided
    let common_params = StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    };

    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            ..common_params.clone()
        })),
        xform(1.0, -0.1, 0.),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(0.8).into(),
            ..common_params.clone()
        })),
        xform(0.5, 0.2, -0.5),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GREEN.with_green(1.0).with_alpha(0.5).into(),
            ..common_params.clone()
        })),
        xform(0.0, 0.4, -1.),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: YELLOW.with_alpha(0.3).into(),
            ..common_params.clone()
        })),
        xform(-0.5, 0.6, -1.1),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(quad_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BLUE.with_alpha(0.2).into(),
            ..common_params
        })),
        xform(-0.8, 0.8, -1.2),
        render_layers.clone(),
    ));
}

/// Spawns a combination of opaque cubes and transparent spheres.
/// This is useful to make sure transparent meshes drawn with OIT
/// are properly occluded by opaque meshes.
fn spawn_occlusion_test(commands: &mut Commands, resources: &mut SceneResources) {
    let meshes = &mut resources.meshes;
    let materials = &mut resources.materials;

    let sphere_handle = meshes.add(Sphere::new(1.0).mesh());
    let cube_handle = meshes.add(Cuboid::from_size(Vec3::ONE).mesh());
    let cube_material = materials.add(Color::srgb(0.8, 0.7, 0.6));

    let render_layers = RenderLayers::layer(1);

    // front
    let x = -2.5;
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material.clone()),
        Transform::from_xyz(x, 0.0, 2.0),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(x, 0., 0.),
        render_layers.clone(),
    ));

    // intersection
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material.clone()),
        Transform::from_xyz(x, 0.0, 1.0),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0., 0., 0.),
        render_layers.clone(),
    ));

    // back
    let x = 2.5;
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(cube_material.clone()),
        Transform::from_xyz(x, 0.0, -2.0),
        render_layers.clone(),
    ));
    commands.spawn((
        Mesh3d(sphere_handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED.with_alpha(0.5).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(x, 0., 0.),
        render_layers.clone(),
    ));
}

/// Spawns multiple entities with the same Mesh+Material. They should automatically be drawn using
/// instancing (when GPU preprocessing is not active) or MultiDrawIndirect (when it is).
fn spawn_auto_instancing_test(commands: &mut Commands, resources: &mut SceneResources) {
    let meshes = &mut resources.meshes;
    let materials = &mut resources.materials;
    let asset_server = &mut resources.asset_server;

    let render_layers = RenderLayers::layer(1);

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material_handle = materials.add(StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        base_color_texture: Some(asset_server.load("textures/slice_square.png")),
        ..Default::default()
    });
    let mut bundles = Vec::with_capacity(3 * 3 * 3);

    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                bundles.push((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material_handle.clone()),
                    Transform::from_xyz(x as f32 * 2.0, y as f32 * 2.0, z as f32 * 2.0),
                    render_layers.clone(),
                ));
            }
        }
    }
    commands.spawn_batch(bundles);
}

const EXTENDED_MATERIAL_SHADER_ASSET_PATH: &str = "shaders/oit_compatible_extended_material.wgsl";

/// Material extension that defines the extra data that will be passed to your shader
/// Used as ExtendedMaterial<StandardMaterial, CheckeredMaterialExtension>
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
struct CheckeredMaterialExtension {
    #[uniform(100)]
    color_1: LinearRgba,
    #[uniform(100)]
    color_2: LinearRgba,
}

impl MaterialExtension for CheckeredMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        EXTENDED_MATERIAL_SHADER_ASSET_PATH.into()
    }

    fn alpha_mode() -> Option<AlphaMode> {
        Some(AlphaMode::Blend)
    }
}

const CUSTOM_MATERIAL_SHADER_ASSET_PATH: &str = "shaders/oit_compatible_custom_material.wgsl";

/// A custom material
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
struct NoisyOpacityMaterial {
    #[uniform(0)]
    color: LinearRgba,
}

impl Material for NoisyOpacityMaterial {
    fn fragment_shader() -> ShaderRef {
        CUSTOM_MATERIAL_SHADER_ASSET_PATH.into()
    }

    /// For simplicity this material is always transparent
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Optional: specialize the pipeline to deactivate back face culling
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy_render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy_render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
/// This scene demonstrates the integration of OIT into extended/custom materials
fn spawn_custom_material(commands: &mut Commands, resources: &mut SceneResources) {
    let meshes = &mut resources.meshes;
    let ext_materials = &mut resources.extended_materials;
    let custom_materials = &mut resources.custom_materials;
    let materials = &mut resources.materials;

    let render_layers = RenderLayers::layer(1);

    let torus = meshes.add(Torus::new(2.0, 3.0));

    // Spawn a torus with an ExtendedMaterial
    commands.spawn((
        Mesh3d(torus.clone()),
        MeshMaterial3d(ext_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                cull_mode: None,
                ..default()
            },
            extension: CheckeredMaterialExtension {
                color_1: LinearRgba::new(0.9, 0.1, 0.2, 0.4).into(),
                color_2: LinearRgba::new(0.2, 0.1, 0.9, 0.7).into(),
            },
        })),
        Transform::from_rotation(Quat::from_rotation_z(0.4)),
        render_layers.clone(),
    ));

    // Spawn a torus with an custom material
    commands.spawn((
        Mesh3d(torus.clone()),
        MeshMaterial3d(custom_materials.add(NoisyOpacityMaterial {
            color: LinearRgba::new(0.9, 0.6, 0.0, 0.5).into(),
        })),
        Transform::from_rotation(Quat::from_rotation_z(1.0)),
        render_layers.clone(),
    ));

    // Spawn a torus with a StandardMaterial
    commands.spawn((
        Mesh3d(torus.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: LinearRgba::new(0.3, 1.0, 0.1, 0.5).into(),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::from_rotation(Quat::from_rotation_x(1.0)),
        render_layers.clone(),
    ));
}
