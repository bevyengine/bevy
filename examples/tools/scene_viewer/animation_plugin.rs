//! Control animations of entities in the loaded scene.
use std::collections::HashMap;

use bevy::{animation::AnimationTarget, ecs::entity::EntityHashMap, gltf::Gltf, prelude::*};

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

/// Automatically assign [`AnimationClip`]s to [`AnimationPlayer`] and play
/// them, if the clips refer to descendants of the animation player (which is
/// the common case).
#[allow(clippy::too_many_arguments)]
fn assign_clips(
    mut players: Query<&mut AnimationPlayer>,
    targets: Query<(Entity, &AnimationTarget)>,
    parents: Query<&Parent>,
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
    if animations.is_empty() {
        return;
    }

    let count = animations.len();
    let plural = if count == 1 { "" } else { "s" };
    info!("Found {} animation{plural}", animations.len());
    let names: Vec<_> = gltf.named_animations.keys().collect();
    info!("Animation names: {names:?}");

    // Map animation target IDs to entities.
    let animation_target_id_to_entity: HashMap<_, _> = targets
        .iter()
        .map(|(entity, target)| (target.id, entity))
        .collect();

    // Build up a list of all animation clips that belong to each player. A clip
    // is considered to belong to an animation player if all targets of the clip
    // refer to entities whose nearest ancestor player is that animation player.

    let mut player_to_clips: EntityHashMap<Vec<_>> = EntityHashMap::default();

    for (clip_id, clip) in clips.iter() {
        let mut ancestor_player = None;
        for target_id in clip.curves().keys() {
            // If the animation clip refers to entities that aren't present in
            // the scene, bail.
            let Some(&target) = animation_target_id_to_entity.get(target_id) else {
                continue;
            };

            // Find the nearest ancestor animation player.
            let mut current = Some(target);
            while let Some(entity) = current {
                if players.contains(entity) {
                    match ancestor_player {
                        None => {
                            // If we haven't found a player yet, record the one
                            // we found.
                            ancestor_player = Some(entity);
                        }
                        Some(ancestor) => {
                            // If we have found a player, then make sure it's
                            // the same player we located before.
                            if ancestor != entity {
                                // It's a different player. Bail.
                                ancestor_player = None;
                                break;
                            }
                        }
                    }
                }

                // Go to the next parent.
                current = parents.get(entity).ok().map(|parent| parent.get());
            }
        }

        let Some(ancestor_player) = ancestor_player else {
            warn!(
                "Unexpected animation hierarchy for animation clip {:?}; ignoring.",
                clip_id
            );
            continue;
        };

        let Some(clip_handle) = assets.get_id_handle(clip_id) else {
            warn!("Clip {:?} wasn't loaded.", clip_id);
            continue;
        };

        player_to_clips
            .entry(ancestor_player)
            .or_default()
            .push(clip_handle);
    }

    // Now that we've built up a list of all clips that belong to each player,
    // package them up into a `Clips` component, play the first such animation,
    // and add that component to the player.
    for (player_entity, clips) in player_to_clips {
        let Ok(mut player) = players.get_mut(player_entity) else {
            warn!("Animation targets referenced a nonexistent player. This shouldn't happen.");
            continue;
        };
        let animations = Clips::new(clips);
        player.play(animations.current()).repeat();
        commands.entity(player_entity).insert(animations);
    }
}

fn handle_inputs(
    keyboard_input: Res<ButtonInput<KeyCode>>,
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

        if keyboard_input.just_pressed(KeyCode::Enter) {
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
