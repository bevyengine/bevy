//! This example shows how to place reflection probes in the scene.
//!
//! Use the radio buttons to switch between no reflections, environment map
//! reflections (i.e. the skybox only, not the cubes), and static and dynamic
//! full reflections. Static reflections are "baked" ahead of time and
//! consequently rotating the cubes won't change the reflections. Dynamic
//! reflections are updated every frame and so rotating the cube will update
//! them.
//!
//! Reflection probes don't work on WebGL 2 or WebGPU.

use bevy::core_pipeline::core_3d::OmnidirectionalCamera3dBundle;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::view::RenderLayers;
use bevy::{core_pipeline::Skybox, ecs::system::EntityCommands};

use std::{
    f32::consts::PI,
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

static FONT_PATH: &str = "fonts/FiraMono-Medium.ttf";

const SKY_INTENSITY: f32 = 5000.0;
const ENVIRONMENT_MAP_INTENSITY: f32 = 1000.0;

// The mode the application is in.
#[derive(Resource, Default)]
struct AppStatus {
    // Which environment maps the user has requested to display.
    reflection_mode: ReflectionMode,
    // Whether the user has requested the scene to rotate.
    camera_rotating: CameraRotationMode,
    // Whether the user has requested the cubes to rotate.
    cubes_rotating: CubeRotationMode,
}

// Which environment maps the user has requested to display.
#[derive(Clone, Copy, Default, PartialEq)]
enum ReflectionMode {
    // No environment maps are shown.
    None = 0,
    // Only a world environment map is shown.
    EnvironmentMap = 1,
    // Both a world environment map and a reflection probe are present. The
    // reflection probe is shown in the sphere.
    #[default]
    StaticReflectionProbe = 2,
    // Both a world environment map and a dynamic reflection probe, updated
    // every frame, are present.  The reflection probe is shown in the sphere.
    DynamicReflectionProbe = 3,
}

// Whether the user has requested the scene to rotate.
#[derive(Clone, Copy, Default, PartialEq)]
enum CameraRotationMode {
    #[default]
    Rotating,
    Stationary,
}

// Whether the user has requested the cubes to rotate.
#[derive(Clone, Copy, Default, PartialEq)]
enum CubeRotationMode {
    #[default]
    Stationary,
    Rotating,
}

// A marker component that we place on all radio `Button`s.
#[derive(Component, Deref, DerefMut)]
struct RadioButton<T>(T);

// A marker component that we place on all `Text` children of the radio buttons.
#[derive(Component, Deref, DerefMut)]
struct RadioButtonText<T>(T);

// An event that's sent whenever one of the radio buttons changes state.
#[derive(Event)]
struct RadioButtonChangeEvent<T>(PhantomData<T>);

// A marker component for the main viewing camera that renders to the window.
#[derive(Component)]
struct MainCamera;

// A marker component for the reflection camera that generates the reflection in
// the sphere.
#[derive(Component)]
struct ReflectionCamera;

// Stores the original transform for each cube.
//
// We do this so that the cubes will snap back to their original positions when
// rotation is disabled.
#[derive(Component, Deref, DerefMut)]
struct OriginalTransform(Transform);

// The various reflection maps.
#[derive(Resource)]
struct Cubemaps {
    // The blurry diffuse cubemap. This is used for both the world environment
    // map and the reflection probe. (In reality you wouldn't do this, but this
    // reduces complexity of this example a bit.)
    diffuse: Handle<Image>,

    // The specular cubemap that reflects the world, but not the cubes.
    specular_environment_map: Handle<Image>,

    // The static specular cubemap that reflects both the world and the cubes.
    //
    // This is baked ahead of time and consequently won't changes as the cubes
    // rotate.
    static_specular_reflection_probe: Handle<Image>,

    // The dynamic specular cubemap that reflects the world and the cubes,
    // updated in real time.
    dynamic_specular_reflection_probe: Handle<Image>,

    // The skybox cubemap image. This is almost the same as
    // `specular_environment_map`.
    skybox: Handle<Image>,
}

fn main() {
    // Create the app.
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Reflection Probes Example".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<AppStatus>()
        .init_resource::<Cubemaps>()
        .add_event::<RadioButtonChangeEvent<ReflectionMode>>()
        .add_event::<RadioButtonChangeEvent<CameraRotationMode>>()
        .add_event::<RadioButtonChangeEvent<CubeRotationMode>>()
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (
                add_environment_map_to_camera,
                save_original_cubemap_transforms,
            ),
        )
        .add_systems(Update, change_reflection_type)
        .add_systems(Update, toggle_rotation)
        .add_systems(
            Update,
            (rotate_camera, rotate_cubes)
                .after(toggle_rotation)
                .after(change_reflection_type),
        )
        .add_systems(
            Update,
            handle_ui_interactions
                .after(rotate_camera)
                .after(rotate_cubes),
        )
        .add_systems(
            Update,
            (
                update_reflection_mode_radio_buttons,
                update_camera_rotation_mode_radio_buttons,
                update_cube_rotation_mode_radio_buttons,
            )
                .after(handle_ui_interactions),
        )
        .run();
}

