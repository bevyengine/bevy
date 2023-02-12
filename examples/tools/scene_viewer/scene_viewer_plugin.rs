//! A glTF scene viewer plugin.  Provides controls for animation, directional lighting, and switching between scene cameras.
//! To use in your own application:
//! - Copy the code for the `SceneViewerPlugin` and add the plugin to your App.
//! - Insert an initialized `SceneHandle` resource into your App's `AssetServer`.

use bevy::{asset::LoadState, gltf::Gltf, prelude::*, scene::InstanceId};

use std::f32::consts::*;
use std::fmt;

use super::camera_controller_plugin::*;

#[derive(Resource)]
pub struct SceneHandle {
    gltf_handle: Handle<Gltf>,
    scene_index: usize,
    #[cfg(feature = "animation")]
    animations: Vec<Handle<AnimationClip>>,
    instance_id: Option<InstanceId>,
    pub is_loaded: bool,
    pub has_light: bool,
}

impl SceneHandle {
    pub fn new(gltf_handle: Handle<Gltf>, scene_index: usize) -> Self {
        Self {
            gltf_handle,
            scene_index,
            #[cfg(feature = "animation")]
            animations: Vec::new(),
            instance_id: None,
            is_loaded: false,
            has_light: false,
        }
    }
}

impl fmt::Display for SceneHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Scene Controls:
    L           - animate light direction
    U           - toggle shadows
    C           - cycle through the camera controller and any cameras loaded from the scene

    Space       - Play/Pause animation
    Enter       - Cycle through animations
"
        )
    }
}

pub struct SceneViewerPlugin;

impl Plugin for SceneViewerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraTracker>()
            .add_system(scene_load_check.in_base_set(CoreSet::PreUpdate))
            .add_system(update_lights)
            .add_system(camera_tracker);

        #[cfg(feature = "animation")]
        app.add_system(start_animation)
            .add_system(keyboard_animation_control);
    }
}

fn scene_load_check(
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Assets<Scene>>,
    gltf_assets: ResMut<Assets<Gltf>>,
    mut scene_handle: ResMut<SceneHandle>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    match scene_handle.instance_id {
        None => {
            if asset_server.get_load_state(&scene_handle.gltf_handle) == LoadState::Loaded {
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

                #[cfg(feature = "animation")]
                {
                    scene_handle.animations = gltf.animations.clone();
                    if !scene_handle.animations.is_empty() {
                        info!(
                            "Found {} animation{}",
                            scene_handle.animations.len(),
                            if scene_handle.animations.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        );
                    }
                }

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

#[cfg(feature = "animation")]
fn start_animation(
    mut player: Query<&mut AnimationPlayer>,
    mut done: Local<bool>,
    scene_handle: Res<SceneHandle>,
) {
    if !*done {
        if let Ok(mut player) = player.get_single_mut() {
            if let Some(animation) = scene_handle.animations.first() {
                player.play(animation.clone_weak()).repeat();
                *done = true;
            }
        }
    }
}

#[cfg(feature = "animation")]
fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    mut animation_player: Query<&mut AnimationPlayer>,
    scene_handle: Res<SceneHandle>,
    mut current_animation: Local<usize>,
    mut changing: Local<bool>,
) {
    if scene_handle.animations.is_empty() {
        return;
    }

    if let Ok(mut player) = animation_player.get_single_mut() {
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.is_paused() {
                player.resume();
            } else {
                player.pause();
            }
        }

        if *changing {
            // change the animation the frame after return was pressed
            *current_animation = (*current_animation + 1) % scene_handle.animations.len();
            player
                .play(scene_handle.animations[*current_animation].clone_weak())
                .repeat();
            *changing = false;
        }

        if keyboard_input.just_pressed(KeyCode::Return) {
            // delay the animation change for one frame
            *changing = true;
            // set the current animation to its start and pause it to reset to its starting state
            player.set_elapsed(0.0).pause();
        }
    }
}

fn update_lights(
    key_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut DirectionalLight)>,
    mut animate_directional_light: Local<bool>,
) {
    for (_, mut light) in &mut query {
        if key_input.just_pressed(KeyCode::U) {
            light.shadows_enabled = !light.shadows_enabled;
        }
    }

    if key_input.just_pressed(KeyCode::L) {
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
    keyboard_input: Res<Input<KeyCode>>,
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

    if keyboard_input.just_pressed(KeyCode::C) {
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
