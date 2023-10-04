//! Control animations of entities in the loaded scene.
use bevy::{gltf::Gltf, prelude::*};

use crate::scene_viewer_plugin::SceneHandle;

/// Controls animation clips for a unique entity.
#[derive(Component)]
struct Clips {
    clips: Vec<Handle<AnimationClip>>,
    current: usize,
}
impl Clips {
    fn new(clips: Vec<Handle<AnimationClip>>) -> Self {
        Clips { clips, current: 0 }
    }
    /// # Panics
    ///
    /// When no clips are present.
    fn current(&self) -> Handle<AnimationClip> {
        self.clips[self.current].clone_weak()
    }
    fn advance_to_next(&mut self) {
        self.current = (self.current + 1) % self.clips.len();
    }
}

/// Read [`AnimationClip`]s from the loaded [`Gltf`] and assign them to the
/// entities they control. [`AnimationClip`]s control specific entities, and
/// trying to play them on an [`AnimationPlayer`] controlling a different
/// entities will result in odd animations, we take extra care to store
/// animation clips for given entities in the [`Clips`] component we defined
/// earlier in this file.
fn assign_clips(
    mut players: Query<(Entity, &mut AnimationPlayer, &Name)>,
    scene_handle: Res<SceneHandle>,
    clips: Res<Assets<AnimationClip>>,
    gltf_assets: Res<Assets<Gltf>>,
    assets: Res<AssetServer>,
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
        info!("Found {} animation{plural}", animations.len());
        let names: Vec<_> = gltf.named_animations.keys().collect();
        info!("Animation names: {names:?}");
    }
    for (entity, mut player, name) in &mut players {
        let clips = clips
            .iter()
            .filter_map(|(k, v)| v.compatible_with(name).then_some(k))
            .map(|id| assets.get_id_handle(id).unwrap())
            .collect();
        let animations = Clips::new(clips);
        player.play(animations.current()).repeat();
        commands.entity(entity).insert(animations);
    }
}

fn handle_inputs(
    keyboard_input: Res<Input<KeyCode>>,
    mut animation_player: Query<(&mut AnimationPlayer, &mut Clips, Entity, Option<&Name>)>,
) {
    for (mut player, mut clips, entity, name) in &mut animation_player {
        let display_entity_name = match name {
            Some(name) => name.to_string(),
            None => format!("entity {entity:?}"),
        };
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.is_paused() {
                info!("resuming animation for {display_entity_name}");
                player.resume();
            } else {
                info!("pausing animation for {display_entity_name}");
                player.pause();
            }
        }
        if clips.clips.len() <= 1 {
            continue;
        }

        if keyboard_input.just_pressed(KeyCode::Return) {
            info!("switching to new animation for {display_entity_name}");

            let resume = !player.is_paused();
            // set the current animation to its start and pause it to reset to its starting state
            player.seek_to(0.0).pause();

            clips.advance_to_next();
            let current_clip = clips.current();
            player.play(current_clip).repeat();
            if resume {
                player.resume();
            }
        }
    }
}

pub struct AnimationManipulationPlugin;
impl Plugin for AnimationManipulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_inputs, assign_clips));
    }
}