// Spawns all the scene objects.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    cubemaps: Res<Cubemaps>,
) {
    let font = asset_server.load(FONT_PATH);

    spawn_scene(&mut commands, &asset_server);
    spawn_main_camera(&mut commands);
    spawn_sphere(&mut commands, &mut meshes, &mut materials);
    spawn_reflection_probes(&mut commands, &cubemaps, ReflectionMode::default());
    spawn_reflection_camera(&mut commands, &cubemaps);
    spawn_buttons(&mut commands, &font);
}

// Spawns the cubes, light, and camera.
fn spawn_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cubes/Cubes.glb")),
        ..SceneBundle::default()
    });
}

// Spawns the camera.
fn spawn_main_camera(commands: &mut Commands) {
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(-6.483, 0.325, 4.381).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(MainCamera);
}

// Creates the sphere mesh and spawns it.
fn spawn_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Create a sphere mesh.
    let sphere_mesh = meshes.add(Sphere::new(1.0).mesh().ico(7).unwrap());

    // Create a sphere.
    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: materials.add(StandardMaterial {
            base_color: Srgba::hex("#ffd891").unwrap().into(),
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..StandardMaterial::default()
        }),
        transform: Transform::default(),
        ..PbrBundle::default()
    });
}

// Spawns the reflection probes.
fn spawn_reflection_probes(commands: &mut Commands, cubemaps: &Cubemaps, mode: ReflectionMode) {
    if mode == ReflectionMode::None {
        return;
    }

    // Spawn the static light probe.
    //
    // This is on render layer 1 so that it can be hidden when the static
    // reflection mode isn't in use.
    commands
        .spawn(ReflectionProbeBundle {
            spatial: SpatialBundle {
                // 2.0 because the sphere's radius is 1.0 and we want to fully enclose it.
                transform: Transform::from_scale(Vec3::splat(2.0)),
                ..SpatialBundle::default()
            },
            light_probe: LightProbe,
            environment_map: EnvironmentMapLight {
                diffuse_map: cubemaps.diffuse.clone(),
                specular_map: match mode {
                    ReflectionMode::DynamicReflectionProbe => {
                        cubemaps.dynamic_specular_reflection_probe.clone()
                    }
                    ReflectionMode::StaticReflectionProbe => {
                        cubemaps.static_specular_reflection_probe.clone()
                    }
                    ReflectionMode::EnvironmentMap | ReflectionMode::None => {
                        cubemaps.specular_environment_map.clone()
                    }
                },
                intensity: match mode {
                    ReflectionMode::DynamicReflectionProbe
                    | ReflectionMode::StaticReflectionProbe => ENVIRONMENT_MAP_INTENSITY,
                    ReflectionMode::EnvironmentMap | ReflectionMode::None => SKY_INTENSITY,
                },
            },
        })
        .insert(RenderLayers::layer(1));

    if mode != ReflectionMode::DynamicReflectionProbe {
        return;
    }

    // Spawn the dynamic light probe, which provides a reflection that's updated
    // every frame.
    //
    // This is on render layer 2 so that it won't be applied to the rendering of
    // the reflection itself, which would be circular.
    commands
        .spawn(ReflectionProbeBundle {
            spatial: SpatialBundle {
                // 2.0 because the sphere's radius is 1.0 and we want to fully enclose it.
                transform: Transform::from_scale(Vec3::splat(2.0)),
                ..SpatialBundle::default()
            },
            light_probe: LightProbe,
            environment_map: EnvironmentMapLight {
                diffuse_map: cubemaps.diffuse.clone(),
                specular_map: cubemaps.dynamic_specular_reflection_probe.clone(),
                intensity: ENVIRONMENT_MAP_INTENSITY,
            },
        })
        .insert(RenderLayers::layer(2));
}

