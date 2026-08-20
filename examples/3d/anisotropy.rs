//! Demonstrates anisotropy with the glTF sample barn lamp model.

use bevy::{
    color::palettes::{self, css::WHITE},
    feathers::{controls::FeathersCheckbox, theme::UiTheme, FeathersPlugins},
    light::Skybox,
    math::vec3,
    prelude::*,
    time::Stopwatch,
    ui_widgets::{checkbox_self_update, radio_self_update, ValueChange},
};
use checkbox::{feathers_option_checkbox, IsChecked};
use radio::{feathers_option_buttons, RadioButtonOptionValue};
use scene::bottom_left_scene;

#[path = "../helpers/radio.rs"]
#[expect(dead_code, reason = "main_ui_node_scene not used in this example")]
mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

#[path = "../helpers/checkbox.rs"]
mod checkbox;

#[path = "../helpers/scene.rs"]
#[expect(dead_code, reason = "some scenes are not used in this example")]
mod scene;

/// The initial position of the camera.
const CAMERA_INITIAL_POSITION: Vec3 = vec3(-0.4, 0.0, 0.0);

/// The current settings of the app, as chosen by the user.
#[derive(Resource)]
struct AppStatus {
    /// Which type of light is in the scene.
    light_mode: LightMode,
    /// Whether anisotropy is enabled.
    anisotropy_enabled: bool,
    /// Which mesh is visible
    visible_scene: Scene,
}

/// Which type of light we're using: a directional light, a point light, or an
/// environment map.
#[derive(Clone, Copy, PartialEq, Resource, Default, Debug)]
enum LightMode {
    /// A rotating directional light.
    #[default]
    Directional,
    /// A rotating point light.
    Point,
    /// An environment map (image-based lighting, including skybox).
    EnvironmentMap,
}

/// A component that stores the version of the material with anisotropy and the
/// version of the material without it.
///
/// This is placed on each mesh with a material. It exists so that the
/// appropriate system can replace the materials when the user presses Enter to
/// turn anisotropy on and off.
#[derive(Component)]
struct MaterialVariants {
    /// The version of the material in the glTF file, with anisotropy.
    anisotropic: Handle<StandardMaterial>,
    /// The version of the material with anisotropy removed.
    isotropic: Handle<StandardMaterial>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
enum Scene {
    #[default]
    BarnLamp,
    Sphere,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
enum CheckboxInput {
    #[default]
    Anisotropy,
}

/// The application entry point.
fn main() {
    App::new()
        .init_resource::<AppStatus>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Anisotropy Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FeathersPlugins)
        .init_resource::<LightMode>()
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .add_systems(Startup, setup)
        .add_systems(Update, create_material_variants)
        .add_systems(Update, animate_light)
        .add_systems(Update, rotate_camera)
        .add_observer(handle_value_change_checkbox)
        .add_observer(handle_scene_selection_change)
        .add_observer(handle_light_mode_selection_change)
        .add_observer(radio_self_update)
        .add_observer(checkbox_self_update)
        .run();
}

/// Creates the initial scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, light_mode: Res<LightMode>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_INITIAL_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    spawn_directional_light(&mut commands);

    commands.spawn((
        WorldAssetRoot(
            asset_server.load("models/AnisotropyBarnLamp/AnisotropyBarnLamp.gltf#Scene0"),
        ),
        Transform::from_xyz(0.0, 0.07, -0.13),
        Scene::BarnLamp,
    ));

    commands.spawn((
        Mesh3d(
            asset_server.add(
                Mesh::from(Sphere::new(0.1))
                    .with_generated_tangents()
                    .unwrap(),
            ),
        ),
        MeshMaterial3d(asset_server.add(StandardMaterial {
            base_color: palettes::tailwind::GRAY_300.into(),
            anisotropy_rotation: 0.5,
            anisotropy_strength: 1.,
            ..default()
        })),
        Scene::Sphere,
        Visibility::Hidden,
    ));

    spawn_buttons(&mut commands, light_mode, Scene::BarnLamp);
}

/// Spawns UI controls in the bottom left corner of the screen.
fn spawn_buttons(commands: &mut Commands, light_mode: Res<LightMode>, scene: Scene) {
    commands.spawn_scene(bsn! {
        bottom_left_scene()
            Children [
            feathers_option_checkbox("Enable Anisotropy",
                Some(CheckboxInput::Anisotropy),
                IsChecked(true),
            ),
            feathers_option_buttons(
                "",
                &[
                    (Scene::BarnLamp, "Barn Lamp"),
                    (Scene::Sphere, "Sphere"),
                ],
                if scene == Scene::Sphere { 1 } else { 0 },
            ),
            feathers_option_buttons(
                "",
                &[
                    (LightMode::EnvironmentMap, "Environment Map"),
                    (LightMode::Directional, "Directional"),
                    (LightMode::Point, "Point"),
                ],
                if *light_mode == LightMode::Point { 0 } else { 1 },
            ),
        ]
    });
}

/// Updates the light mode when the user toggles that setting's radio.
fn handle_light_mode_selection_change(
    event: On<ValueChange<Entity>>,
    commands: Commands,
    cameras: Query<Entity, With<Camera>>,
    asset_server: Res<AssetServer>,
    mut app_status: ResMut<AppStatus>,
    lights: Query<Entity, Or<(With<PointLight>, With<DirectionalLight>)>>,
    new_value_query: Query<&RadioButtonOptionValue<LightMode>>,
) {
    let Ok(RadioButtonOptionValue(selection)) = new_value_query.get(event.value) else {
        return;
    };
    let old_setting = app_status.light_mode;

    app_status.light_mode = *selection;
    light_mode_update_helper(
        commands,
        asset_server,
        old_setting,
        app_status.light_mode,
        cameras,
        lights,
    );
}

