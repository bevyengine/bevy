//! An FBX scene viewer plugin. Provides controls for directional lighting, material inspection, and scene navigation.
//! To use in your own application:
//! - Copy the code for the `FbxViewerPlugin` and add the plugin to your App.
//! - Insert an initialized `FbxSceneHandle` resource into your App's `AssetServer`.

use bevy::{fbx::Fbx, input::common_conditions::input_just_pressed, prelude::*, scene::InstanceId};

use std::{f32::consts::*, fmt};

use super::camera_controller::*;

#[derive(Resource)]
pub struct FbxSceneHandle {
    pub fbx_handle: Handle<Fbx>,
    instance_id: Option<InstanceId>,
    pub is_loaded: bool,
    pub has_light: bool,
}

impl FbxSceneHandle {
    pub fn new(fbx_handle: Handle<Fbx>) -> Self {
        Self {
            fbx_handle,
            instance_id: None,
            is_loaded: false,
            has_light: false,
        }
    }
}

const INSTRUCTIONS: &str = r#"
FBX Scene Controls:
    L           - animate light direction
    U           - toggle shadows
    B           - toggle bounding boxes
    C           - cycle through the camera controller and any cameras loaded from the scene
    M           - toggle material debug info
    I           - print FBX asset information
"#;

impl fmt::Display for FbxSceneHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{INSTRUCTIONS}")
    }
}

pub struct FbxViewerPlugin;

impl Plugin for FbxViewerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraTracker>()
            .init_resource::<MaterialDebugInfo>()
            .add_systems(PreUpdate, fbx_load_check)
            .add_systems(
                Update,
                (
                    update_lights,
                    camera_tracker,
                    toggle_bounding_boxes.run_if(input_just_pressed(KeyCode::KeyB)),
                    toggle_material_debug.run_if(input_just_pressed(KeyCode::KeyM)),
                    print_fbx_info.run_if(input_just_pressed(KeyCode::KeyI)),
                ),
            );
    }
}

#[derive(Resource, Default)]
struct MaterialDebugInfo {
    enabled: bool,
}

fn toggle_bounding_boxes(mut config: ResMut<GizmoConfigStore>) {
    config.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
}

fn toggle_material_debug(mut debug_info: ResMut<MaterialDebugInfo>) {
    debug_info.enabled = !debug_info.enabled;
    if debug_info.enabled {
        info!("Material debug info enabled - press M again to disable");
    } else {
        info!("Material debug info disabled");
    }
}

fn print_fbx_info(
    fbx_assets: Res<Assets<Fbx>>,
    scene_handle: Res<FbxSceneHandle>,
    materials: Query<&MeshMaterial3d<StandardMaterial>>,
    meshes: Query<&Mesh3d>,
    standard_materials: Res<Assets<StandardMaterial>>,
) {
    if let Some(fbx) = fbx_assets.get(&scene_handle.fbx_handle) {
        info!("=== FBX Asset Information ===");
        info!("Meshes: {}", fbx.meshes.len());
        info!("Materials: {}", fbx.materials.len());
        info!("Nodes: {}", fbx.nodes.len());
        info!("Skins: {}", fbx.skins.len());
        info!("Animation clips: {}", fbx.animations.len());

        // Print material information
        info!("=== Material Details ===");
        for (i, material_handle) in fbx.materials.iter().enumerate() {
            if let Some(material) = standard_materials.get(material_handle) {
                info!(
                    "Material {}: base_color={:?}, metallic={}, roughness={}",
                    i, material.base_color, material.metallic, material.perceptual_roughness
                );

                if material.base_color_texture.is_some() {
                    info!("  - Has base color texture");
                }
                if material.normal_map_texture.is_some() {
                    info!("  - Has normal map");
                }
                if material.metallic_roughness_texture.is_some() {
                    info!("  - Has metallic/roughness texture");
                }
                if material.emissive_texture.is_some() {
                    info!("  - Has emissive texture");
                }
                if material.occlusion_texture.is_some() {
                    info!("  - Has occlusion texture");
                }
            }
        }

        info!("=== Scene Statistics ===");
        info!("Total mesh entities: {}", meshes.iter().count());
        info!("Total material entities: {}", materials.iter().count());
    } else {
        info!("FBX asset not yet loaded");
    }
}