// Spawns the omnidirectional camera that provides the dynamic reflection probe.
fn spawn_reflection_camera(commands: &mut Commands, cubemaps: &Cubemaps) {
    commands
        .spawn(OmnidirectionalCamera3dBundle {
            camera: Camera {
                target: RenderTarget::Image(cubemaps.dynamic_specular_reflection_probe.clone()),
                order: -1,
                hdr: true,
                is_active: false,
                ..default()
            },
            tonemapping: Tonemapping::None,
            ..default()
        })
        .insert(ReflectionCamera)
        .insert(Skybox {
            image: cubemaps.skybox.clone(),
            brightness: SKY_INTENSITY,
        })
        .insert(EnvironmentMapLight {
            diffuse_map: cubemaps.diffuse.clone(),
            specular_map: cubemaps.static_specular_reflection_probe.clone(),
            intensity: SKY_INTENSITY,
        })
        .insert(RenderLayers::from_layers(&[0, 1]));
}

// Adds a world environment map to the camera. This separate system is needed because the camera is
// managed by the scene spawner, as it's part of the glTF file with the cubes, so we have to add
// the environment map after the fact.
fn add_environment_map_to_camera(
    mut commands: Commands,
    main_camera_query: Query<Entity, (Added<Camera3d>, With<MainCamera>)>,
    cubemaps: Res<Cubemaps>,
) {
    for camera_entity in main_camera_query.iter() {
        commands
            .entity(camera_entity)
            .insert(create_camera_environment_map_light(&cubemaps))
            .insert(Skybox {
                image: cubemaps.skybox.clone(),
                brightness: SKY_INTENSITY,
            });
    }
}

// Stores the original transform on the cubes so we can restore it later.
fn save_original_cubemap_transforms(
    mut commands: Commands,
    mut cubes: Query<
        (Entity, &Transform),
        (With<Handle<Mesh>>, With<Name>, Without<OriginalTransform>),
    >,
) {
    for (cube, cube_transform) in cubes.iter_mut() {
        commands
            .entity(cube)
            .insert(OriginalTransform(*cube_transform));
    }
}

// A system that handles switching between different reflection modes.
fn change_reflection_type(
    mut commands: Commands,
    main_camera_query: Query<Entity, (With<Camera3d>, With<MainCamera>)>,
    mut reflection_camera_query: Query<&mut Camera, (With<Camera3d>, With<ReflectionCamera>)>,
    mut reflection_probe_query: Query<Entity, With<LightProbe>>,
    app_status: ResMut<AppStatus>,
    cubemaps: Res<Cubemaps>,
    mut reflection_mode_change_events: EventReader<RadioButtonChangeEvent<ReflectionMode>>,
) {
    if reflection_mode_change_events.read().count() == 0 {
        return;
    }

    for camera_entity in main_camera_query.iter() {
        // Add or remove the reflection probes.
        for reflection_probe in reflection_probe_query.iter_mut() {
            commands.entity(reflection_probe).despawn();
        }
        spawn_reflection_probes(&mut commands, &cubemaps, app_status.reflection_mode);

        // Add or remove the environment map from the camera.
        match app_status.reflection_mode {
            ReflectionMode::None => {
                commands
                    .entity(camera_entity)
                    .remove::<EnvironmentMapLight>();
            }
            ReflectionMode::EnvironmentMap
            | ReflectionMode::StaticReflectionProbe
            | ReflectionMode::DynamicReflectionProbe => {
                commands
                    .entity(camera_entity)
                    .insert(create_camera_environment_map_light(&cubemaps));
            }
        }

        // Set the render layers for the camera.
        match app_status.reflection_mode {
            ReflectionMode::DynamicReflectionProbe => {
                commands
                    .entity(camera_entity)
                    .insert(RenderLayers::from_layers(&[0, 2]));
            }
            ReflectionMode::None
            | ReflectionMode::EnvironmentMap
            | ReflectionMode::StaticReflectionProbe => {
                commands.entity(camera_entity).remove::<RenderLayers>();
            }
        }
    }

    // Enable or disable the reflection camera.
    for mut camera in reflection_camera_query.iter_mut() {
        camera.is_active = app_status.reflection_mode == ReflectionMode::DynamicReflectionProbe;
    }
}