/// Helper to spawn new light entities on a selection change.
fn light_mode_spawn_helper(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    new_setting: LightMode,
    cameras: Query<Entity, With<Camera>>,
) {
    match new_setting {
        LightMode::Directional => {
            spawn_directional_light(&mut commands);
        }
        LightMode::Point => {
            spawn_point_light(&mut commands);
        }
        LightMode::EnvironmentMap => {
            for camera in cameras.iter() {
                add_skybox_and_environment_map(&mut commands, &asset_server, camera);
            }
        }
    }
}

/// Helper to teardown light entities on a selection change.
fn light_mode_update_helper(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    old_setting: LightMode,
    new_setting: LightMode,
    cameras: Query<Entity, With<Camera>>,
    lights: Query<Entity, Or<(With<PointLight>, With<DirectionalLight>)>>,
) {
    match old_setting {
        LightMode::Directional | LightMode::Point => {
            for light in lights.iter() {
                commands.entity(light).despawn();
            }
        }
        LightMode::EnvironmentMap => {
            for camera in cameras.iter() {
                commands
                    .entity(camera)
                    .remove::<Skybox>()
                    .remove::<EnvironmentMapLight>();
            }
        }
    }
    light_mode_spawn_helper(commands, asset_server, new_setting, cameras);
}

/// Updates the scene when the user toggles that setting's radio.
fn handle_scene_selection_change(
    event: On<ValueChange<Entity>>,
    mut app_status: ResMut<AppStatus>,
    new_value_query: Query<&RadioButtonOptionValue<Scene>>,
    mut scenes: Query<(&mut Visibility, &Scene)>,
) {
    let Ok(RadioButtonOptionValue(scene_selection)) = new_value_query.get(event.value) else {
        return;
    };
    app_status.visible_scene = *scene_selection;
    for (mut visibility, scene) in scenes.iter_mut() {
        let new_vis = if *scene == app_status.visible_scene {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *visibility = new_vis;
    }
}

fn handle_value_change_checkbox(
    event: On<ValueChange<bool>>,
    mut app_status: ResMut<AppStatus>,
    checkbox_input_q: Query<&CheckboxInput, With<FeathersCheckbox>>,
    mut meshes: Query<(&mut MeshMaterial3d<StandardMaterial>, &MaterialVariants)>,
) {
    if let Ok(checkbox_input) = checkbox_input_q.get(event.source) {
        match checkbox_input {
            CheckboxInput::Anisotropy => {
                app_status.anisotropy_enabled = event.value;
            }
        }
    };

    // Go through each mesh and alter its material.
    for (mut material_handle, material_variants) in meshes.iter_mut() {
        material_handle.0 = if app_status.anisotropy_enabled {
            material_variants.anisotropic.clone()
        } else {
            material_variants.isotropic.clone()
        }
    }
}

/// For each material, creates a version with the anisotropy removed.
///
/// This allows the user to check a checkbox to toggle anisotropy on and off.
fn create_material_variants(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_meshes: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (
            Added<MeshMaterial3d<StandardMaterial>>,
            Without<MaterialVariants>,
        ),
    >,
) {
    for (entity, anisotropic_material_handle) in new_meshes.iter() {
        let Some(anisotropic_material) = materials.get(anisotropic_material_handle).cloned() else {
            continue;
        };

        commands.entity(entity).insert(MaterialVariants {
            anisotropic: anisotropic_material_handle.0.clone(),
            isotropic: materials.add(StandardMaterial {
                anisotropy_strength: 0.0,
                anisotropy_rotation: 0.0,
                ..anisotropic_material
            }),
        });
    }
}

/// A system that animates the light every frame, if there is one.
fn animate_light(
    mut lights: Query<&mut Transform, Or<(With<DirectionalLight>, With<PointLight>)>>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for mut transform in lights.iter_mut() {
        transform.translation = vec3(ops::cos(now), 1.0, ops::sin(now)) * vec3(3.0, 4.0, 3.0);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// A system that rotates the camera if the environment map is enabled.
fn rotate_camera(
    mut camera: Query<&mut Transform, With<Camera>>,
    app_status: Res<AppStatus>,
    time: Res<Time>,
    mut stopwatch: Local<Stopwatch>,
) {
    if app_status.light_mode == LightMode::EnvironmentMap {
        stopwatch.tick(time.delta());
    }

    let now = stopwatch.elapsed_secs();
    for mut transform in camera.iter_mut() {
        *transform = Transform::from_translation(
            Quat::from_rotation_y(now).mul_vec3(CAMERA_INITIAL_POSITION),
        )
        .looking_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Adds the skybox and environment map to the scene.
fn add_skybox_and_environment_map(
    commands: &mut Commands,
    asset_server: &AssetServer,
    entity: Entity,
) {
    commands
        .entity(entity)
        .insert(Skybox {
            brightness: 5000.0,
            image: Some(asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2")),
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2500.0,
            ..default()
        });
}

/// Spawns a rotating directional light.
fn spawn_directional_light(commands: &mut Commands) {
    commands.spawn(DirectionalLight {
        color: WHITE.into(),
        illuminance: 3000.0,
        ..default()
    });
}

/// Spawns a rotating point light.
fn spawn_point_light(commands: &mut Commands) {
    commands.spawn(PointLight {
        color: WHITE.into(),
        intensity: 200000.0,
        ..default()
    });
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            light_mode: default(),
            anisotropy_enabled: true,
            visible_scene: default(),
        }
    }
}
