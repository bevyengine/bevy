//! This example shows how irradiance volumes affect the indirect lighting of
//! objects in a scene.
//!
//! The controls are as follows:
//!
//! * Space toggles the irradiance volume on and off.
//!
//! * Enter toggles the camera rotation on and off.
//!
//! * Tab switches the object between a plain sphere and a running fox.
//!
//! * Backspace shows and hides the voxel cubes.
//!
//! * Clicking anywhere moves the object.

use bevy::core_pipeline::Skybox;
use bevy::math::{uvec3, vec3};
use bevy::pbr::irradiance_volume::IrradianceVolume;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, NotShadowCaster};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::window::PrimaryWindow;

// Rotation speed in radians per frame.
const ROTATION_SPEED: f32 = 0.2;

const FOX_SCALE: f32 = 0.05;
const SPHERE_SCALE: f32 = 2.0;

const IRRADIANCE_VOLUME_INTENSITY: f32 = 1800.0;

const AMBIENT_LIGHT_BRIGHTNESS: f32 = 0.06;

const VOXEL_CUBE_SCALE: f32 = 0.4;

static DISABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Space: Disable the irradiance volume";
static ENABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Space: Enable the irradiance volume";

static HIDE_VOXELS_HELP_TEXT: &str = "Backspace: Hide the voxels";
static SHOW_VOXELS_HELP_TEXT: &str = "Backspace: Show the voxels";

static STOP_ROTATION_HELP_TEXT: &str = "Enter: Stop rotation";
static START_ROTATION_HELP_TEXT: &str = "Enter: Start rotation";

static SWITCH_TO_FOX_HELP_TEXT: &str = "Tab: Switch to a skinned mesh";
static SWITCH_TO_SPHERE_HELP_TEXT: &str = "Tab: Switch to a plain sphere mesh";

static CLICK_TO_MOVE_HELP_TEXT: &str = "Left click: Move the object";

static GIZMO_COLOR: Color = Color::YELLOW;

static VOXEL_TRANSFORM: Mat4 = Mat4::from_cols_array_2d(&[
    [-42.317566, 0.0, 0.0, 0.0],
    [0.0, 0.0, 44.601563, 0.0],
    [0.0, 16.73776, 0.0, 0.0],
    [0.0, 6.544792, 0.0, 1.0],
]);

// The mode the application is in.
#[derive(Resource)]
struct AppStatus {
    // Whether the user wants the irradiance volume to be applied.
    irradiance_volume_present: bool,
    // Whether the user wants the unskinned sphere mesh or the skinned fox mesh.
    model: ExampleModel,
    // Whether the user has requested the scene to rotate.
    rotating: bool,
    // Whether the user has requested the voxels to be displayed.
    voxels_visible: bool,
}

// Which model the user wants to display.
#[derive(Clone, Copy, PartialEq)]
enum ExampleModel {
    // The plain sphere.
    Sphere,
    // The fox, which is skinned.
    Fox,
}

// Handles to all the assets used in this example.
#[derive(Resource)]
struct ExampleAssets {
    // The glTF scene containing the colored floor.
    main_scene: Handle<Scene>,

    // The 3D texture containing the irradiance volume.
    irradiance_volume: Handle<Image>,

    // The plain sphere mesh.
    main_sphere: Handle<Mesh>,

    // The material used for the sphere.
    main_sphere_material: Handle<StandardMaterial>,

    // The glTF scene containing the animated fox.
    fox: Handle<Scene>,

    // The animation that the fox will play.
    fox_animation: Handle<AnimationClip>,

    // The voxel cube mesh.
    voxel_cube: Handle<Mesh>,

    // The skybox.
    skybox: Handle<Image>,
}

// The sphere and fox both have this component.
#[derive(Component)]
struct MainObject;

// Marks each of the voxel cubes.
#[derive(Component)]
struct VoxelCube;

// Marks the voxel cube parent object.
#[derive(Component)]
struct VoxelCubeParent;

type VoxelVisualizationMaterial = ExtendedMaterial<StandardMaterial, VoxelVisualizationExtension>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct VoxelVisualizationExtension {
    #[uniform(100)]
    irradiance_volume_info: VoxelVisualizationIrradianceVolumeInfo,
}

#[derive(ShaderType, Debug, Clone)]
struct VoxelVisualizationIrradianceVolumeInfo {
    transform: Mat4,
    inverse_transform: Mat4,
    resolution: UVec3,
    intensity: f32,
}

