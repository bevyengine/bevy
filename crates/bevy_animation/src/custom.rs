use anyhow::Result;
use bevy_asset::{Assets, Handle /*, HandleUntyped*/};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_property::Properties;
use bevy_transform::prelude::*;

use crate::clip::Clip;
use crate::curve::CurveUntyped;
use crate::lerping::LerpValue;

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

#[derive(Default, Debug, Properties)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,
    #[property(ignore)]
    bind_clips: Vec<Bind>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
}

#[derive(Default, Debug)]
struct Bind {
    entities: Vec<Option<Entity>>,
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
    type Item = (&'a Layer, &'a Handle<Clip>, &'a [Option<Entity>]);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;

        let layer = self.animator.layers.get(index)?;
        let clip_handle = self.animator.clips.get(layer.clip as usize)?;
        let entities = &self.animator.bind_clips.get(layer.clip as usize)?.entities[..];

        return Some((layer, clip_handle, entities));
    }
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) fn animator_update_system(
    time: Res<Time>,
    clips: Res<Assets<Clip>>,
    mut animators_query: Query<(Entity, &mut Animator)>,
    mut children_query: Query<(&Children,)>,
    mut name_query: Query<(&Parent, &Name)>,
) {
    let delta_time = time.delta_seconds;

    for (animator_entity, mut animator) in animators_query.iter_mut() {
        let animator = &mut *animator;

        // Time scales by component
        let delta_time = delta_time * animator.time_scale;

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

#[derive(Default, Debug)]
pub struct Visited {
    table: fnv::FnvHashSet<*const u8>,
}

impl Visited {
    pub fn clear(&mut self) {
        self.table.clear()
    }

    pub fn is_visited<T>(&mut self, ptr: *const T) -> bool {
        let ptr = ptr as *const u8;
        if self.table.contains(&ptr) {
            true
        } else {
            self.table.insert(ptr);
            false
        }
    }
}

pub(crate) fn animator_transform_update_system(
    clips: Res<Assets<Clip>>,
    mut animators_query: Query<(&mut Animator,)>,
    mut transform_query: Query<(&mut Transform,)>,
) {
    // TODO: Make const
    let translation_name: Name = Name::from_str("Transform.translation");
    let rotation_name: Name = Name::from_str("Transform.rotation");
    let scale_name: Name = Name::from_str("Transform.scale");

    let mut visited = Visited::default();
    // let mut transforms = vec![];

    for (mut animator,) in animators_query.iter_mut() {
        let animator = &mut *animator;
        visited.clear();

        for (layer, clip_handle, entities) in animator.animate() {
            let w = layer.weight;
            if w < 1.0e-8 {
                continue;
            }

            let time = layer.time;

            if let Some(clip) = clips.get(clip_handle) {
                // Fetch properties indexes for this particular clip
                let mut translation = u16::MAX;
                let mut rotation = u16::MAX;
                let mut scale = u16::MAX;

                for (property_index, prop_name) in clip.properties().iter().enumerate() {
                    if translation == u16::MAX && prop_name == &translation_name {
                        translation = property_index as u16;
                    } else if rotation == u16::MAX && prop_name == &rotation_name {
                        rotation = property_index as u16;
                    } else if scale == u16::MAX && prop_name == &scale_name {
                        scale = property_index as u16;
                    }
                }

                // transforms.clear();

                // // SAFETY: Pre-fetch all transforms to avoid calling get_mut multiple fimes
                // // this is safe because it doesn't change the safe logic
                // unsafe {
                //     for entry in entities {
                //         transforms.push(
                //             entry
                //                 .map(|entity| transform_query.get_unsafe(entity).ok())
                //                 .flatten()
                //                 .map(|(transform,)| transform),
                //         );
                //     }
                // }

                for (curve_index, (entry, curve)) in clip.curves().enumerate() {
                    let property_index = entry.property_index;

                    // Pre check before fetching the entity
                    if property_index != translation
                        || property_index != rotation
                        || property_index != scale
                    {
                        continue;
                    }

                    if let Some(entity) = entities[entry.entity_index as usize] {
                        // Entity found
                        // TODO: Optimize for entity fetch
                        if let Ok((mut transform,)) = transform_query.get_mut(entity) {
                            if property_index == translation {
                                if let CurveUntyped::Vec3(curve) = curve {
                                    // let (k, v) =
                                    //     curve.sample_indexed(layer.keyframe[curve_index], time);
                                    // layer.keyframe[curve_index] = k;
                                    let v = curve.sample(time);

                                    if visited.is_visited(transform.translation.as_ref().as_ptr()) {
                                        transform.translation =
                                            LerpValue::lerp(&transform.translation, &v, w);
                                    } else {
                                        transform.translation = v;
                                    }
                                }
                            } else if property_index == rotation {
                                if let CurveUntyped::Quat(curve) = curve {
                                    // let (k, v) =
                                    //     curve.sample_indexed(layer.keyframe[curve_index], time);
                                    // layer.keyframe[curve_index] = k;
                                    let v = curve.sample(time);

                                    if visited.is_visited(transform.rotation.as_ref().as_ptr()) {
                                        transform.rotation =
                                            LerpValue::lerp(&transform.rotation, &v, w);
                                    } else {
                                        transform.rotation = v;
                                    }
                                }
                            } else if property_index == scale {
                                if let CurveUntyped::Vec3(curve) = curve {
                                    // let (k, v) =
                                    //     curve.sample_indexed(layer.keyframe[curve_index], time);
                                    // layer.keyframe[curve_index] = k;
                                    let v = curve.sample(time);

                                    if visited.is_visited(transform.scale.as_ref().as_ptr()) {
                                        transform.scale = LerpValue::lerp(&transform.scale, &v, w);
                                    } else {
                                        transform.scale = v;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
