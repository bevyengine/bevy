//! Animation for the game engine Bevy

#![warn(missing_docs)]

use std::ops::Deref;

use bevy_app::{App, CoreStage, Plugin};
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::{Entity, MapEntities},
    prelude::Component,
    reflect::ReflectComponent,
    schedule::IntoSystemDescriptor,
    system::{Query, Res},
};
use bevy_hierarchy::Children;
use bevy_math::{Quat, Vec3};
use bevy_reflect::{FromReflect, Reflect, TypeUuid};
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_utils::{tracing::warn, HashMap};

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AnimationClip, AnimationPlayer, AnimationPlugin, EntityPath, Keyframes, VariableCurve,
    };
}

/// List of keyframes for one of the attribute of a [`Transform`].
#[derive(Reflect, FromReflect, Clone, Debug)]
pub enum Keyframes {
    /// Keyframes for rotation.
    Rotation(Vec<Quat>),
    /// Keyframes for translation.
    Translation(Vec<Vec3>),
    /// Keyframes for scale.
    Scale(Vec<Vec3>),
}

/// Describes how an attribute of a [`Transform`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Reflect, FromReflect, Clone, Debug)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    pub keyframes: Keyframes,
}

/// Path to an entity, with [`Name`]s. Each entity in a path must have a name.
#[derive(Reflect, FromReflect, Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct EntityPath {
    /// Parts of the path
    pub parts: Vec<Name>,
}

/// A list of [`VariableCurve`], and the [`EntityPath`] to which they apply.
#[derive(Reflect, FromReflect, Clone, TypeUuid, Debug, Default)]
#[uuid = "d81b7179-0448-4eb0-89fe-c067222725bf"]
pub struct AnimationClip {
    paths: HashMap<EntityPath, usize>,
    path_curves: Vec<PathCurves>,
    duration: f32,
}

#[derive(Reflect, FromReflect, Clone, Debug, Default)]
struct PathCurves {
    /// Keyframes for rotation.
    rotation: Vec<(f32, Quat)>,
    /// Keyframes for translation.
    translation: Vec<(f32, Vec3)>,
    /// Keyframes for scale.
    scale: Vec<(f32, Vec3)>,
}

impl AnimationClip {
    /// Duration of the clip, represented in seconds
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Add a [`VariableCurve`] to an [`EntityPath`].
    pub fn add_curve_to_path(&mut self, path: EntityPath, curve: VariableCurve) {
        // Update the duration of the animation by this curve duration if it's longer
        self.duration = self.duration.max(
            (curve.keyframe_timestamps.last())
                .copied()
                .unwrap_or_default(),
        );
        let curves_len = self.path_curves.len();
        let curve_index = *self.paths.entry(path).or_insert_with(|| {
            self.path_curves.push(PathCurves {
                rotation: vec![],
                translation: vec![],
                scale: vec![],
            });
            curves_len
        });
        let timestamps = curve.keyframe_timestamps.into_iter();
        match curve.keyframes {
            Keyframes::Rotation(r) => self.path_curves[curve_index]
                .rotation
                .extend(timestamps.zip(r)),
            Keyframes::Translation(t) => self.path_curves[curve_index]
                .translation
                .extend(timestamps.zip(t)),
            Keyframes::Scale(s) => self.path_curves[curve_index]
                .scale
                .extend(timestamps.zip(s)),
        }
    }
}

/// Animation controls
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationPlayer {
    paused: bool,
    repeat: bool,
    speed: f32,
    elapsed: f32,
    animation_clip: Handle<AnimationClip>,
    #[reflect(ignore)]
    memoize: AnimationPlayerMemoize,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            paused: false,
            repeat: false,
            speed: 1.0,
            elapsed: 0.0,
            animation_clip: Default::default(),
            memoize: Default::default(),
        }
    }
}

#[derive(Default)]
struct AnimationPlayerMemoize {
    path_entities: HashMap<EntityPath, Entity>,
    curve_state: HashMap<usize, CurveState>,
}
impl AnimationPlayerMemoize {
    fn is_empty(&self) -> bool {
        self.curve_state.is_empty()
    }

