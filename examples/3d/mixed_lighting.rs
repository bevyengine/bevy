//! Demonstrates how to combine baked and dynamic lighting.

use bevy::{
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

/// The application settings.
#[derive(Clone, Default, Resource)]
struct AppStatus {
    /// The lighting mode that the user currently has set: baked, mixed, or
    /// real-time.
    lighting_mode: LightingMode,
}

/// The type of lighting mode.
#[derive(Clone, Copy, PartialEq, Default)]
enum LightingMode {
    /// All light is computed ahead of time; no lighting takes place at runtime.
    ///
    /// In this mode, the sphere can't be moved, as the light shining on it was
    /// precomputed. On the plus side, the sphere has indirect lighting in this
    /// mode, as the red hue on the bottom of the sphere demonstrates.
    Baked,

    /// Light for the static objects is computed ahead of time, but the light
    /// for the dynamic sphere is computed at runtime.
    ///
    /// In this mode, the sphere can be moved, and the light will be computed
    /// for it as you do so. The sphere loses indirect illumination; notice the
    /// lack of a red hue at the base of the sphere. However, the rest of the
    /// scene has indirect illumination. Note also that the sphere doesn't cast
    /// shadows on the static objects in this mode, because shadows are part of
    /// the lighting computation.
    #[default]
    Mixed,

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
#[derive(Clone, Copy, Default, Event)]
struct LightingModeChanged;

/// The name of every static object in the scene that has a lightmap, as well as
/// the UV rect of its lightmap.
static LIGHTMAPS: [(&str, Rect); 5] = [
    (
        "Plane",
        uv_rect_opengl(Vec2::splat(0.026), Vec2::splat(0.710)),
    ),
    (
        "SheenChair_fabric",
        uv_rect_opengl(vec2(0.786, 0.024), Vec2::splat(0.191)),
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
            affects_lightmapped_meshes: false,
        })
        .init_resource::<AppStatus>()
        .add_event::<WidgetClickEvent<LightingMode>>()
        .add_event::<LightingModeChanged>()
        .add_systems(Startup, setup)
        .add_systems(Update, add_lightmaps_to_meshes)
        .add_systems(Update, initialize_lights)
        .add_systems(Update, make_sphere_nonpickable)
        .add_systems(Update, update_radio_buttons)
        .add_systems(Update, update_shadows)
        .add_systems(Update, handle_lighting_mode_change)
        .add_systems(Update, widgets::handle_ui_interactions::<LightingMode>)
        .add_systems(Update, move_sphere)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(-0.7, 0.7, 1.0).looking_at(vec3(0.0, 0.3, 0.0), Vec3::Y));

    commands
        .spawn(SceneRoot(
            asset_server.load(
                GltfAssetLabel::Scene(0)
                    .from_asset("models/MixedLightingExample/MixedLightingExample.gltf"),
            ),
        ))
        .observe(on_scene_loaded);

    commands
        .spawn(widgets::main_ui_node())
        .with_children(|parent| {
            widgets::spawn_option_buttons(
                parent,
                "Lighting",
                &[
                    (LightingMode::Baked, "Baked"),
                    (LightingMode::Mixed, "Mixed"),
                    (LightingMode::RealTime, "Real-Time"),
                ],
            );
        });
}

fn add_lightmaps_to_meshes(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<(Entity, &Name, &MeshMaterial3d<StandardMaterial>), With<Mesh3d>>,
    mut events: EventReader<LightingModeChanged>,
    app_status: Res<AppStatus>,
) {
    if events.read().next().is_none() {
        return;
    }

    'outer: for (entity, name, material) in &meshes {
        for (lightmap_name, uv_rect) in LIGHTMAPS {
            if &**name != lightmap_name {
                continue;
            }

            if let Some(ref mut material) = materials.get_mut(material) {
                material.lightmap_exposure = LIGHTMAP_EXPOSURE;
            }

            if app_status.lighting_mode == LightingMode::RealTime {
                commands.entity(entity).remove::<Lightmap>();
            } else {
                commands.entity(entity).insert(Lightmap {
                    image: asset_server.load("lightmaps/MixedLightingExample.zstd.ktx2"),
                    uv_rect,
                });
            }

            continue 'outer;
        }

        if &**name == "Sphere" {
            if let Some(ref mut material) = materials.get_mut(material) {
                material.lightmap_exposure = LIGHTMAP_EXPOSURE;
            }

            if app_status.lighting_mode == LightingMode::Baked {
                commands.entity(entity).insert(Lightmap {
                    image: asset_server.load("lightmaps/MixedLightingExample.zstd.ktx2"),
                    uv_rect: uv_rect_opengl(vec2(0.788, 0.484), Vec2::splat(0.062)),
                });
            } else {
                commands.entity(entity).remove::<Lightmap>();
            }
        }
    }
}

fn initialize_lights(mut lights: Query<&mut DirectionalLight>) {
    for mut light in &mut lights {
        // Do this check to avoid incurring change events on every frame.
        if light.affects_lightmapped_meshes {
            light.affects_lightmapped_meshes = false;
        }
    }
}

const fn uv_rect_opengl(gl_min: Vec2, size: Vec2) -> Rect {
    let min = vec2(gl_min.x, 1.0 - gl_min.y - size.y);
    Rect {
        min,
        max: vec2(min.x + size.x, min.y + size.y),
    }
}

fn make_sphere_nonpickable(
    mut commands: Commands,
    mut query: Query<(Entity, &Name), (With<Mesh3d>, Without<PickingBehavior>)>,
) {
    for (sphere, name) in &mut query {
        if &**name == "Sphere" {
            commands.entity(sphere).insert(PickingBehavior::IGNORE);
        }
    }
}

fn update_shadows(
    mut lights: Query<&mut DirectionalLight>,
    mut events: EventReader<LightingModeChanged>,
    app_status: Res<AppStatus>,
) {
    if events.read().next().is_none() {
        return;
    }

    for mut light in &mut lights {
        light.shadows_enabled = app_status.lighting_mode == LightingMode::RealTime;
    }
}

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

fn handle_lighting_mode_change(
    mut widget_click_events: EventReader<WidgetClickEvent<LightingMode>>,
    mut lighting_mode_change_events: EventWriter<LightingModeChanged>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in widget_click_events.read() {
        app_status.lighting_mode = **event;
        lighting_mode_change_events.send(LightingModeChanged);
    }
}

fn move_sphere(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    pointers: Query<&PointerInteraction>,
    mut meshes: Query<(&Name, &Parent), With<Mesh3d>>,
    mut transforms: Query<&mut Transform>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }

    let Some(parent) = meshes
        .iter_mut()
        .filter_map(|(name, parent)| {
            if &**name == "Sphere" {
                Some(parent)
            } else {
                None
            }
        })
        .next()
    else {
        return;
    };

    let Ok(mut transform) = transforms.get_mut(**parent) else {
        return;
    };

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

fn on_scene_loaded(
    _: Trigger<SceneInstanceReady>,
    mut lighting_mode_change_event_writer: EventWriter<LightingModeChanged>,
) {
    lighting_mode_change_event_writer.send(LightingModeChanged);
}