fn main() {
    // Create the example app.
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Irradiance Volumes Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<VoxelVisualizationMaterial>::default())
        .init_resource::<AppStatus>()
        .init_resource::<ExampleAssets>()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.0,
        })
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, create_cubes)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, play_animations)
        .add_systems(
            Update,
            handle_mouse_clicks
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            change_main_object
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            toggle_irradiance_volumes
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            toggle_voxel_visibility
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            toggle_rotation.after(rotate_camera).after(play_animations),
        )
        .add_systems(
            Update,
            draw_gizmo
                .after(handle_mouse_clicks)
                .after(change_main_object)
                .after(toggle_irradiance_volumes)
                .after(toggle_voxel_visibility)
                .after(toggle_rotation),
        )
        .add_systems(
            Update,
            update_text
                .after(handle_mouse_clicks)
                .after(change_main_object)
                .after(toggle_irradiance_volumes)
                .after(toggle_voxel_visibility)
                .after(toggle_rotation),
        )
        .run();
}

// Spawns all the scene objects.
fn setup(
    mut commands: Commands,
    assets: Res<ExampleAssets>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    spawn_main_scene(&mut commands, &assets);
    spawn_camera(&mut commands, &assets);
    spawn_irradiance_volume(&mut commands, &assets);
    spawn_light(&mut commands);
    spawn_sphere(&mut commands, &assets);
    spawn_voxel_cube_parent(&mut commands);
    spawn_fox(&mut commands, &assets);
    spawn_text(&mut commands, &app_status, &asset_server);
}

fn spawn_main_scene(commands: &mut Commands, assets: &ExampleAssets) {
    commands.spawn(SceneBundle {
        scene: assets.main_scene.clone(),
        ..SceneBundle::default()
    });
}

fn spawn_camera(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-10.012, 4.8605, 13.281).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(Skybox {
            image: assets.skybox.clone(),
            brightness: 150.0,
        });
}

fn spawn_irradiance_volume(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(SpatialBundle {
            transform: Transform::from_matrix(VOXEL_TRANSFORM),
            ..SpatialBundle::default()
        })
        .insert(IrradianceVolume {
            voxels: assets.irradiance_volume.clone(),
            intensity: IRRADIANCE_VOLUME_INTENSITY,
        })
        .insert(LightProbe);
}

fn spawn_light(commands: &mut Commands) {
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 250000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0762, 5.9039, 1.0055),
        ..default()
    });
}

