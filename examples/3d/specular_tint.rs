//! Demonstrates specular tints and maps.

use std::f32::consts::PI;

use bevy::{color::palettes::css::WHITE, core_pipeline::Skybox, prelude::*, render::view::Hdr};

/// The camera rotation speed in radians per frame.
const ROTATION_SPEED: f32 = 0.005;
/// The rate at which the specular tint hue changes in degrees per frame.
const HUE_SHIFT_SPEED: f32 = 0.2;

static SWITCH_TO_MAP_HELP_TEXT: &str = "Press Space to switch to a specular map";
static SWITCH_TO_SOLID_TINT_HELP_TEXT: &str = "Press Space to switch to a solid specular tint";

/// The current settings the user has chosen.
#[derive(Resource, Default)]
struct AppStatus {
    /// The type of tint (solid or texture map).
    tint_type: TintType,
    /// The hue of the solid tint in radians.
    hue: f32,
}

/// Assets needed by the demo.
#[derive(Resource)]
struct AppAssets {
    /// A color tileable 3D noise texture.
    noise_texture: Handle<Image>,
}

impl FromWorld for AppAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            noise_texture: asset_server.load("textures/AlphaNoise.png"),
        }
    }
}

/// The type of specular tint that the user has selected.
#[derive(Clone, Copy, PartialEq, Default)]
enum TintType {
    /// A solid color.
    #[default]
    Solid,
    /// A Perlin noise texture.
    Map,
}

/// The entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Specular Tint Example".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<AppAssets>()
        .init_resource::<AppStatus>()
        .insert_resource(AmbientLight {
            color: Color::BLACK,
            brightness: 0.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, (toggle_specular_map, update_text).chain())
        .add_systems(Update, shift_hue.after(toggle_specular_map))
        .run();
}

/// Creates the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    app_status: Res<AppStatus>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawns a camera.
    commands.spawn((
        Transform::from_xyz(-2.0, 0.0, 3.5).looking_at(Vec3::ZERO, Vec3::Y),
        Hdr,
        Camera3d::default(),
        Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: 3000.0,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            // We want relatively high intensity here in order for the specular
            // tint to show up well.
            intensity: 25000.0,
            ..default()
        },
    ));

    // Spawn the sphere.
    commands.spawn((
        Transform::from_rotation(Quat::from_rotation_x(PI * 0.5)),
        Mesh3d(meshes.add(Sphere::default().mesh().uv(32, 18))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            // We want only reflected specular light here, so we set the base
            // color as black.
            base_color: Color::BLACK,
            reflectance: 1.0,
            specular_tint: Color::hsva(app_status.hue, 1.0, 1.0, 1.0),
            // The object must not be metallic, or else the reflectance is
            // ignored per the Filament spec:
            //
            // <https://google.github.io/filament/Filament.html#listing_fnormal>
            metallic: 0.0,
            perceptual_roughness: 0.0,
            ..default()
        })),
    ));

    // Spawn the help text.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        app_status.create_text(),
    ));
}

/// Rotates the camera a bit every frame.
fn rotate_camera(mut cameras: Query<&mut Transform, With<Camera3d>>) {
    for mut camera_transform in cameras.iter_mut() {
        camera_transform.translation =
            Quat::from_rotation_y(ROTATION_SPEED) * camera_transform.translation;
        camera_transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Alters the hue of the solid color a bit every frame.
fn shift_hue(
    mut app_status: ResMut<AppStatus>,
    objects_with_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    if app_status.tint_type != TintType::Solid {
        return;
    }

    app_status.hue += HUE_SHIFT_SPEED;

    for material_handle in objects_with_materials.iter() {
        let Some(material) = standard_materials.get_mut(material_handle) else {
            continue;
        };
        material.specular_tint = Color::hsva(app_status.hue, 1.0, 1.0, 1.0);
    }
}

impl AppStatus {
    /// Returns appropriate help text that reflects the current app status.
    fn create_text(&self) -> Text {
        let tint_map_help_text = match self.tint_type {
            TintType::Solid => SWITCH_TO_MAP_HELP_TEXT,
            TintType::Map => SWITCH_TO_SOLID_TINT_HELP_TEXT,
        };

        Text::new(tint_map_help_text)
    }
}

/// Changes the specular tint to a solid color or map when the user presses
/// Space.
fn toggle_specular_map(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    app_assets: Res<AppAssets>,
    objects_with_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    // Swap tint type.
    app_status.tint_type = match app_status.tint_type {
        TintType::Solid => TintType::Map,
        TintType::Map => TintType::Solid,
    };

    for material_handle in objects_with_materials.iter() {
        let Some(material) = standard_materials.get_mut(material_handle) else {
            continue;
        };

        // Adjust the tint type.
        match app_status.tint_type {
            TintType::Solid => {
                material.reflectance = 1.0;
                material.specular_tint_texture = None;
            }
            TintType::Map => {
                // Set reflectance to 2.0 to spread out the map's reflectance
                // range from the default [0.0, 0.5] to [0.0, 1.0].
                material.reflectance = 2.0;
                // As the tint map is multiplied by the tint color, we set the
                // latter to white so that only the map has an effect.
                material.specular_tint = WHITE.into();
                material.specular_tint_texture = Some(app_assets.noise_texture.clone());
            }
        };
    }
}

/// Updates the help text at the bottom of the screen to reflect the current app
/// status.
fn update_text(mut text_query: Query<&mut Text>, app_status: Res<AppStatus>) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_text();
    }
}
