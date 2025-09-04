//! Demonstrates how to combine baked and dynamic lighting.

use bevy::{
    gltf::GltfMeshName,
    pbr::Lightmap,
    picking::{backend::HitData, pointer::PointerInteraction},
    prelude::*,
    scene::SceneInstanceReady,
};

use crate::widgets::{RadioButton, RadioButtonText, WidgetClickEvent, WidgetClickSender};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// How bright the lightmaps are.
const LIGHTMAP_EXPOSURE: f32 = 600.0;

/// How far above the ground the sphere's origin is when moved, in scene units.
const SPHERE_OFFSET: f32 = 0.2;

/// The settings that the user has currently chosen for the app.
#[derive(Clone, Default, Resource)]
struct AppStatus {
    /// The lighting mode that the user currently has set: baked, mixed, or
    /// real-time.
    lighting_mode: LightingMode,
}

/// The type of lighting to use in the scene.
#[derive(Clone, Copy, PartialEq, Default)]
enum LightingMode {
    /// All light is computed ahead of time; no lighting takes place at runtime.
    ///
    /// In this mode, the sphere can't be moved, as the light shining on it was
    /// precomputed. On the plus side, the sphere has indirect lighting in this
    /// mode, as the red hue on the bottom of the sphere demonstrates.
    Baked,

    /// All light for the static objects is computed ahead of time, but the
    /// light for the dynamic sphere is computed at runtime.
    ///
    /// In this mode, the sphere can be moved, and the light will be computed
    /// for it as you do so. The sphere loses indirect illumination; notice the
    /// lack of a red hue at the base of the sphere. However, the rest of the
    /// scene has indirect illumination. Note also that the sphere doesn't cast
    /// a shadow on the static objects in this mode, because shadows are part of
    /// the lighting computation.
    MixedDirect,

    /// Indirect light for the static objects is computed ahead of time, and
    /// direct light for all objects is computed at runtime.
    ///
    /// In this mode, the sphere can be moved, and the light will be computed
    /// for it as you do so. The sphere loses indirect illumination; notice the
    /// lack of a red hue at the base of the sphere. However, the rest of the
    /// scene has indirect illumination. The sphere does cast a shadow on
    /// objects in this mode, because the direct light for all objects is being
    /// computed dynamically.
    #[default]
    MixedIndirect,

    /// Light is computed at runtime for all objects.
    ///
    /// In this mode, no lightmaps are used at all. All objects are dynamically
    /// lit, which provides maximum flexibility. However, the downside is that
    /// global illumination is lost; note that the base of the sphere isn't red
    /// as it is in baked mode.
    RealTime,
}

/// An event that's fired whenever the user changes the lighting mode.
///
/// This is also fired when the scene loads for the first time.
#[derive(Clone, Copy, Default, BufferedEvent)]
struct LightingModeChanged;

#[derive(Clone, Copy, Component, Debug)]
struct HelpText;

/// The name of every static object in the scene that has a lightmap, as well as
/// the UV rect of its lightmap.
///
/// Storing this as an array and doing a linear search through it is rather
/// inefficient, but we do it anyway for clarity's sake.
static LIGHTMAPS: [(&str, Rect); 5] = [
    (
        "Plane",
        uv_rect_opengl(Vec2::splat(0.026), Vec2::splat(0.710)),
    ),
    (
        "SheenChair_fabric",
        uv_rect_opengl(vec2(0.7864, 0.02377), vec2(0.1910, 0.1912)),
    ),
    (
        "SheenChair_label",
        uv_rect_opengl(vec2(0.275, -0.016), vec2(0.858, 0.486)),
    ),
    (
        "SheenChair_metal",
        uv_rect_opengl(vec2(0.998, 0.506), vec2(-0.029, -0.067)),
    ),
    (
        "SheenChair_wood",
        uv_rect_opengl(vec2(0.787, 0.257), vec2(0.179, 0.177)),
    ),
];

static SPHERE_UV_RECT: Rect = uv_rect_opengl(vec2(0.788, 0.484), Vec2::splat(0.062));