fn spawn_sphere(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(PbrBundle {
            mesh: assets.main_sphere.clone(),
            material: assets.main_sphere_material.clone(),
            transform: Transform::from_xyz(0.0, SPHERE_SCALE, 0.0)
                .with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(MainObject);
}

fn spawn_voxel_cube_parent(commands: &mut Commands) {
    commands
        .spawn(SpatialBundle {
            visibility: Visibility::Hidden,
            ..default()
        })
        .insert(VoxelCubeParent);
}

fn spawn_fox(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(SceneBundle {
            scene: assets.fox.clone(),
            visibility: Visibility::Hidden,
            transform: Transform::from_scale(Vec3::splat(FOX_SCALE)),
            ..default()
        })
        .insert(MainObject);
}

fn spawn_text(commands: &mut Commands, app_status: &AppStatus, asset_server: &AssetServer) {
    commands.spawn(
        TextBundle {
            text: app_status.create_text(asset_server),
            ..TextBundle::default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );
}

// A system that updates the help text.
fn update_text(
    mut text_query: Query<&mut Text>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_text(&asset_server);
    }
}

impl AppStatus {
    // Constructs the help text at the bottom of the screen based on the
    // application status.
    fn create_text(&self, asset_server: &AssetServer) -> Text {
        let irradiance_volume_help_text = if self.irradiance_volume_present {
            DISABLE_IRRADIANCE_VOLUME_HELP_TEXT
        } else {
            ENABLE_IRRADIANCE_VOLUME_HELP_TEXT
        };

        let voxels_help_text = if self.voxels_visible {
            HIDE_VOXELS_HELP_TEXT
        } else {
            SHOW_VOXELS_HELP_TEXT
        };

        let rotation_help_text = if self.rotating {
            STOP_ROTATION_HELP_TEXT
        } else {
            START_ROTATION_HELP_TEXT
        };

        let switch_mesh_help_text = match self.model {
            ExampleModel::Sphere => SWITCH_TO_FOX_HELP_TEXT,
            ExampleModel::Fox => SWITCH_TO_SPHERE_HELP_TEXT,
        };

        Text::from_section(
            format!(
                "{}\n{}\n{}\n{}\n{}",
                CLICK_TO_MOVE_HELP_TEXT,
                voxels_help_text,
                irradiance_volume_help_text,
                rotation_help_text,
                switch_mesh_help_text
            ),
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 24.0,
                color: Color::ANTIQUE_WHITE,
            },
        )
    }
}

// Rotates the camera a bit every frame.
fn rotate_camera(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
    app_status: Res<AppStatus>,
) {
    if !app_status.rotating {
        return;
    }

    for mut transform in camera_query.iter_mut() {
        transform.translation = Vec2::from_angle(ROTATION_SPEED * time.delta_seconds())
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

// Toggles between the unskinned sphere model and the skinned fox model if the
// user requests it.
fn change_main_object(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    mut sphere_query: Query<
        &mut Visibility,
        (With<MainObject>, With<Handle<Mesh>>, Without<Handle<Scene>>),
    >,
    mut fox_query: Query<&mut Visibility, (With<MainObject>, With<Handle<Scene>>)>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }
    let Some(mut sphere_visibility) = sphere_query.iter_mut().next() else {
        return;
    };
    let Some(mut fox_visibility) = fox_query.iter_mut().next() else {
        return;
    };

    match app_status.model {
        ExampleModel::Sphere => {
            *sphere_visibility = Visibility::Hidden;
            *fox_visibility = Visibility::Visible;
            app_status.model = ExampleModel::Fox;
        }
        ExampleModel::Fox => {
            *sphere_visibility = Visibility::Visible;
            *fox_visibility = Visibility::Hidden;
            app_status.model = ExampleModel::Sphere;
        }
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            irradiance_volume_present: true,
            rotating: true,
            model: ExampleModel::Sphere,
            voxels_visible: false,
        }
    }
}

// Turns on and off the irradiance volume as requested by the user.
fn toggle_irradiance_volumes(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    light_probe_query: Query<Entity, With<LightProbe>>,
    mut app_status: ResMut<AppStatus>,
    assets: Res<ExampleAssets>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    };

    let Some(light_probe) = light_probe_query.iter().next() else {
        return;
    };

    if app_status.irradiance_volume_present {
        commands.entity(light_probe).remove::<IrradianceVolume>();
        ambient_light.brightness = AMBIENT_LIGHT_BRIGHTNESS * IRRADIANCE_VOLUME_INTENSITY;
        app_status.irradiance_volume_present = false;
    } else {
        commands.entity(light_probe).insert(IrradianceVolume {
            voxels: assets.irradiance_volume.clone(),
            intensity: IRRADIANCE_VOLUME_INTENSITY,
        });
        ambient_light.brightness = 0.0;
        app_status.irradiance_volume_present = true;
    }
}

fn toggle_rotation(keyboard: Res<ButtonInput<KeyCode>>, mut app_status: ResMut<AppStatus>) {
    if keyboard.just_pressed(KeyCode::Enter) {
        app_status.rotating = !app_status.rotating;
    }
}

// Handles clicks on the plane that reposition the object.
fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut main_objects: Query<&mut Transform, With<MainObject>>,
) {
    if !buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(mouse_position) = windows
        .iter()
        .next()
        .and_then(|window| window.cursor_position())
    else {
        return;
    };
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };

    // Figure out where the user clicked on the plane.
    let Some(ray) = camera.viewport_to_world(camera_transform, mouse_position) else {
        return;
    };
    let Some(ray_distance) = ray.intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y)) else {
        return;
    };
    let plane_intersection = ray.origin + ray.direction.normalize() * ray_distance;

    // Move all the main objeccts.
    for mut transform in main_objects.iter_mut() {
        transform.translation = vec3(
            plane_intersection.x,
            transform.translation.y,
            plane_intersection.z,
        );
    }
}

impl FromWorld for ExampleAssets {
    fn from_world(world: &mut World) -> Self {
        // Load all the assets.
        let asset_server = world.resource::<AssetServer>();
        let fox = asset_server.load("models/animated/Fox.glb#Scene0");
        let main_scene =
            asset_server.load("models/IrradianceVolumeExample/IrradianceVolumeExample.glb#Scene0");
        let irradiance_volume = asset_server.load::<Image>("irradiance_volumes/Example.vxgi.ktx2");
        let fox_animation =
            asset_server.load::<AnimationClip>("models/animated/Fox.glb#Animation1");

        // Just use a specular map for the skybox since it's not too blurry.
        // In reality you wouldn't do this--you'd use a real skybox texture--but
        // reusing the textures like this saves space in the Bevy repository.
        let skybox = asset_server.load::<Image>("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");

        let mut mesh_assets = world.resource_mut::<Assets<Mesh>>();
        let main_sphere = mesh_assets.add(Sphere::default().mesh().uv(32, 18));
        let voxel_cube = mesh_assets.add(Cuboid::default());

        let mut standard_material_assets = world.resource_mut::<Assets<StandardMaterial>>();
        let main_material = standard_material_assets.add(Color::SILVER);

        ExampleAssets {
            main_sphere,
            fox,
            main_sphere_material: main_material,
            main_scene,
            irradiance_volume,
            fox_animation,
            voxel_cube,
            skybox,
        }
    }
}

