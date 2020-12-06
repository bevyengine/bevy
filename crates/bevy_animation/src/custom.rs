use anyhow::Result;
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle /*, HandleUntyped*/};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::{Location, SystemId};
use bevy_math::prelude::*;
use bevy_property::{Properties, Property, PropertyType};
use bevy_transform::prelude::*;
use bevy_type_registry::{TypeRegistry, TypeUuid};
//use serde::{Deserialize, Serialize};
use fnv::FnvHashMap;
use smallvec::{smallvec, SmallVec};
use std::any::TypeId;
use std::hash::Hash;
use std::ptr::null_mut;

use super::hierarchy::Hierarchy;
use super::lerping::LerpValue;

use super::clip::Clip;
use super::curve::CurveUntyped;

#[derive(Debug, Clone, Properties)]
pub struct Layer {
    pub weight: f32,
    pub clip_index: u16,
    pub time: f32,
    pub time_scale: f32,
    keyframe: Vec<usize>,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            weight: 1.0,
            clip_index: 0,
            time: 0.0,
            time_scale: 1.0,
            keyframe: vec![],
        }
    }
}

#[derive(Debug, Properties)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,
    #[property(ignore)]
    bind_clips: Vec<ClipBind>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
}

#[derive(Default, Debug)]
struct ClipBind {
    entities: Vec<Option<Entity>>,
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) fn animator_udpate(
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
            .resize_with(animator.clips.len(), ClipBind::default);

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

                let curves_count = clip.len() as usize;

                for layer in &mut animator.layers {
                    if layer.clip_index as usize != clip_index {
                        continue;
                    }

                    // Ensure capacity for cached keyframe index vec
                    if layer.keyframe.len() != curves_count {
                        layer.keyframe.clear();
                        layer
                            .keyframe
                            .resize_with(curves_count, || Default::default());
                    }

                    // Update time
                    let mut time = layer.time + delta_time * layer.time_scale;

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration() {
                            time = (time / clip.duration()).fract() * clip.duration();
                            // Reset all keyframes cached indexes
                            layer
                                .keyframe
                                .iter_mut()
                                .for_each(|x| *x = Default::default())
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

// pub(crate) fn animator_transform_update(
//     clips: Res<Assets<Clip>>,
//     mut animators_query: Query<(&mut Animator,)>,
//     mut transform_query: Query<(&mut Transform,)>,
// ) {
//     const TRANSLATION: Name = Name::from_str("Transform.translation");
//     const ROTATION: Name = Name::from_str("Transform.rotation");
//     const SCALE: Name = Name::from_str("Transform.scale");

//     for (mut animator,) in animators_query.iter_mut() {
//         let animator = &mut *animator;

//         for layer in &mut animator.layers {
//             let w = layer.weight;
//             if w < 1.0e-8 {
//                 continue;
//             }

//             let time = layer.time;
//             let clip_index = layer.clip_index as usize;

//             if let Some(bind) = animator.bind_clips.get(clip_index) {
//                 if let Some(clip_handle) = animator.clips.get(clip_index) {
//                     if let Some(clip) = clips.get(clip_handle) {
//                         // Fetch properties indexes for this particular clip
//                         let mut translation = u16::MAX;
//                         let mut rotation = u16::MAX;
//                         let mut scale = u16::MAX;

//                         for (property_index, prop_name) in clip.properties().iter().enumerate() {
//                             if prop_name == &TRANSLATION {
//                                 translation = property_index as u16;
//                             } else if prop_name == &ROTATION {
//                                 rotation = property_index as u16;
//                             } else if prop_name == &SCALE {
//                                 scale = property_index as u16;
//                             }
//                         }

//                         // if let Some(clip_props) = clip_props.get(clip_handle) {
//                         //     // Update properties
//                         //     for prop in clip_props.get("Transform.translation").unwrap_or(&[]) {
//                         //         if let Some(entity) = bind.entities[prop.entity_index as usize] {
//                         //             let curve_index = prop.curve_index as usize;
//                         //             match &clip.curves[curve_index] {
//                         //                 CurveUntyped::Vec3(curve) => {
//                         //                     // TODO: Expensive query
//                         //                     if let Ok((mut transform,)) =
//                         //                         transform_query.get_mut(entity)
//                         //                     {
//                         //                         let (k, v) = curve.sample_indexed(
//                         //                             layer.keyframe[curve_index],
//                         //                             time,
//                         //                         );
//                         //                         transform.translation = v;
//                         //                         layer.keyframe[curve_index] = k;
//                         //                     }
//                         //                 }
//                         //                 _ => {}
//                         //             }
//                         //         }
//                         //     }

//                         //     for prop in clip_props.get("Transform.rotation").unwrap_or(&[]) {
//                         //         if let Some(entity) = bind.entities[prop.entity_index as usize] {
//                         //             let curve_index = prop.curve_index as usize;
//                         //             match &clip.curves[curve_index] {
//                         //                 CurveUntyped::Quat(curve) => {
//                         //                     // TODO: Expensive query
//                         //                     if let Ok((mut transform,)) =
//                         //                         transform_query.get_mut(entity)
//                         //                     {
//                         //                         let (k, v) = curve.sample_indexed(
//                         //                             layer.keyframe[curve_index],
//                         //                             time,
//                         //                         );
//                         //                         transform.rotation = v;
//                         //                         layer.keyframe[curve_index] = k;
//                         //                     }
//                         //                 }
//                         //                 _ => {}
//                         //             }
//                         //         }
//                         //     }

//                         //     for prop in clip_props.get("Transform.scale").unwrap_or(&[]) {
//                         //         if let Some(entity) = bind.entities[prop.entity_index as usize] {
//                         //             let curve_index = prop.curve_index as usize;
//                         //             match &clip.curves[curve_index] {
//                         //                 CurveUntyped::Vec3(curve) => {
//                         //                     // TODO: Expensive query
//                         //                     if let Ok((mut transform,)) =
//                         //                         transform_query.get_mut(entity)
//                         //                     {
//                         //                         let (k, v) = curve.sample_indexed(
//                         //                             layer.keyframe[curve_index],
//                         //                             time,
//                         //                         );
//                         //                         transform.scale = v;
//                         //                         layer.keyframe[curve_index] = k;
//                         //                     }
//                         //                 }
//                         //                 _ => {}
//                         //             }
//                         //         }
//                         //     }
//                         // }
//                     }
//                 }
//             }
//         }
//     }
// }