// A system that handles enabling and disabling rotation.
fn toggle_rotation(keyboard: Res<ButtonInput<KeyCode>>, mut app_status: ResMut<AppStatus>) {
    if keyboard.just_pressed(KeyCode::Enter) {
        app_status.camera_rotating = match app_status.camera_rotating {
            CameraRotationMode::Rotating => CameraRotationMode::Stationary,
            CameraRotationMode::Stationary => CameraRotationMode::Rotating,
        }
    }
}

impl TryFrom<u32> for ReflectionMode {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReflectionMode::None),
            1 => Ok(ReflectionMode::EnvironmentMap),
            2 => Ok(ReflectionMode::StaticReflectionProbe),
            3 => Ok(ReflectionMode::DynamicReflectionProbe),
            _ => Err(()),
        }
    }
}

impl Display for ReflectionMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let text = match *self {
            ReflectionMode::None => "No reflections",
            ReflectionMode::EnvironmentMap => "Environment map",
            ReflectionMode::StaticReflectionProbe => "Static reflection probe",
            ReflectionMode::DynamicReflectionProbe => "Dynamic reflection probe",
        };
        formatter.write_str(text)
    }
}

// Creates the world environment map light, used as a fallback if no reflection
// probe is applicable to a mesh.
fn create_camera_environment_map_light(cubemaps: &Cubemaps) -> EnvironmentMapLight {
    EnvironmentMapLight {
        diffuse_map: cubemaps.diffuse.clone(),
        specular_map: cubemaps.specular_environment_map.clone(),
        intensity: SKY_INTENSITY,
    }
}

// Rotates the camera a bit every frame, if enabled.
fn rotate_camera(
    time: Res<Time>,
    mut main_camera_query: Query<&mut Transform, (With<Camera3d>, With<MainCamera>)>,
    app_status: Res<AppStatus>,
) {
    if app_status.camera_rotating != CameraRotationMode::Rotating {
        return;
    }

    for mut transform in main_camera_query.iter_mut() {
        transform.translation = Vec2::from_angle(time.delta_seconds() * PI / 5.0)
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

// If cube rotation is enabled, rotates the cubes a bit every frame; otherwise,
// resets them to their original orientation.
fn rotate_cubes(
    time: Res<Time>,
    mut cube_query: Query<(&mut Transform, &OriginalTransform), (With<Handle<Mesh>>, With<Name>)>,
    app_status: Res<AppStatus>,
) {
    let delta_time = time.delta_seconds();
    for (mut transform, original_transform) in cube_query.iter_mut() {
        match app_status.cubes_rotating {
            CubeRotationMode::Rotating => {
                transform.rotate(Quat::from_euler(
                    EulerRot::XZY,
                    delta_time * 4.2,
                    delta_time * -3.0,
                    delta_time * 1.38,
                ));
            }
            CubeRotationMode::Stationary => *transform = **original_transform,
        }
    }
}

// Loads the cubemaps from the assets directory.
impl FromWorld for Cubemaps {
    fn from_world(world: &mut World) -> Self {
        // Just use the specular map for the skybox since it's not too blurry.
        // In reality you wouldn't do this--you'd use a real skybox texture--but
        // reusing the textures like this saves space in the Bevy repository.
        let specular_map = world.load_asset("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");

        let dynamic_specular_reflection_probe_size = Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 6,
        };

        let mut dynamic_specular_reflection_probe = Image {
            texture_descriptor: TextureDescriptor {
                label: Some("dynamic specular reflection probe"),
                size: dynamic_specular_reflection_probe_size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            texture_view_descriptor: Some(TextureViewDescriptor {
                label: Some("dynamic specular reflection probe view"),
                format: None,
                dimension: Some(TextureViewDimension::Cube),
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            }),
            ..default()
        };

        dynamic_specular_reflection_probe.resize(dynamic_specular_reflection_probe_size);

        Cubemaps {
            diffuse: world.load_asset("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            static_specular_reflection_probe: world
                .load_asset("environment_maps/cubes_reflection_probe_specular_rgb9e5_zstd.ktx2"),
            dynamic_specular_reflection_probe: world.add_asset(dynamic_specular_reflection_probe),
            specular_environment_map: specular_map.clone(),
            skybox: specular_map,
        }
    }
}

// Spawns all the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                row_gap: Val::Px(6.0),
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Spawn the camera rotation buttons in the first row.
            spawn_option_buttons(
                parent,
                "Camera Rotation",
                &[
                    (CameraRotationMode::Rotating, "On"),
                    (CameraRotationMode::Stationary, "Off"),
                ],
                font,
            );

            // Spawn the cube rotation buttons in the second row.
            spawn_option_buttons(
                parent,
                "Cube Rotation",
                &[
                    (CubeRotationMode::Rotating, "On"),
                    (CubeRotationMode::Stationary, "Off"),
                ],
                font,
            );

            // Spawn the reflection mode buttons in the third row.
            spawn_option_buttons(
                parent,
                "Reflection Mode",
                &[
                    (ReflectionMode::None, "None"),
                    (ReflectionMode::EnvironmentMap, "Environment Map"),
                    (
                        ReflectionMode::StaticReflectionProbe,
                        "Static Reflection Probe",
                    ),
                    (
                        ReflectionMode::DynamicReflectionProbe,
                        "Dynamic Reflection Probe",
                    ),
                ],
                font,
            );
        });
}

