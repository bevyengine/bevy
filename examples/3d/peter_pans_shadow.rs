//! This example demonstrates "Peter Pan's Shadow" effect by using `OnlyShadowCaster` and `NotShadowCaster` components.
//! It shows how to create a visible entity that does not cast shadows and an invisible entity that only casts shadows,
//! allowing independent control over the visible model and its shadow.
//!
//! Press 'S' to toggle between the shadow being cast by the visible fox or the invisible fox.
//! Showcases how to use shadow casting components combined with render layers to achieve complex shadow effects.
use bevy::animation::AnimationClip;
use bevy::camera::visibility::RenderLayers;
use bevy::gltf::Gltf;
use bevy::gltf::GltfAssetLabel;
use bevy::light::{DirectionalLightShadowMap, NotShadowCaster, OnlyShadowCaster};
use bevy::math::ops::{cos, sin};
use bevy::prelude::*;
use bevy::scene::{SceneInstanceReady, SceneRoot};

const MAIN_CAMERA_LAYER: usize = 0;
const SHADOW_ONLY_LAYER: usize = 1;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(ShadowToggle {
            body_casts_shadow: true,
        })
        .init_resource::<PeterPanEntities>()
        .init_resource::<AnimationOverrideState>()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_systems(Startup, setup)
        .add_systems(Update, move_peter_pan)
        .add_systems(Update, try_apply_named_animations)
        .add_systems(Update, toggle_shadows_on_key_s)
        .run();
}

/// Create an `AnimationToPlay` component from an animation clip handle.
fn create_animation(
    clip_handle: Handle<AnimationClip>,
    graphs: &mut ResMut<Assets<AnimationGraph>>,
) -> AnimationToPlay {
    let (graph, index) = AnimationGraph::from_clip(clip_handle);
    let graph_handle = graphs.add(graph);
    AnimationToPlay {
        graph_handle,
        index,
    }
}

#[derive(Component)]
struct PeterPanBody;

#[derive(Component)]
struct PeterPanShadow;

#[derive(Resource)]
struct ShadowToggle {
    /// If true, the visible body casts the shadow. If false, the hidden surrogate does.
    body_casts_shadow: bool,
}

#[derive(Component, Clone)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // The 3D camera can see both the main and shadow render layers
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.5, 6.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
        RenderLayers::from_layers(&[MAIN_CAMERA_LAYER, SHADOW_ONLY_LAYER]),
    ));

    // UI: A text label for the "S" key in the top-left corner.
    // The 2D camera order is set to 1 so that UI renders on top
    // of the 3D scene that is rendered with order 0.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..Default::default()
        },
    ));
    let font_handle: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn((
        Text::new("Press S to toggle shadows"),
        TextFont {
            font: font_handle,
            font_size: 20.0,
            ..Default::default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Px(8.0),
            ..Default::default()
        },
    ));

    // Directional light can see both main and shadow layers for rendering
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, -0.5, 0.0)),
        RenderLayers::from_layers(&[MAIN_CAMERA_LAYER, SHADOW_ONLY_LAYER]),
    ));

    // Floor plane to receive shadows
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 0.8),
            ..default()
        })),
        RenderLayers::layer(MAIN_CAMERA_LAYER),
    ));

    // Load the Fox model and animations
    // NOTE: Asset paths are relative to the `assets/` directory
    let gltf_path: &str = "models/animated/Fox.glb";
    let glb_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(gltf_path));
    let gltf_asset = asset_server.load(gltf_path);
    let glb_survey_anim = asset_server.load(GltfAssetLabel::Animation(0).from_asset(gltf_path));
    let glb_run_anim = asset_server.load(GltfAssetLabel::Animation(2).from_asset(gltf_path));
    let glb_scene_scale = 0.025;

    // Create animation graphs for body and shadow
    let survey_animation = create_animation(glb_survey_anim, &mut graphs);
    let run_animation = create_animation(glb_run_anim, &mut graphs);

    // Spawn the visible body with "survey" animation
    let body_entity = commands
        .spawn((
            SceneRoot(glb_scene.clone()),
            Transform::from_scale(Vec3::splat(glb_scene_scale)),
            PeterPanBody,
            survey_animation,
            RenderLayers::layer(MAIN_CAMERA_LAYER),
        ))
        .observe(play_peter_pan_when_ready)
        .id();

    // Spawn the shadow-only instance with "run" animation
    let shadow_entity = commands
        .spawn((
            SceneRoot(glb_scene),
            Transform::from_scale(Vec3::splat(glb_scene_scale)),
            PeterPanShadow,
            run_animation,
            RenderLayers::layer(SHADOW_ONLY_LAYER),
        ))
        .observe(play_peter_pan_when_ready)
        .id();

    // Store entity IDs for later animation patching if named animations are discovered
    commands.insert_resource(PeterPanEntities {
        body: Some(body_entity),
        shadow: Some(shadow_entity),
        gltf_handle: gltf_asset,
    });
}