/// The initial position of the sphere.
///
/// When the user sets the light mode to [`LightingMode::Baked`], we reset the
/// position to this point.
const INITIAL_SPHERE_POSITION: Vec3 = vec3(0.0, 0.5233223, 0.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Mixed Lighting Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .insert_resource(AmbientLight {
            color: ClearColor::default().0,
            brightness: 10000.0,
            affects_lightmapped_meshes: true,
        })
        .init_resource::<AppStatus>()
        .add_event::<WidgetClickEvent<LightingMode>>()
        .add_event::<LightingModeChanged>()
        .add_systems(Startup, setup)
        .add_systems(Update, update_lightmaps)
        .add_systems(Update, update_directional_light)
        .add_systems(Update, make_sphere_nonpickable)
        .add_systems(Update, update_radio_buttons)
        .add_systems(Update, handle_lighting_mode_change)
        .add_systems(Update, widgets::handle_ui_interactions::<LightingMode>)
        .add_systems(Update, reset_sphere_position)
        .add_systems(Update, move_sphere)
        .add_systems(Update, adjust_help_text)
        .run();
}

/// Creates the scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_status: Res<AppStatus>) {
    spawn_camera(&mut commands);
    spawn_scene(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
    spawn_help_text(&mut commands, &app_status);
}

/// Spawns the 3D camera.
fn spawn_camera(commands: &mut Commands) {
    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(-0.7, 0.7, 1.0).looking_at(vec3(0.0, 0.3, 0.0), Vec3::Y));
}

/// Spawns the scene.
///
/// The scene is loaded from a glTF file.
fn spawn_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn(SceneRoot(
            asset_server.load(
                GltfAssetLabel::Scene(0)
                    .from_asset("models/MixedLightingExample/MixedLightingExample.gltf"),
            ),
        ))
        .observe(
            |_: On<SceneInstanceReady>,
             mut lighting_mode_change_event_writer: EventWriter<LightingModeChanged>| {
                // When the scene loads, send a `LightingModeChanged` event so
                // that we set up the lightmaps.
                lighting_mode_change_event_writer.write(LightingModeChanged);
            },
        );
}

/// Spawns the buttons that allow the user to change the lighting mode.
fn spawn_buttons(commands: &mut Commands) {
    commands
        .spawn(widgets::main_ui_node())
        .with_children(|parent| {
            widgets::spawn_option_buttons(
                parent,
                "Lighting",
                &[
                    (LightingMode::Baked, "Baked"),
                    (LightingMode::MixedDirect, "Mixed (Direct)"),
                    (LightingMode::MixedIndirect, "Mixed (Indirect)"),
                    (LightingMode::RealTime, "Real-Time"),
                ],
            );
        });
}

/// Spawns the help text at the top of the window.
fn spawn_help_text(commands: &mut Commands, app_status: &AppStatus) {
    commands.spawn((
        create_help_text(app_status),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        HelpText,
    ));
}

