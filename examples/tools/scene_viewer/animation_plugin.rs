//! Control animations of entities in the loaded scene.
use bevy::{asset::HandleId, gltf::Gltf, prelude::*};

use crate::scene_viewer_plugin::SceneHandle;

struct Clip {
    name: Option<String>,
    handle: Handle<AnimationClip>,
}
/// Controls animation clips for a unique entity.
#[derive(Resource)]
struct Clips {
    clips: Vec<Clip>,
    current: usize,
}
impl Clips {
    fn new(clips: Vec<Clip>) -> Self {
        Clips { clips, current: 0 }
    }
    fn current(&self) -> Option<&Clip> {
        self.clips.get(self.current)
    }
    fn advance_to_next(&mut self) {
        if !self.clips.is_empty() {
            self.current = (self.current + 1) % self.clips.len();
        }
    }
}

/// Read [`AnimationClip`]s from the loaded [`Gltf`] and write them to [`Clips`].
fn assign_clips(
    mut players: Query<&mut AnimationPlayer>,
    scene_handle: Res<SceneHandle>,
    animation_clips: Res<Assets<AnimationClip>>,
    gltf_assets: Res<Assets<Gltf>>,
    mut commands: Commands,
    mut setup: Local<bool>,
) {
    if scene_handle.is_loaded && !*setup {
        *setup = true;
    } else {
        return;
    }
    let gltf = gltf_assets.get(&scene_handle.gltf_handle).unwrap();
    let animations = &gltf.animations;
    if !animations.is_empty() {
        let count = animations.len();
        let plural = if count == 1 { "" } else { "s" };
        info!("Found {count} animation{plural}");
        let names: Vec<_> = gltf.named_animations.keys().collect();
        info!("Animation names: {names:?}");
    }
    let gltf_animation_name = |id: HandleId| {
        let named = gltf.named_animations.iter().find(|(_, h)| h.id() == id);
        Clip {
            name: named.map(|(name, _)| name.clone()),
            handle: named.map_or_else(
                || animation_clips.get_handle(id),
                |(_, handle)| handle.clone_weak(),
            ),
        }
    };
    let clips = Clips::new(animation_clips.ids().map(gltf_animation_name).collect());
    if let Some(current) = clips.current() {
        for mut player in &mut players {
            player.play(current.handle.clone_weak()).repeat();
        }
        commands.insert_resource(clips);
    }
}

fn handle_inputs(
    keyboard_input: Res<Input<KeyCode>>,
    mut clips: ResMut<Clips>,
    mut animation_player: Query<&mut AnimationPlayer>,
) {
    for mut player in &mut animation_player {
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.is_paused() {
                info!("Resuming animations");
                player.resume();
            } else {
                info!("Pausing animations");
                player.pause();
            }
        }
        if clips.clips.len() <= 1 {
            continue;
        }

        if keyboard_input.just_pressed(KeyCode::Return) {
            let resume = !player.is_paused();
            // set the current animation to its start and pause it to reset to its starting state
            player.set_elapsed(0.0).pause();

            clips.advance_to_next();
            let current_clip = clips.current().unwrap();
            if let Some(animation_name) = &current_clip.name {
                info!("Now playing {animation_name}");
            } else {
                info!("Switching to new animation");
            }
            player.play(current_clip.handle.clone_weak()).repeat();
            if resume {
                player.resume();
            }
        }
    }
}

pub struct AnimationManipulationPlugin;
impl Plugin for AnimationManipulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_inputs.run_if(resource_exists::<Clips>()),
                assign_clips,
            ),
        );
    }
}