// Spawns the buttons that allow configuration of a setting.
//
// The user may change the setting to any one of the labeled `options`.
fn spawn_option_buttons<T>(
    parent: &mut ChildBuilder,
    title: &str,
    options: &[(T, &str)],
    font: &Handle<Font>,
) where
    T: Clone + Send + Sync + 'static,
{
    // Add the parent node for the row.
    parent
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            spawn_ui_text(parent, title, font, Color::WHITE).insert(Style {
                width: Val::Px(150.0),
                ..default()
            });

            for (option_index, (option_value, option_name)) in options.iter().enumerate() {
                spawn_option_button(
                    parent,
                    option_value,
                    option_name,
                    option_index == 0,
                    option_index == 0,
                    option_index == options.len() - 1,
                    font,
                );
            }
        });
}

/// Spawns a single radio button that allows configuration of a setting.
///
/// The type parameter specifies the particular setting: one of `ReflectionMode`
/// or `RotationMode`.
fn spawn_option_button<T>(
    parent: &mut ChildBuilder,
    option_value: &T,
    option_name: &str,
    is_selected: bool,
    is_first: bool,
    is_last: bool,
    font: &Handle<Font>,
) where
    T: Clone + Send + Sync + 'static,
{
    let (bg_color, fg_color) = if is_selected {
        (Color::WHITE, Color::BLACK)
    } else {
        (Color::BLACK, Color::WHITE)
    };

    // Add the button node.
    parent
        .spawn(ButtonBundle {
            style: Style {
                border: UiRect::all(Val::Px(1.0)).with_left(if is_first {
                    Val::Px(1.0)
                } else {
                    Val::Px(0.0)
                }),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            border_color: BorderColor(Color::WHITE),
            border_radius: BorderRadius::ZERO
                .with_left(if is_first { Val::Px(6.0) } else { Val::Px(0.0) })
                .with_right(if is_last { Val::Px(6.0) } else { Val::Px(0.0) }),
            image: UiImage::default().with_color(bg_color),
            ..default()
        })
        .insert(RadioButton(option_value.clone()))
        .with_children(|parent| {
            spawn_ui_text(parent, option_name, font, fg_color)
                .insert(RadioButtonText(option_value.clone()));
        });
}

/// Updates the style of the button part of a radio button to reflect its
/// selected status.
fn update_ui_radio_button<T>(image: &mut UiImage, radio_button: &RadioButton<T>, value: T)
where
    T: PartialEq,
{
    *image = UiImage::default().with_color(if value == **radio_button {
        Color::WHITE
    } else {
        Color::BLACK
    });
}

// Updates the style of the label of a radio button to reflect its selected
// status.
fn update_ui_radio_button_text<T>(text: &mut Text, radio_button_text: &RadioButtonText<T>, value: T)
where
    T: PartialEq,
{
    let text_color = if value == **radio_button_text {
        Color::BLACK
    } else {
        Color::WHITE
    };

    for section in &mut text.sections {
        section.style.color = text_color;
    }
}

// Spawns text for the UI.
//
// Returns the `EntityCommands`, which allow further customization of the text
// style.
fn spawn_ui_text<'a>(
    parent: &'a mut ChildBuilder,
    label: &str,
    font: &Handle<Font>,
    color: Color,
) -> EntityCommands<'a> {
    parent.spawn(TextBundle::from_section(
        label,
        TextStyle {
            font: font.clone(),
            font_size: 18.0,
            color,
        },
    ))
}