/// Adds lightmaps to and/or removes lightmaps from objects in the scene when
/// the lighting mode changes.
///
/// This is also called right after the scene loads in order to set up the
/// lightmaps.
fn update_lightmaps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<(Entity, &GltfMeshName, &MeshMaterial3d<StandardMaterial>), With<Mesh3d>>,
    mut lighting_mode_change_event_reader: EventReader<LightingModeChanged>,
    app_status: Res<AppStatus>,
) {
    // Only run if the lighting mode changed. (Note that a change event is fired
    // when the scene first loads.)
    if lighting_mode_change_event_reader.read().next().is_none() {
        return;
    }

    // Select the lightmap to use, based on the lighting mode.
    let lightmap: Option<Handle<Image>> = match app_status.lighting_mode {
        LightingMode::Baked => {
            Some(asset_server.load("lightmaps/MixedLightingExample-Baked.zstd.ktx2"))
        }
        LightingMode::MixedDirect => {
            Some(asset_server.load("lightmaps/MixedLightingExample-MixedDirect.zstd.ktx2"))
        }
        LightingMode::MixedIndirect => {
            Some(asset_server.load("lightmaps/MixedLightingExample-MixedIndirect.zstd.ktx2"))
        }
        LightingMode::RealTime => None,
    };

    'outer: for (entity, name, material) in &meshes {
        // Add lightmaps to or remove lightmaps from the scenery objects in the
        // scene (all objects but the sphere).
        //
        // Note that doing a linear search through the `LIGHTMAPS` array is
        // inefficient, but we do it anyway in this example to improve clarity.
        for (lightmap_name, uv_rect) in LIGHTMAPS {
            if &**name != lightmap_name {
                continue;
            }

            // Lightmap exposure defaults to zero, so we need to set it.
            if let Some(ref mut material) = materials.get_mut(material) {
                material.lightmap_exposure = LIGHTMAP_EXPOSURE;
            }

            // Add or remove the lightmap.
            match lightmap {
                Some(ref lightmap) => {
                    commands.entity(entity).insert(Lightmap {
                        image: (*lightmap).clone(),
                        uv_rect,
                        bicubic_sampling: false,
                    });
                }
                None => {
                    commands.entity(entity).remove::<Lightmap>();
                }
            }
            continue 'outer;
        }

        // Add lightmaps to or remove lightmaps from the sphere.
        if &**name == "Sphere" {
            // Lightmap exposure defaults to zero, so we need to set it.
            if let Some(ref mut material) = materials.get_mut(material) {
                material.lightmap_exposure = LIGHTMAP_EXPOSURE;
            }

            // Add or remove the lightmap from the sphere. We only apply the
            // lightmap in fully-baked mode.
            match (&lightmap, app_status.lighting_mode) {
                (Some(lightmap), LightingMode::Baked) => {
                    commands.entity(entity).insert(Lightmap {
                        image: (*lightmap).clone(),
                        uv_rect: SPHERE_UV_RECT,
                        bicubic_sampling: false,
                    });
                }
                _ => {
                    commands.entity(entity).remove::<Lightmap>();
                }
            }
        }
    }
}

/// Converts a uv rectangle from the OpenGL coordinate system (origin in the
/// lower left) to the Vulkan coordinate system (origin in the upper left) that
/// Bevy uses.
///
/// For this particular example, the baking tool happened to use the OpenGL
/// coordinate system, so it was more convenient to do the conversion at compile
/// time than to pre-calculate and hard-code the values.
const fn uv_rect_opengl(gl_min: Vec2, size: Vec2) -> Rect {
    let min = vec2(gl_min.x, 1.0 - gl_min.y - size.y);
    Rect {
        min,
        max: vec2(min.x + size.x, min.y + size.y),
    }
}

/// Ensures that clicking on the scene to move the sphere doesn't result in a
/// hit on the sphere itself.
fn make_sphere_nonpickable(
    mut commands: Commands,
    mut query: Query<(Entity, &Name), (With<Mesh3d>, Without<Pickable>)>,
) {
    for (sphere, name) in &mut query {
        if &**name == "Sphere" {
            commands.entity(sphere).insert(Pickable::IGNORE);
        }
    }
}

/// Updates the directional light settings as necessary when the lighting mode
/// changes.
fn update_directional_light(
    mut lights: Query<&mut DirectionalLight>,
    mut lighting_mode_change_event_reader: EventReader<LightingModeChanged>,
    app_status: Res<AppStatus>,
) {
    // Only run if the lighting mode changed. (Note that a change event is fired
    // when the scene first loads.)
    if lighting_mode_change_event_reader.read().next().is_none() {
        return;
    }

    // Real-time direct light is used on the scenery if we're using mixed
    // indirect or real-time mode.
    let scenery_is_lit_in_real_time = matches!(
        app_status.lighting_mode,
        LightingMode::MixedIndirect | LightingMode::RealTime
    );

    for mut light in &mut lights {
        light.affects_lightmapped_mesh_diffuse = scenery_is_lit_in_real_time;
        // Don't bother enabling shadows if they won't show up on the scenery.
        light.shadows_enabled = scenery_is_lit_in_real_time;
    }
}