    fn get_entity(
        &mut self,
        root_entity: Entity,
        names: &Query<&Name>,
        children: &Query<&Children>,
        path: &EntityPath,
    ) -> Option<Entity> {
        if let Some(current_entity) = self.path_entities.get(path) {
            return Some(*current_entity);
        }

        let mut entity = root_entity;
        // Ignore the first name, it is the root node which we already have
        for part in path.parts.iter().skip(1) {
            let mut children = children.get(entity).ok()?.deref().into_iter();
            entity = *children.find(|child| names.get(**child) == Ok(part))?;
        }

        self.path_entities.insert(path.clone(), entity);

        Some(entity)
    }
}

struct CurveState {
    entity: Entity,
    rotation_keyframe_hint: Option<usize>,
    translation_keyframe_hint: Option<usize>,
    scale_keyframe_hint: Option<usize>,
}

impl MapEntities for AnimationPlayer {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::entity::EntityMap,
    ) -> Result<(), bevy_ecs::entity::MapEntitiesError> {
        let AnimationPlayerMemoize {
            path_entities,
            curve_state,
        } = &mut self.memoize;
        for entity in path_entities.values_mut() {
            *entity = entity_map.get(*entity)?;
        }
        for state in curve_state.values_mut() {
            state.entity = entity_map.get(state.entity)?;
        }
        Ok(())
    }
}

impl AnimationPlayer {
    /// Start playing an animation, resetting state of the player
    pub fn start(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        let path_entities = std::mem::take(&mut self.memoize.path_entities);
        *self = Self {
            animation_clip: handle,
            memoize: AnimationPlayerMemoize {
                path_entities,
                ..Default::default()
            },
            ..Default::default()
        };
        self
    }

    /// Start playing an animation, resetting state of the player, unless the requested animation is already playing.
    pub fn play(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        if self.animation_clip != handle || self.is_paused() {
            self.start(handle);
        }
        self
    }

    /// Set the animation to repeat
    pub fn repeat(&mut self) -> &mut Self {
        self.repeat = true;
        self
    }

    /// Stop the animation from repeating
    pub fn stop_repeating(&mut self) -> &mut Self {
        self.repeat = false;
        self
    }

    /// Pause the animation
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpause the animation
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Is the animation paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Speed of the animation playback
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Set the speed of the animation playback
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    /// Time elapsed playing the animation
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Seek to a specific time in the animation
    pub fn set_elapsed(&mut self, elapsed: f32) -> &mut Self {
        self.elapsed = elapsed;
        for curve_state in self.memoize.curve_state.values_mut() {
            curve_state.rotation_keyframe_hint = None;
            curve_state.translation_keyframe_hint = None;
            curve_state.scale_keyframe_hint = None;
        }
        self
    }
}

