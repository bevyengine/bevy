use anyhow::Result;
use bevy_asset::{Assets, Handle /*HandleUntyped*/};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_property::Properties;
//use bevy_render::color::Color;
use bevy_transform::prelude::*;
use bevy_type_registry::TypeUuid;
use fnv::FnvHashMap as HashMap;
use smallvec::{smallvec, SmallVec};
use std::any::Any;

use crate::curve::Curve;
use crate::hierarchy::Hierarchy;
use crate::lerping::LerpValue;

#[derive(Debug)]
pub struct Curves<T> {
    id: usize,
    /// Maps each curve to an entity index or other value
    indexes: SmallVec<[u16; 8]>,
    curves: Vec<Curve<T>>,
}

impl<T> Curves<T> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.curves.len()
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (u16, &Curve<T>)> {
        self.indexes.iter().copied().zip(self.curves.iter())
    }
}

#[derive(Debug)]
pub struct CurvesUntyped {
    duration: f32,
    untyped: Box<dyn Any + 'static>,
}

unsafe impl Send for CurvesUntyped {}
unsafe impl Sync for CurvesUntyped {}

impl CurvesUntyped {
    #[inline(always)]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&Curves<T>> {
        self.untyped.downcast_ref()
    }

    #[inline(always)]
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut Curves<T>> {
        self.untyped.downcast_mut()
    }
}

// TODO: impl Serialize, Deserialize
#[derive(Debug, TypeUuid)]
#[uuid = "79e2ea58-8bf7-43af-8219-5898edb02f80"]
pub struct Clip {
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    duration: f32,
    /// Entity identification made by parent index and name
    hierarchy: Hierarchy,
    properties: HashMap<String, CurvesUntyped>,
}

// fn clip_default_warp() -> bool {
//     true
// }

impl Default for Clip {
    fn default() -> Self {
        Self {
            warp: true,
            duration: 0.0,
            hierarchy: Hierarchy::default(),
            properties: HashMap::default(),
        }
    }
}

impl Clip {
    /// Property to be animated must be in the following format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    ///
    /// **NOTE** This is a expensive function
    pub fn add_animated_prop<T>(&mut self, property_path: &str, mut curve: Curve<T>)
    where
        T: LerpValue + Clone + 'static,
    {
        // Split in entity and attribute path,
        // NOTE: use rfind because it's expected the latter to be generally shorter
        let path =
            property_path.split_at(property_path.rfind('@').expect("property path missing @"));

        let (entity_index, _) = self.hierarchy.get_or_insert_entity(path.0);
        let target_name = path.1.split_at(1).1;

        if let Some(curves_untyped) = self.properties.get_mut(target_name) {
            let curves = curves_untyped
                .downcast_mut::<T>()
                .expect("properties can't have the same name and different curve types");

            // If some entity was created it means this property is a new one so we can safely skip the attribute testing
            if let Some(i) = curves
                .indexes
                .iter()
                .position(|index| *index == entity_index)
            {
                // Found a property equal to the one been inserted, next replace the curve
                std::mem::swap(&mut curves.curves[i], &mut curve);

                // Update curve duration in two stages
                let duration = curves
                    .curves
                    .iter()
                    .map(|c| c.duration())
                    .fold(0.0f32, |acc, x| acc.max(x));

                std::mem::drop(curves);
                curves_untyped.duration = duration;

                std::mem::drop(curves_untyped);
                self.duration = self
                    .properties
                    .iter()
                    .map(|(_, c)| c.duration)
                    .fold(0.0f32, |acc, x| acc.max(x));
            } else {
                // Append newly added curve
                let duration = curve.duration();
                curves.curves.push(curve);
                curves.indexes.push(entity_index);
                std::mem::drop(curves);

                self.duration = self.duration.max(duration);
                curves_untyped.duration = curves_untyped.duration.max(duration);
            }
            return;
        }

        self.duration = self.duration.max(curve.duration());
        let id = self.properties.len();
        self.properties.insert(
            target_name.to_string(),
            CurvesUntyped {
                duration: curve.duration(),
                untyped: Box::new(Curves {
                    id,
                    curves: vec![curve],
                    indexes: smallvec![entity_index],
                }),
            },
        );
    }