/// Triggered when a scene instance is spawned; this will find `AnimationPlayer`
/// components in the scene and start the requested animation, and also apply
/// shadow-related components (`NotShadowCaster` / `OnlyShadowCaster`) to mesh
/// descendants depending on whether the root is a `PeterPanBody` or `PeterPanShadow`.
///
fn play_peter_pan_when_ready(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    animation_query: Query<&AnimationToPlay>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    body_query: Query<&PeterPanBody>,
    shadow_query: Query<&PeterPanShadow>,
    shadow_toggle: Res<ShadowToggle>,
) {
    // Optionally get the AnimationToPlay; it's not required to apply the shadow components
    let anim = animation_query.get(scene_ready.entity).ok();

    for child in children.iter_descendants(scene_ready.entity) {
        // Start animations on any AnimationPlayer only if AnimationToPlay exists
        if let Some(animation_to_play) = anim
            && let Ok(mut player) = players.get_mut(child)
        {
            player.play(animation_to_play.index).repeat();
            // Connect the animation graph to the player
            commands
                .entity(child)
                .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
        }

        if shadow_query.get(scene_ready.entity).is_ok() {
            // For shadow root, ensure descendants are hidden from camera
            commands
                .entity(child)
                .insert((Visibility::Hidden, RenderLayers::layer(SHADOW_ONLY_LAYER)));
        }
    }

    // Apply the currently-selected shadow mode to descendants. We apply the components here
    // because the scene may be spawned before the initial toggle command otherwise.
    if body_query.get(scene_ready.entity).is_ok() {
        apply_shadow_components(
            &mut commands,
            scene_ready.entity,
            &children,
            shadow_toggle.body_casts_shadow,
        );
    }
    if shadow_query.get(scene_ready.entity).is_ok() {
        apply_shadow_components(
            &mut commands,
            scene_ready.entity,
            &children,
            !shadow_toggle.body_casts_shadow,
        );
    }
}

// Animate `Peter Pan` and his shadow independently
fn move_peter_pan(
    time: Res<Time>,
    mut shadow_query: Query<&mut Transform, (With<PeterPanShadow>, Without<PeterPanBody>)>,
) {
    let t = time.elapsed_secs();

    // Visible body stays stationary
    // Move Shadow: Follows x/z but stays on the ground plane
    if let Some(mut shadow_transform) = shadow_query.iter_mut().next() {
        // Offset phase for a lazy animation
        let shadow_t = t - 0.5;
        shadow_transform.translation.x = -sin(shadow_t) * 2.0;
        shadow_transform.translation.z = cos(shadow_t) * 2.0;
    }
}

fn toggle_shadows_on_key_s(
    // If you press `S` before the GLTF scene finishes spawning there may be no mesh
    // descendants to update yet; So we store the command until everything is ready.
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    body_entities: Query<Entity, With<PeterPanBody>>,
    shadow_entities: Query<Entity, With<PeterPanShadow>>,
    children: Query<&Children>,
    mut shadow_toggle: ResMut<ShadowToggle>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyS) {
        shadow_toggle.body_casts_shadow = !shadow_toggle.body_casts_shadow;

        // Swap shadow casting between body and shadow entities
        for body_entity in body_entities.iter() {
            apply_shadow_components(
                &mut commands,
                body_entity,
                &children,
                shadow_toggle.body_casts_shadow,
            );
        }
        for shadow_entity in shadow_entities.iter() {
            apply_shadow_components(
                &mut commands,
                shadow_entity,
                &children,
                !shadow_toggle.body_casts_shadow,
            );
        }
    }
}