/// System that will play all animations, using any entity with a [`AnimationPlayer`]
/// and a [`Handle<AnimationClip>`] as an animation root
pub fn animation_player(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip>>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
    names: Query<&Name>,
    mut transforms: Query<&mut Transform>,
    children: Query<&Children>,
) {
    for (entity, mut player) in &mut animation_players {
        if let Some(animation_clip) = animations.get(&player.animation_clip) {
            // This is only done once when when the animation is started.
            if player.memoize.is_empty() {
                for (path, curve_index) in animation_clip.paths.iter() {
                    if let Some(entity) =
                        (player.memoize).get_entity(entity, &names, &children, path)
                    {
                        player.memoize.curve_state.insert(
                            *curve_index,
                            CurveState {
                                entity,
                                rotation_keyframe_hint: None,
                                translation_keyframe_hint: None,
                                scale_keyframe_hint: None,
                            },
                        );
                    } else {
                        warn!("Entity not found for path {:?}", path);
                    }
                }
            }

            // Continue if paused unless the `AnimationPlayer` was changed
            // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
            if player.paused && !player.is_changed() {
                continue;
            }
            if !player.paused {
                player.elapsed += time.delta_seconds() * player.speed;
            }
            let mut elapsed = player.elapsed;
            if player.repeat {
                elapsed %= animation_clip.duration;
            }
            if elapsed < 0.0 {
                elapsed += animation_clip.duration;
            }
            for (idx, path_curves) in animation_clip.path_curves.iter().enumerate() {
                let CurveState {
                    entity: current_entity,
                    ref mut rotation_keyframe_hint,
                    ref mut translation_keyframe_hint,
                    ref mut scale_keyframe_hint,
                } = *if let Some(state) = player.memoize.curve_state.get_mut(&idx) {
                    state
                } else {
                    continue;
                };

                let mut transform = if let Ok(t) = transforms.get_mut(current_entity) {
                    t
                } else {
                    continue;
                };

                #[cold]
                fn search_range<T>(elapsed: f32, keyframes: &[(f32, T)]) -> (usize, usize) {
                    debug_assert!(!keyframes.is_empty());

                    let len = keyframes.len();
                    match keyframes.binary_search_by(|(k, _)| k.partial_cmp(&elapsed).unwrap()) {
                        Ok(idx) => (idx, (idx + 1) % len),
                        Err(idx) => (idx.max(1) - 1, idx),
                    }
                }

                fn interpolate<T: Copy>(
                    elapsed: f32,
                    hint: &mut Option<usize>,
                    keyframes: &[(f32, T)],
                    apply: impl FnOnce(f32, T, T) -> T,
                ) -> T {
                    debug_assert!(!keyframes.is_empty());

                    let (idx, next_idx) = if let Some(hint_idx) = hint.take() {
                        const BINARY_MIN: usize = 3;
                        let linear_end = (hint_idx + BINARY_MIN + 1).min(keyframes.len() - 1);

                        if elapsed < keyframes[hint_idx].0 {
                            search_range(elapsed, &keyframes[..=hint_idx])
                        } else if keyframes[linear_end].0 < elapsed {
                            let ids = search_range(elapsed, &keyframes[linear_end..]);
                            (linear_end + ids.0, linear_end + ids.1)
                        } else {
                            // It's most likely in the next few values, so assume a linear search over those.
                            let next_idx = (keyframes[(hint_idx + 1)..linear_end].iter())
                                .position(|(t, _)| elapsed < *t)
                                .map(|idx| idx + hint_idx + 1)
                                .unwrap_or(linear_end);
                            let idx = next_idx - 1;
                            *hint = Some(idx);
                            (idx, next_idx)
                        }
                    } else {
                        search_range(elapsed, keyframes)
                    };

                    *hint = Some(idx);
                    let kf = &keyframes[idx];
                    let kf_next = &keyframes[next_idx];
                    let delta = if kf.0 < kf_next.0 {
                        (elapsed - kf.0) / (kf_next.0 - kf.0)
                    } else {
                        // TODO: Does this need to wrap?
                        0.0
                    };

                    apply(delta, kf.1, kf_next.1)
                }

                // Update the rotation.
                if !path_curves.rotation.is_empty() {
                    let rotation = interpolate(
                        elapsed,
                        rotation_keyframe_hint,
                        &path_curves.rotation,
                        |lerp, rot_start, mut rot_end| {
                            // Choose the smallest angle for the rotation
                            if rot_end.dot(rot_start) < 0.0 {
                                rot_end = -rot_end;
                            }
                            // Rotations are using a spherical linear interpolation
                            rot_start.normalize().slerp(rot_end.normalize(), lerp)
                        },
                    );
                    transform.rotation = rotation;
                }

                // Update the translation.
                if !path_curves.translation.is_empty() {
                    let translation = interpolate(
                        elapsed,
                        translation_keyframe_hint,
                        &path_curves.translation,
                        |lerp, translation_start, translation_end| {
                            translation_start.lerp(translation_end, lerp)
                        },
                    );
                    transform.translation = translation;
                }

                // Update the scale.
                if !path_curves.scale.is_empty() {
                    let scale = interpolate(
                        elapsed,
                        scale_keyframe_hint,
                        &path_curves.scale,
                        |lerp, scale_start, scale_end| scale_start.lerp(scale_end, lerp),
                    );
                    transform.scale = scale;
                }
            }
        }
    }
}

/// Adds animation support to an app
#[derive(Default)]
pub struct AnimationPlugin {}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AnimationClip>()
            .register_asset_reflect::<AnimationClip>()
            .register_type::<AnimationPlayer>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                animation_player.before(TransformSystem::TransformPropagate),
            );
    }
}