    // /// Number of animated properties in this clip
    // #[inline(always)]
    // pub fn len(&self) -> u16 {
    //     self.curves.len() as u16
    // }

    // /// Returns the property curve property path.
    // ///
    // /// The clip stores a property path in a specific way to improve search performance
    // /// thus it needs to rebuilt the curve property path in the human readable format
    // pub fn get_property_path(&self, index: u16) -> String {
    //     let CurveEntry {
    //         entity_index,
    //         property_index,
    //     } = &self.entries[index as usize];
    //
    //     format!(
    //         "{}@{}",
    //         self.hierarchy
    //             .get_entity_path_at(*entity_index)
    //             .expect("property as an invalid entity"),
    //         self.properties[*property_index as usize].as_str()
    //     )
    // }

    /// Clip duration
    #[inline(always)]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[inline(always)]
    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hierarchy
    }

    // #[inline(always)]
    // pub fn properties(&self) -> &[Name] {
    //     &self.properties[..]
    // }

    // #[inline(always)]
    // pub fn curves(&self) -> impl Iterator<Item = (&Curves, &CurveUntyped)> {
    //     self.entries.iter().zip(self.curves.iter())
    // }

    // #[inline(always)]
    // pub fn get(&self, curve_index: u16) -> Option<&CurveUntyped> {
    //     self.curves.get(curve_index as usize)
    // }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Properties)]
pub struct Layer {
    pub weight: f32,
    pub clip: u16,
    pub time: f32,
    pub time_scale: f32,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            weight: 1.0,
            clip: 0,
            time: 0.0,
            time_scale: 1.0,
        }
    }
}

#[derive(Default, Debug)]
struct Bind {
    entities: Vec<Option<Entity>>,
}

#[derive(Debug, Properties)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,
    #[property(ignore)]
    bind_clips: Vec<Bind>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            clips: vec![],
            bind_clips: vec![],
            time_scale: 1.0,
            layers: vec![],
        }
    }
}

impl Animator {
    pub fn add_clip(&mut self, clip: Handle<Clip>) -> u16 {
        if let Some(i) = self.clips.iter().position(|c| *c == clip) {
            i as u16
        } else {
            // TODO: assert too many clips ...
            let i = self.clips.len();
            self.clips.push(clip);
            i as u16
        }
    }

    pub fn add_layer(&mut self, clip: Handle<Clip>, weight: f32) -> u16 {
        let clip = self.add_clip(clip);
        let layer_index = self.layers.len();
        self.layers.push(Layer {
            clip,
            weight,
            ..Default::default()
        });
        layer_index as u16
    }

    pub fn clips_len(&self) -> u16 {
        self.clips.len() as u16
    }

    pub fn animate<'a>(&'a self) -> LayerIterator<'a> {
        LayerIterator {
            index: 0,
            animator: self,
        }
    }
}
pub struct LayerIterator<'a> {
    index: usize,
    animator: &'a Animator,
}

impl<'a> Iterator for LayerIterator<'a> {
    type Item = (usize, &'a Layer, &'a Handle<Clip>, &'a [Option<Entity>]);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;

        let layer = self.animator.layers.get(index)?;
        let clip_handle = self.animator.clips.get(layer.clip as usize)?;
        let entities = &self.animator.bind_clips.get(layer.clip as usize)?.entities[..];

        return Some((index, layer, clip_handle, entities));
    }
}

#[derive(Default, Debug, Properties)]
pub struct KeyframeCache {
    cache: Vec<Vec<usize>>,
}