// Handles clicks on the various radio buttons.
fn handle_ui_interactions(
    mut interactions: Query<
        (
            &Interaction,
            AnyOf<(
                &RadioButton<ReflectionMode>,
                &RadioButton<CameraRotationMode>,
                &RadioButton<CubeRotationMode>,
            )>,
        ),
        With<Button>,
    >,
    mut reflection_mode_events: EventWriter<RadioButtonChangeEvent<ReflectionMode>>,
    mut camera_rotation_mode_events: EventWriter<RadioButtonChangeEvent<CameraRotationMode>>,
    mut cube_rotation_mode_events: EventWriter<RadioButtonChangeEvent<CubeRotationMode>>,
    mut app_status: ResMut<AppStatus>,
) {
    for (
        interaction,
        (maybe_reflection_mode, maybe_camera_rotation_mode, maybe_cube_rotation_mode),
    ) in interactions.iter_mut()
    {
        // Only handle clicks.
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Check each setting. If the clicked button matched one, then send the
        // appropriate event.
        if let Some(reflection_mode) = maybe_reflection_mode {
            app_status.reflection_mode = **reflection_mode;
            reflection_mode_events.send(RadioButtonChangeEvent(PhantomData));
        }
        if let Some(camera_rotation_mode) = maybe_camera_rotation_mode {
            app_status.camera_rotating = **camera_rotation_mode;
            camera_rotation_mode_events.send(RadioButtonChangeEvent(PhantomData));
        }
        if let Some(cube_rotation_mode) = maybe_cube_rotation_mode {
            app_status.cubes_rotating = **cube_rotation_mode;
            cube_rotation_mode_events.send(RadioButtonChangeEvent(PhantomData));
        }
    }
}

// Updates the style of the radio buttons that select the reflection mode to reflect
// the reflection mode in use.
fn update_reflection_mode_radio_buttons(
    mut reflection_mode_buttons: Query<(&mut UiImage, &RadioButton<ReflectionMode>)>,
    mut reflection_mode_button_texts: Query<
        (&mut Text, &RadioButtonText<ReflectionMode>),
        Without<UiImage>,
    >,
    app_status: Res<AppStatus>,
) {
    for (mut button_style, button) in reflection_mode_buttons.iter_mut() {
        update_ui_radio_button(&mut button_style, button, app_status.reflection_mode);
    }
    for (mut button_text_style, button_text) in reflection_mode_button_texts.iter_mut() {
        update_ui_radio_button_text(
            &mut button_text_style,
            button_text,
            app_status.reflection_mode,
        );
    }
}

// Updates the style of the radio buttons that select the camera rotation mode
// to reflect the camera rotation mode in use.
fn update_camera_rotation_mode_radio_buttons(
    mut rotation_mode_buttons: Query<(&mut UiImage, &RadioButton<CameraRotationMode>)>,
    mut rotation_mode_button_texts: Query<
        (&mut Text, &RadioButtonText<CameraRotationMode>),
        Without<UiImage>,
    >,
    app_status: Res<AppStatus>,
) {
    for (mut button_style, button) in rotation_mode_buttons.iter_mut() {
        update_ui_radio_button(&mut button_style, button, app_status.camera_rotating);
    }
    for (mut button_text_style, button_text) in rotation_mode_button_texts.iter_mut() {
        update_ui_radio_button_text(
            &mut button_text_style,
            button_text,
            app_status.camera_rotating,
        );
    }
}

// Updates the style of the radio buttons that select the cube rotation mode
// to reflect the cube rotation mode in use.
fn update_cube_rotation_mode_radio_buttons(
    mut rotation_mode_buttons: Query<(&mut UiImage, &RadioButton<CubeRotationMode>)>,
    mut rotation_mode_button_texts: Query<
        (&mut Text, &RadioButtonText<CubeRotationMode>),
        Without<UiImage>,
    >,
    app_status: Res<AppStatus>,
) {
    for (mut button_style, button) in rotation_mode_buttons.iter_mut() {
        update_ui_radio_button(&mut button_style, button, app_status.cubes_rotating);
    }
    for (mut button_text_style, button_text) in rotation_mode_button_texts.iter_mut() {
        update_ui_radio_button_text(
            &mut button_text_style,
            button_text,
            app_status.cubes_rotating,
        );
    }
}
