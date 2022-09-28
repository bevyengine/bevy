//! Animation implementations for the game engine Bevy

#![warn(missing_docs)]

use std::ops::Deref;
use std::{fmt::Debug, ops::DerefMut};

use bevy_app::{App, CoreStage};
use bevy_asset::{AddAsset, Asset, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::Component,
    reflect::ReflectComponent,
    schedule::IntoSystemDescriptor,
    system::{Query, Res},
};
use bevy_hierarchy::Children;
use bevy_reflect::{GetTypeRegistration, Reflect, TypeUuid};
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_utils::{tracing::warn, HashMap};

use crate::common::EntityPath;

pub trait AnimationImpl: 'static + Sized {
    type Keyframes: Debug + Clone;
    type VariableCurve;
    type AnimationClip: Asset
        + Clone
        + Debug
        + TypeUuid
        + Default
        + DerefMut<Target = AnimationClipImpl<Self>>;
    type AnimationPlayer: Component
        + GetTypeRegistration
        + Reflect
        + DerefMut<Target = AnimationPlayerImpl<Self>>;

    fn assign(keyframes: &Self::Keyframes, index: usize, transform: &mut Transform);
    fn lerp(keyframes: &Self::Keyframes, start: usize, lerp: f32, transform: &mut Transform);
    fn keyframe_timestamps(curve: &Self::VariableCurve) -> &[f32];
    fn keyframes(curve: &Self::VariableCurve) -> &Self::Keyframes;
}

macro_rules! wrapped_impl {
    ($wrapper: ident ( $impl: ident < $generic: ident > )) => {
        impl ::std::ops::Deref for $wrapper {
            type Target = $impl<$generic>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl ::std::ops::DerefMut for $wrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}
pub(crate) use wrapped_impl;

#[derive(Clone, Debug)]
pub struct AnimationClipImpl<Impl: AnimationImpl> {
    curves: HashMap<EntityPath, Vec<Impl::VariableCurve>>,
    duration: f32,
}
impl<Impl: AnimationImpl> Default for AnimationClipImpl<Impl> {
    fn default() -> Self {
        Self {
            curves: Default::default(),
            duration: Default::default(),
        }
    }
}

impl<Impl: AnimationImpl> AnimationClipImpl<Impl> {
    #[inline]
    /// Hashmap of the [`VariableCurve`]s per [`EntityPath`].
    pub fn curves(&self) -> &HashMap<EntityPath, Vec<Impl::VariableCurve>> {
        &self.curves
    }

    /// Duration of the clip, represented in seconds
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Add a [`VariableCurve`] to an [`EntityPath`].
    pub fn add_curve_to_path(&mut self, path: EntityPath, curve: Impl::VariableCurve) {
        // Update the duration of the animation by this curve duration if it's longer
        self.duration = self
            .duration
            .max(*Impl::keyframe_timestamps(&curve).last().unwrap_or(&0.0));
        self.curves.entry(path).or_default().push(curve);
    }
}

/// Animation controls
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationPlayerImpl<Impl: AnimationImpl> {
    paused: bool,
    repeat: bool,
    speed: f32,
    elapsed: f32,
    animation_clip: Handle<Impl::AnimationClip>,
}

impl<Impl: AnimationImpl> Default for AnimationPlayerImpl<Impl> {
    fn default() -> Self {
        Self {
            paused: false,
            repeat: false,
            speed: 1.0,
            elapsed: 0.0,
            animation_clip: Default::default(),
        }
    }
}

impl<Impl: AnimationImpl> AnimationPlayerImpl<Impl> {
    /// Start playing an animation, resetting state of the player
    pub fn play(&mut self, handle: Handle<Impl::AnimationClip>) -> &mut Self {
        *self = Self {
            animation_clip: handle,
            ..Default::default()
        };
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
        self
    }
}

#[inline]
pub(crate) fn animation_player_impl<Impl: AnimationImpl>(
    time: Res<Time>,
    animations: Res<Assets<Impl::AnimationClip>>,
    mut animation_players: Query<(Entity, &mut Impl::AnimationPlayer)>,
    names: Query<&Name>,
    mut transforms: Query<&mut Transform>,
    children: Query<&Children>,
) {
    for (entity, mut player) in &mut animation_players {
        if let Some(animation_clip) = animations.get(&player.animation_clip) {
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
            'entity: for (path, curves) in &animation_clip.curves {
                // PERF: finding the target entity can be optimised
                let mut current_entity = entity;
                // Ignore the first name, it is the root node which we already have
                for part in path.parts.iter().skip(1) {
                    let mut found = false;
                    if let Ok(children) = children.get(current_entity) {
                        for child in children.deref() {
                            if let Ok(name) = names.get(*child) {
                                if name == part {
                                    // Found a children with the right name, continue to the next part
                                    current_entity = *child;
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found {
                        warn!("Entity not found for path {:?} on part {:?}", path, part);
                        continue 'entity;
                    }
                }
                if let Ok(mut transform) = transforms.get_mut(current_entity) {
                    for curve in curves {
                        let keyframe_timestamps = Impl::keyframe_timestamps(curve);
                        let keyframes = Impl::keyframes(curve);

                        // Some curves have only one keyframe used to set a transform
                        if keyframe_timestamps.len() == 1 {
                            Impl::assign(keyframes, 0, &mut transform);
                            continue;
                        }

                        // Find the current keyframe
                        // PERF: finding the current keyframe can be optimised
                        let step_start = match keyframe_timestamps
                            .binary_search_by(|probe| probe.partial_cmp(&elapsed).unwrap())
                        {
                            Ok(i) => i,
                            Err(0) => continue, // this curve isn't started yet
                            Err(n) if n > keyframe_timestamps.len() - 1 => continue, // this curve is finished
                            Err(i) => i - 1,
                        };
                        let ts_start = keyframe_timestamps[step_start];
                        let ts_end = keyframe_timestamps[step_start + 1];
                        let lerp = (elapsed - ts_start) / (ts_end - ts_start);

                        // Apply the keyframe
                        Impl::lerp(&keyframes, step_start, lerp, &mut transform);
                    }
                }
            }
        }
    }
}

pub(crate) fn build_plugin_impl<Impl: AnimationImpl>(app: &mut App) {
    app.add_asset::<Impl::AnimationClip>()
        .register_type::<Impl::AnimationPlayer>()
        .add_system_to_stage(
            CoreStage::PostUpdate,
            animation_player_impl::<Impl>.before(TransformSystem::TransformPropagate),
        );
}