impl KeyframeCache {
    pub fn get(&mut self, index: usize) -> &mut Vec<usize> {
        self.cache.resize_with(index + 1, Default::default);
        &mut self.cache[index]
    }
}

///////////////////////////////////////////////////////////////////////////////

#[tracing::instrument(skip(commands, time, clips, animators_query, children_query, name_query))]
pub(crate) fn animator_update_system(
    commands: &mut Commands,
    time: Res<Time>,
    clips: Res<Assets<Clip>>,
    mut animators_query: Query<(
        Entity,
        &mut Animator,
        Option<&KeyframeCache>,
        Option<&AnimatorBlending>,
    )>,
    mut children_query: Query<(&Children,)>,
    mut name_query: Query<(&Parent, &Name)>,
) {
    for (animator_entity, mut animator, keyframe_cache, visited) in animators_query.iter_mut() {
        let animator = &mut *animator;

        // Insert KeyframeCache if not already
        if keyframe_cache.is_none() {
            commands.insert_one(animator_entity, KeyframeCache::default());
        }

        if visited.is_none() {
            commands.insert_one(animator_entity, AnimatorBlending::default());
        }

        // Time scales by component
        let delta_time = time.delta_seconds * animator.time_scale;

        let w_total = animator
            .layers
            .iter()
            .fold(0.0, |w, layer| w + layer.weight);

        let norm = 1.0 / w_total;

        // Normalize all states weights
        for layer in &mut animator.layers {
            layer.weight *= norm;
        }

        // Make run for the binds
        animator
            .bind_clips
            .resize_with(animator.clips.len(), Bind::default);

        for (clip_index, clip_handle) in animator.clips.iter().enumerate() {
            if let Some(clip) = clips.get(clip_handle) {
                let bind = &mut animator.bind_clips[clip_index];

                // Prepare the entities table cache
                bind.entities.clear();
                bind.entities.resize(clip.hierarchy().len(), None);
                // Assign the root entity as the first element
                bind.entities[0] = Some(animator_entity);

                // Find entitites ...
                for entity_index in 1..clip.hierarchy().len() {
                    clip.hierarchy().find_entity(
                        entity_index as u16,
                        &mut bind.entities,
                        &mut children_query,
                        &mut name_query,
                    );
                }

                // let curves_count = clip.len() as usize;

                for layer in &mut animator.layers {
                    if layer.clip as usize != clip_index {
                        continue;
                    }

                    // // Ensure capacity for cached keyframe index vec
                    // if layer.keyframe.len() != curves_count {
                    //     layer.keyframe.clear();
                    //     layer
                    //         .keyframe
                    //         .resize_with(curves_count, || Default::default());
                    // }

                    // Update time
                    let mut time = layer.time + delta_time * layer.time_scale;

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration() {
                            time = (time / clip.duration()).fract() * clip.duration();
                            // // Reset all keyframes cached indexes
                            // layer
                            //     .keyframe
                            //     .iter_mut()
                            //     .for_each(|x| *x = Default::default())
                        }
                    } else {
                        // Hold
                        time = time.min(clip.duration());
                    }

                    // Advance state time
                    layer.time = time;
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Ptr(*const u8);

// SAFETY: Store pointers to each attribute to be updated, a clip can't have two pointers
// with the same value. Each clip per Animator will be updated sequentially
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

#[derive(Default, Debug)]
pub struct AnimatorBlending {
    table: fnv::FnvHashSet<Ptr>,
}

impl AnimatorBlending {
    pub fn begin_blending(&mut self) -> AnimatorBlendGroup {
        self.table.clear();
        AnimatorBlendGroup { blending: self }
    }
}

pub struct AnimatorBlendGroup<'a> {
    blending: &'a mut AnimatorBlending,
}

impl<'a> AnimatorBlendGroup<'a> {
    pub fn blend_lerp<T: LerpValue>(&mut self, attribute: &mut T, value: T, weight: f32) {
        let ptr = Ptr(attribute as *const _ as *const u8);
        if self.blending.table.contains(&ptr) {
            *attribute = LerpValue::lerp(&*attribute, &value, weight);
        } else {
            self.blending.table.insert(ptr);
            *attribute = value;
        }
    }
}

#[tracing::instrument(skip(clips, animators_query, transform_query))]
pub(crate) fn animator_transform_update_system(
    clips: Res<Assets<Clip>>,
    mut animators_query: Query<(&Animator, &mut KeyframeCache, &mut AnimatorBlending)>,
    transform_query: Query<(&mut Transform,)>,
) {
    let mut components = vec![];

    for (animator, mut keyframe_cache, mut animator_blend) in animators_query.iter_mut() {
        let keyframe_cache = &mut *keyframe_cache;
        let mut blend_group = animator_blend.begin_blending();

        for (_, layer, clip_handle, entities) in animator.animate() {
            let w = layer.weight;
            if w < 1.0e-8 {
                continue;
            }

            if let Some(clip) = clips.get(clip_handle) {
                let time = layer.time;

                // ~15us
                components.clear();

                // SAFETY: Pre-fetch all transforms to avoid calling get_mut multiple times
                // this is safe because it doesn't change the safe logic
                unsafe {
                    for entry in entities {
                        components.push(
                            entry
                                .map(|entity| transform_query.get_unsafe(entity).ok())
                                .flatten()
                                .map(|(transform,)| transform),
                        );
                    }
                }

                // ~23us
                if let Some(curves) = clip
                    .properties
                    .get("Transform.translation")
                    .map(|curve_untyped| curve_untyped.downcast_ref::<Vec3>())
                    .flatten()
                {
                    // Get keyframes and ensure capacity
                    let keyframes = keyframe_cache.get(curves.id);
                    keyframes.resize(curves.len() as usize, 0);

                    for (curve_index, (entity_index, curve)) in curves.iter().enumerate() {
                        if let Some(ref mut component) = components[entity_index as usize] {
                            let (k, v) = curve.sample_indexed(keyframes[curve_index], time);
                            keyframes[curve_index] = k;
                            // let v = curve.sample(time);
                            blend_group.blend_lerp(&mut component.translation, v, w);
                        }
                    }
                }

                // ~23us
                if let Some(curves) = clip
                    .properties
                    .get("Transform.rotation")
                    .map(|curve_untyped| curve_untyped.downcast_ref::<Quat>())
                    .flatten()
                {
                    // Get keyframes and ensure capacity
                    let keyframes = keyframe_cache.get(curves.id);
                    keyframes.resize(curves.len() as usize, 0);

                    for (curve_index, (entity_index, curve)) in curves.iter().enumerate() {
                        if let Some(ref mut component) = components[entity_index as usize] {
                            let (k, v) = curve.sample_indexed(keyframes[curve_index], time);
                            keyframes[curve_index] = k;
                            // let v = curve.sample(time);
                            blend_group.blend_lerp(&mut component.rotation, v, w);
                        }
                    }
                }

                // ~23us
                if let Some(curves) = clip
                    .properties
                    .get("Transform.scale")
                    .map(|curve_untyped| curve_untyped.downcast_ref::<Vec3>())
                    .flatten()
                {
                    // Get keyframes and ensure capacity
                    let keyframes = keyframe_cache.get(curves.id);
                    keyframes.resize(curves.len() as usize, 0);

                    for (curve_index, (entity_index, curve)) in curves.iter().enumerate() {
                        if let Some(ref mut component) = components[entity_index as usize] {
                            let (k, v) = curve.sample_indexed(keyframes[curve_index], time);
                            keyframes[curve_index] = k;
                            //let v = curve.sample(time);
                            blend_group.blend_lerp(&mut component.scale, v, w);
                        }
                    }
                }
            }
        }
    }
}