/// Apply shadow components to all descendants based on whether this entity should cast shadows.
/// If `cast_shadow` is true, add `OnlyShadowCaster` and remove `NotShadowCaster`.
/// If `cast_shadow` is false, add `NotShadowCaster` and remove `OnlyShadowCaster`.
fn apply_shadow_components(
    commands: &mut Commands,
    root: Entity,
    children: &Query<&Children>,
    cast_shadow: bool,
) {
    for child in children.iter_descendants(root) {
        if cast_shadow {
            commands
                .entity(child)
                .insert(OnlyShadowCaster)
                .remove::<NotShadowCaster>();
        } else {
            commands
                .entity(child)
                .insert(NotShadowCaster)
                .remove::<OnlyShadowCaster>();
        }
    }
}

#[derive(Resource, Default)]
struct PeterPanEntities {
    body: Option<Entity>,
    shadow: Option<Entity>,
    gltf_handle: Handle<Gltf>,
}

#[derive(Resource, Default)]
struct AnimationOverrideState {
    applied: bool,
}

// We know that the GLB has named animations "Survey" and "Run", so we can try to apply them here.
fn try_apply_named_animations(
    mut commands: Commands,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    entities: Res<PeterPanEntities>,
    mut override_state: ResMut<AnimationOverrideState>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if override_state.applied {
        return;
    }
    let gltf_handle = &entities.gltf_handle;
    if let Some(gltf) = gltfs.get(gltf_handle) {
        // Log any named animations discovered in the GLTF
        if gltf.named_animations.is_empty() {
            info!("No named animations found in GLTF");
        } else {
            let names: Vec<&str> = gltf.named_animations.keys().map(AsRef::as_ref).collect();
            info!("Discovered named animations: {:?}", names);
        }

        // Try to find named animations "survey" / "Survey"
        if let Some(body_entity) = entities.body
            && let Some(anim_handle) = gltf
                .named_animations
                .get("Survey")
                .or_else(|| gltf.named_animations.get("survey"))
        {
            let anim_clip: Handle<AnimationClip> = anim_handle.clone();
            let (graph, idx) = AnimationGraph::from_clip(anim_clip);
            let graph_handle = graphs.add(graph);
            commands.entity(body_entity).insert(AnimationToPlay {
                graph_handle: graph_handle.clone(),
                index: idx,
            });
            // Start the anim on any existing players in case the scene already spawned
            for child in children.iter_descendants(body_entity) {
                if let Ok(mut player) = players.get_mut(child) {
                    player.play(idx).repeat();
                    commands
                        .entity(child)
                        .insert(AnimationGraphHandle(graph_handle.clone()));
                }
            }
            info!("Applied named 'Survey' animation to body. index: {:?}", idx);
        }
        // Run for shadow
        if let Some(shadow_entity) = entities.shadow
            && let Some(anim_handle) = gltf
                .named_animations
                .get("Run")
                .or_else(|| gltf.named_animations.get("run"))
        {
            let anim_clip: Handle<AnimationClip> = anim_handle.clone();
            let (graph, idx) = AnimationGraph::from_clip(anim_clip);
            let graph_handle = graphs.add(graph);
            commands.entity(shadow_entity).insert(AnimationToPlay {
                graph_handle: graph_handle.clone(),
                index: idx,
            });
            for child in children.iter_descendants(shadow_entity) {
                if let Ok(mut player) = players.get_mut(child) {
                    player.play(idx).repeat();
                    commands
                        .entity(child)
                        .insert(AnimationGraphHandle(graph_handle.clone()));
                }
            }
            info!("Applied named 'Run' animation to shadow. index: {:?}", idx);
        }
        override_state.applied = true;
    }
}
