//! A glTF scene viewer plugin.  Provides controls for directional lighting, and switching between scene cameras.
//! To use in your own application:
//! - Copy the code for the `SceneViewerPlugin` and add the plugin to your App.
//! - Insert an initialized `SceneHandle` resource into your App's `AssetServer`.

use bevy::{
    asset::LoadState, gltf::Gltf, input::common_conditions::input_just_pressed, prelude::*,
    scene::InstanceId,
};

use std::f32::consts::*;
use std::fmt;

use super::camera_controller::*;

#[derive(Resource)]
pub struct SceneHandle {
    pub gltf_handle: Handle<Gltf>,
    scene_index: usize,
    instance_id: Option<InstanceId>,
    pub is_loaded: bool,
    pub has_light: bool,
}

impl SceneHandle {
    pub fn new(gltf_handle: Handle<Gltf>, scene_index: usize) -> Self {
        Self {
            gltf_handle,
            scene_index,
            instance_id: None,
            is_loaded: false,
            has_light: false,
        }
    }
}

#[cfg(not(feature = "animation"))]
const INSTRUCTIONS: &str = r#"
Scene Controls:
    L           - animate light direction
    U           - toggle shadows
    C           - cycle through the camera controller and any cameras loaded from the scene

    compile with "--features animation" for animation controls.
"#;

#[cfg(feature = "animation")]
const INSTRUCTIONS: &str = "
Scene Controls:
    L           - animate light direction
    U           - toggle shadows
    B           - toggle bounding boxes
    C           - cycle through the camera controller and any cameras loaded from the scene

    Space       - Play/Pause animation
    Enter       - Cycle through animations
";

impl fmt::Display for SceneHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{INSTRUCTIONS}")
    }
}

pub struct SceneViewerPlugin;

impl Plugin for SceneViewerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraTracker>()
            .add_systems(PreUpdate, scene_load_check)
            .add_systems(
                Update,
                (
                    update_lights,
                    camera_tracker,
                    toggle_bounding_boxes.run_if(input_just_pressed(KeyCode::KeyB)),
                ),
            );
    }
}

fn toggle_bounding_boxes(mut config: ResMut<GizmoConfigStore>) {
    config.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
}

fn scene_load_check(
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Assets<Scene>>,
    gltf_assets: Res<Assets<Gltf>>,
    mut scene_handle: ResMut<SceneHandle>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    match scene_handle.instance_id {
        None => {
            if asset_server.load_state(&scene_handle.gltf_handle) == LoadState::Loaded {
                let gltf = gltf_assets.get(&scene_handle.gltf_handle).unwrap();
                if gltf.scenes.len() > 1 {
                    info!(
                        "Displaying scene {} out of {}",
                        scene_handle.scene_index,
                        gltf.scenes.len()
                    );
                    info!("You can select the scene by adding '#Scene' followed by a number to the end of the file path (e.g '#Scene1' to load the second scene).");
                }

                let gltf_scene_handle =
                    gltf.scenes
                        .get(scene_handle.scene_index)
                        .unwrap_or_else(|| {
                            panic!(
                                "glTF file doesn't contain scene {}!",
                                scene_handle.scene_index
                            )
                        });
                let scene = scenes.get_mut(gltf_scene_handle).unwrap();

                let mut query = scene
                    .world
                    .query::<(Option<&DirectionalLight>, Option<&PointLight>)>();
                scene_handle.has_light =
                    query
                        .iter(&scene.world)
                        .any(|(maybe_directional_light, maybe_point_light)| {
                            maybe_directional_light.is_some() || maybe_point_light.is_some()
                        });

                scene_handle.instance_id =
                    Some(scene_spawner.spawn(gltf_scene_handle.clone_weak()));

                info!("Spawning scene...");
            }
        }
        Some(instance_id) if !scene_handle.is_loaded => {
            if scene_spawner.instance_is_ready(instance_id) {
                info!("...done!");
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
        }
    }

    if key_input.just_pressed(KeyCode::KeyL) {
        *animate_directional_light = !*animate_directional_light;
    }
    if *animate_directional_light {
        for (mut transform, _) in &mut query {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                time.elapsed_seconds() * PI / 15.0,
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
    }
}