fn fbx_load_check(
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Assets<Scene>>,
    fbx_assets: Res<Assets<Fbx>>,
    mut scene_handle: ResMut<FbxSceneHandle>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    match scene_handle.instance_id {
        None => {
            if asset_server
                .load_state(&scene_handle.fbx_handle)
                .is_loaded()
            {
                let fbx = fbx_assets.get(&scene_handle.fbx_handle).unwrap();

                info!("FBX loaded successfully!");
                info!(
                    "Found {} meshes, {} materials, {} nodes",
                    fbx.meshes.len(),
                    fbx.materials.len(),
                    fbx.nodes.len()
                );

                // Check if the FBX scene has lights
                if let Some(scene_handle_ref) = fbx.scenes.first() {
                    let scene = scenes.get_mut(scene_handle_ref).unwrap();
                    let mut query = scene
                        .world
                        .query::<(Option<&DirectionalLight>, Option<&PointLight>)>();
                    scene_handle.has_light = query.iter(&scene.world).any(
                        |(maybe_directional_light, maybe_point_light)| {
                            maybe_directional_light.is_some() || maybe_point_light.is_some()
                        },
                    );

                    scene_handle.instance_id =
                        Some(scene_spawner.spawn(scene_handle_ref.clone_weak()));

                    info!("Spawning FBX scene...");
                } else {
                    warn!("FBX file contains no scenes!");
                }
            }
        }
        Some(instance_id) if !scene_handle.is_loaded => {
            if scene_spawner.instance_is_ready(instance_id) {
                info!("FBX scene loaded and ready!");
                scene_handle.is_loaded = true;
            }
        }
        Some(_) => {}
    }
}

fn update_lights(
    key_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut DirectionalLight)>,
    mut animate_directional_light: Local<bool>,
) {
    for (_, mut light) in &mut query {
        if key_input.just_pressed(KeyCode::KeyU) {
            light.shadows_enabled = !light.shadows_enabled;
            info!(
                "Shadows {}",
                if light.shadows_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }

    if key_input.just_pressed(KeyCode::KeyL) {
        *animate_directional_light = !*animate_directional_light;
        info!(
            "Light animation {}",
            if *animate_directional_light {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
    if *animate_directional_light {
        for (mut transform, _) in &mut query {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                time.elapsed_secs() * PI / 15.0,
                -FRAC_PI_4,
            );
        }
    }
}

#[derive(Resource, Default)]
struct CameraTracker {
    active_index: Option<usize>,
    cameras: Vec<Entity>,
}

impl CameraTracker {
    fn track_camera(&mut self, entity: Entity) -> bool {
        self.cameras.push(entity);
        if self.active_index.is_none() {
            self.active_index = Some(self.cameras.len() - 1);
            true
        } else {
            false
        }
    }

    fn active_camera(&self) -> Option<Entity> {
        self.active_index.map(|i| self.cameras[i])
    }

    fn set_next_active(&mut self) -> Option<Entity> {
        let active_index = self.active_index?;
        let new_i = (active_index + 1) % self.cameras.len();
        self.active_index = Some(new_i);
        Some(self.cameras[new_i])
    }
}

fn camera_tracker(
    mut camera_tracker: ResMut<CameraTracker>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut queries: ParamSet<(
        Query<(Entity, &mut Camera), (Added<Camera>, Without<CameraController>)>,
        Query<(Entity, &mut Camera), (Added<Camera>, With<CameraController>)>,
        Query<&mut Camera>,
    )>,
) {
    // track added scene camera entities first, to ensure they are preferred for the
    // default active camera
    for (entity, mut camera) in queries.p0().iter_mut() {
        camera.is_active = camera_tracker.track_camera(entity);
    }

    // iterate added custom camera entities second
    for (entity, mut camera) in queries.p1().iter_mut() {
        camera.is_active = camera_tracker.track_camera(entity);
    }

    if keyboard_input.just_pressed(KeyCode::KeyC) {
        // disable currently active camera
        if let Some(e) = camera_tracker.active_camera() {
            if let Ok(mut camera) = queries.p2().get_mut(e) {
                camera.is_active = false;
            }
        }

        // enable next active camera
        if let Some(e) = camera_tracker.set_next_active() {
            if let Ok(mut camera) = queries.p2().get_mut(e) {
                camera.is_active = true;
            }
        }
        info!(
            "Switched to camera {}",
            camera_tracker.active_index.unwrap_or(0)
        );
    }
}