// Plays the animation on the fox.
fn play_animations(assets: Res<ExampleAssets>, mut players: Query<&mut AnimationPlayer>) {
    for mut player in players.iter_mut() {
        // This will safely do nothing if the animation is already playing.
        player.play(assets.fox_animation.clone()).repeat();
    }
}

fn create_cubes(
    image_assets: Res<Assets<Image>>,
    mut commands: Commands,
    irradiance_volumes: Query<(&IrradianceVolume, &GlobalTransform)>,
    voxel_cube_parents: Query<Entity, With<VoxelCubeParent>>,
    voxel_cubes: Query<Entity, With<VoxelCube>>,
    example_assets: Res<ExampleAssets>,
    mut voxel_visualization_material_assets: ResMut<Assets<VoxelVisualizationMaterial>>,
) {
    // If voxel cubes have already been spawned, don't do anything.
    if !voxel_cubes.is_empty() {
        return;
    }

    let Some(voxel_cube_parent) = voxel_cube_parents.iter().next() else {
        return;
    };

    for (irradiance_volume, global_transform) in irradiance_volumes.iter() {
        let Some(image) = image_assets.get(&irradiance_volume.voxels) else {
            continue;
        };

        let resolution = image.texture_descriptor.size;

        let voxel_cube_material = voxel_visualization_material_assets.add(ExtendedMaterial {
            base: StandardMaterial::from(Color::RED),
            extension: VoxelVisualizationExtension {
                irradiance_volume_info: VoxelVisualizationIrradianceVolumeInfo {
                    transform: VOXEL_TRANSFORM.inverse(),
                    inverse_transform: VOXEL_TRANSFORM,
                    resolution: uvec3(
                        resolution.width,
                        resolution.height,
                        resolution.depth_or_array_layers,
                    ),
                    intensity: IRRADIANCE_VOLUME_INTENSITY,
                },
            },
        });

        let scale = vec3(
            1.0 / resolution.width as f32,
            1.0 / resolution.height as f32,
            1.0 / resolution.depth_or_array_layers as f32,
        );

        // Spawn a cube for each voxel.
        for z in 0..resolution.depth_or_array_layers {
            for y in 0..resolution.height {
                for x in 0..resolution.width {
                    let uvw = (uvec3(x, y, z).as_vec3() + 0.5) * scale - 0.5;
                    let pos = global_transform.transform_point(uvw);
                    let voxel_cube = commands
                        .spawn(MaterialMeshBundle {
                            mesh: example_assets.voxel_cube.clone(),
                            material: voxel_cube_material.clone(),
                            transform: Transform::from_scale(Vec3::splat(VOXEL_CUBE_SCALE))
                                .with_translation(pos),
                            ..default()
                        })
                        .insert(VoxelCube)
                        .insert(NotShadowCaster)
                        .id();

                    commands.entity(voxel_cube_parent).add_child(voxel_cube);
                }
            }
        }
    }
}

// Draws a gizmo showing the bounds of the irradiance volume.
fn draw_gizmo(
    mut gizmos: Gizmos,
    irradiance_volume_query: Query<&GlobalTransform, With<IrradianceVolume>>,
    app_status: Res<AppStatus>,
) {
    if app_status.voxels_visible {
        for transform in irradiance_volume_query.iter() {
            gizmos.cuboid(*transform, GIZMO_COLOR);
        }
    }
}

// Handles a request from the user to toggle the voxel visibility on and off.
fn toggle_voxel_visibility(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    mut voxel_cube_parent_query: Query<&mut Visibility, With<VoxelCubeParent>>,
) {
    if !keyboard.just_pressed(KeyCode::Backspace) {
        return;
    }

    app_status.voxels_visible = !app_status.voxels_visible;

    for mut visibility in voxel_cube_parent_query.iter_mut() {
        *visibility = if app_status.voxels_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

impl MaterialExtension for VoxelVisualizationExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/irradiance_volume_voxel_visualization.wgsl".into()
    }
}