/// Updates the state of the selection widgets at the bottom of the window when
/// the lighting mode changes.
fn update_radio_buttons(
    mut widgets: Query<
        (
            Entity,
            Option<&mut BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<LightingMode>,
        ),
        Or<(With<RadioButton>, With<RadioButtonText>)>,
    >,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
) {
    for (entity, image, has_text, sender) in &mut widgets {
        let selected = **sender == app_status.lighting_mode;

        if let Some(mut bg_color) = image {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

/// Handles clicks on the widgets at the bottom of the screen and fires
/// [`LightingModeChanged`] events.
fn handle_lighting_mode_change(
    mut widget_click_event_reader: EventReader<WidgetClickEvent<LightingMode>>,
    mut lighting_mode_change_event_writer: EventWriter<LightingModeChanged>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in widget_click_event_reader.read() {
        app_status.lighting_mode = **event;
        lighting_mode_change_event_writer.write(LightingModeChanged);
    }
}

/// Moves the sphere to its original position when the user selects the baked
/// lighting mode.
///
/// As the light from the sphere is precomputed and depends on the sphere's
/// original position, the sphere must be placed there in order for the lighting
/// to be correct.
fn reset_sphere_position(
    mut objects: Query<(&Name, &mut Transform)>,
    mut lighting_mode_change_event_reader: EventReader<LightingModeChanged>,
    app_status: Res<AppStatus>,
) {
    // Only run if the lighting mode changed and if the lighting mode is
    // `LightingMode::Baked`. (Note that a change event is fired when the scene
    // first loads.)
    if lighting_mode_change_event_reader.read().next().is_none()
        || app_status.lighting_mode != LightingMode::Baked
    {
        return;
    }

    for (name, mut transform) in &mut objects {
        if &**name == "Sphere" {
            transform.translation = INITIAL_SPHERE_POSITION;
            break;
        }
    }
}

/// Updates the position of the sphere when the user clicks on a spot in the
/// scene.
///
/// Note that the position of the sphere is locked in baked lighting mode.
fn move_sphere(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    pointers: Query<&PointerInteraction>,
    mut meshes: Query<(&GltfMeshName, &ChildOf), With<Mesh3d>>,
    mut transforms: Query<&mut Transform>,
    app_status: Res<AppStatus>,
) {
    // Only run when the left button is clicked and we're not in baked lighting
    // mode.
    if app_status.lighting_mode == LightingMode::Baked
        || !mouse_button_input.pressed(MouseButton::Left)
    {
        return;
    }

    // Find the sphere.
    let Some(child_of) = meshes
        .iter_mut()
        .filter_map(|(name, child_of)| {
            if &**name == "Sphere" {
                Some(child_of)
            } else {
                None
            }
        })
        .next()
    else {
        return;
    };

    // Grab its transform.
    let Ok(mut transform) = transforms.get_mut(child_of.parent()) else {
        return;
    };

    // Set its transform to the appropriate position, as determined by the
    // picking subsystem.
    for interaction in pointers.iter() {
        if let Some(&(
            _,
            HitData {
                position: Some(position),
                ..
            },
        )) = interaction.get_nearest_hit()
        {
            transform.translation = position + vec3(0.0, SPHERE_OFFSET, 0.0);
        }
    }
}

/// Changes the help text at the top of the screen when the lighting mode
/// changes.
fn adjust_help_text(
    mut commands: Commands,
    help_texts: Query<Entity, With<HelpText>>,
    app_status: Res<AppStatus>,
    mut lighting_mode_change_event_reader: EventReader<LightingModeChanged>,
) {
    if lighting_mode_change_event_reader.read().next().is_none() {
        return;
    }

    for help_text in &help_texts {
        commands
            .entity(help_text)
            .insert(create_help_text(&app_status));
    }
}

/// Returns appropriate text to display at the top of the screen.
fn create_help_text(app_status: &AppStatus) -> Text {
    match app_status.lighting_mode {
        LightingMode::Baked => Text::new(
            "Scenery: Static, baked direct light, baked indirect light
Sphere: Static, baked direct light, baked indirect light",
        ),
        LightingMode::MixedDirect => Text::new(
            "Scenery: Static, baked direct light, baked indirect light
Sphere: Dynamic, real-time direct light, no indirect light
Click in the scene to move the sphere",
        ),
        LightingMode::MixedIndirect => Text::new(
            "Scenery: Static, real-time direct light, baked indirect light
Sphere: Dynamic, real-time direct light, no indirect light
Click in the scene to move the sphere",
        ),
        LightingMode::RealTime => Text::new(
            "Scenery: Dynamic, real-time direct light, no indirect light
Sphere: Dynamic, real-time direct light, no indirect light
Click in the scene to move the sphere",
        ),
    }
}
