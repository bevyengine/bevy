use anyhow::Result;
use bevy_asset::{Assets, Handle /*HandleUntyped*/};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_property::Properties;
use bevy_transform::prelude::*;
use bevy_type_registry::TypeUuid;
use fnv::FnvBuildHasher;
use smallvec::{smallvec, SmallVec};
use std::any::Any;
use std::collections::HashMap;

use crate::blending::{AnimatorBlending, Blend};
use crate::curve::Curve;
use crate::hierarchy::Hierarchy;
use crate::lerping::Lerp;

#[derive(Debug)]
pub struct Curves<T> {
    id: usize,
    /// Maps each curve to an entity index or other value
    indexes: SmallVec<[u16; 8]>,
    curves: Vec<Curve<T>>,
}

impl<T> Curves<T> {
    fn calculate_duration(&self) -> f32 {
        self.curves
            .iter()
            .map(|c| c.duration())
            .fold(0.0f32, |acc, d| acc.max(d))
    }

    /// Number of curves inside
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
    /// Cached calculated curves duration
    duration: f32,
    untyped: Box<dyn Any + 'static>,
}

// SAFETY: CurvesUntyped will only hold on to Curves<T> which also implement Send and Sync
unsafe impl Send for CurvesUntyped {}
unsafe impl Sync for CurvesUntyped {}

impl CurvesUntyped {
    fn new<T: Send + Sync + 'static>(curves: Curves<T>) -> Self {
        CurvesUntyped {
            duration: curves
                .iter()
                .map(|(_, c)| c.duration())
                .fold(0.0, |acc, d| acc.max(d)),
            untyped: Box::new(curves),
        }
    }

    #[inline(always)]
    pub fn duration(&self) -> f32 {
        self.duration
    }

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
    /// Should this clip loop (warping around) or hold
    ///
    /// **NOTE** Keep in mind that sampling with time greater
    /// than the clips duration will always hold.
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    /// Clip compound duration
    duration: f32,
    hierarchy: Hierarchy,
    // ? NOTE: AHash performed worse than FnvHasher
    properties: HashMap<String, CurvesUntyped, FnvBuildHasher>,
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
        T: Lerp + Clone + Send + Sync + 'static,
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
                let duration = curves.calculate_duration();

                // Drop the down casted ref and update the parent curve
                std::mem::drop(curves);
                curves_untyped.duration = duration;

                // Drop the curves untyped (which as a mut borrow) and update the total duration
                std::mem::drop(curves_untyped);
                self.duration = self.calculate_duration();
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
            CurvesUntyped::new(Curves {
                id,
                curves: vec![curve],
                indexes: smallvec![entity_index],
            }),
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

    fn calculate_duration(&self) -> f32 {
        self.properties
            .iter()
            .map(|(_, c)| c.duration)
            .fold(0.0f32, |acc, x| acc.max(x))
    }

    /// Clip duration
    #[inline(always)]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[inline(always)]
    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hierarchy
    }

    /// Get the curves an given property
    #[inline(always)]
    pub fn get(&self, property_name: &str) -> Option<&CurvesUntyped> {
        self.properties.get(property_name)
    }
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

                // TODO: Don't look for the entities every frame, only when something happens
                // TODO: Handle parent changed events
                // TODO: Handle name changed events
                // TODO: Merge newly added clips hierarchies into a single one

                // Prepare the entities table cache
                bind.entities.clear();
                bind.entities.resize(clip.hierarchy().len(), None);
                // Assign the root entity as the first element
                bind.entities[0] = Some(animator_entity);

                // Find entities ...
                for entity_index in 1..clip.hierarchy().len() {
                    clip.hierarchy().find_entity(
                        entity_index as u16,
                        &mut bind.entities,
                        &mut children_query,
                        &mut name_query,
                    );
                }

                for layer in &mut animator.layers {
                    if layer.clip as usize != clip_index {
                        continue;
                    }

                    // Update time
                    let mut time = layer.time + delta_time * layer.time_scale;

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration() {
                            time = (time / clip.duration()).fract() * clip.duration();
                            // TODO: Reset all keyframes cached indexes (speedup)
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

// TODO: This should be auto derived using the "Animated" trait that just returns
// a system able to animate the said component. Use "AnimatedAsset" for asset handles!

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

                // TODO: Merge all the clips hierarchy into a single bigger one
                // and do the component feching once per animator, instead of per clip

                // TODO: Lazly get only the needed entities ?! (it will be bad for missing entities components)

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
                    .get("Transform.translation")
                    .map(|curve_untyped| curve_untyped.downcast_ref::<Vec3>())
                    .flatten()
                {
                    // Get keyframes and ensure capacity
                    let keyframes = keyframe_cache.get(curves.id);
                    keyframes.resize(curves.len() as usize, 0);

                    for (curve_index, (entity_index, curve)) in curves.iter().enumerate() {
                        if let Some(ref mut component) = components[entity_index as usize] {
                            // TODO: I'm not nocing any discernible peformance change from using just `sample`
                            let (k, v) = curve.sample_indexed(keyframes[curve_index], time);
                            keyframes[curve_index] = k;
                            // let v = curve.sample(time);
                            component.translation.blend(&mut blend_group, v, w);
                        }
                    }
                }

                // ~23us
                if let Some(curves) = clip
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
                            component.rotation.blend(&mut blend_group, v, w);
                        }
                    }
                }

                // ~23us
                if let Some(curves) = clip
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
                            component.scale.blend(&mut blend_group, v, w);
                        }
                    }
                }
            }
        }
    }
}

// #[cfg(test)]
// #[allow(dead_code)]
// mod tests {
//     use super::*;
//     use crate::curve::Curve;
//     use bevy_ecs::{ArchetypeComponent, TypeAccess};

//     struct AnimatorTestBench {
//         app: bevy_app::App,
//         entities: Vec<Entity>,
//         schedule: bevy_ecs::Schedule,
//     }

//     impl AnimatorTestBench {
//         fn new() -> Self {
//             let mut app_builder = bevy_app::App::build();
//             app_builder
//                 .add_plugin(bevy_type_registry::TypeRegistryPlugin::default())
//                 .add_plugin(bevy_core::CorePlugin::default())
//                 .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
//                 .add_plugin(bevy_asset::AssetPlugin)
//                 .add_plugin(bevy_transform::TransformPlugin)
//                 .add_plugin(crate::AnimationPlugin);

//             let mut world = World::new();
//             let mut world_builder = world.build();
//             let base = (
//                 GlobalTransform::default(),
//                 Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
//             );

//             // Create animator and assign some clips
//             let mut animator = Animator::default();
//             {
//                 let mut clip_a = Clip::default();
//                 clip_a.add_animated_prop(
//                     "@Transform.translation",
//                     Curve::from_linear(0.0, 1.0, Vec3::unit_x(), -Vec3::unit_x()),
//                 );
//                 let rot = Curve::from_constant(Quat::identity());
//                 clip_a.add_animated_prop("@Transform.rotation", rot.clone());
//                 clip_a.add_animated_prop("/Node1@Transform.rotation", rot.clone());
//                 clip_a.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

//                 let mut clip_b = Clip::default();
//                 clip_b.add_animated_prop(
//                     "@Transform.translation",
//                     Curve::from_constant(Vec3::zero()),
//                 );
//                 let rot = Curve::from_linear(
//                     0.0,
//                     1.0,
//                     Quat::from_axis_angle(Vec3::unit_z(), 0.1),
//                     Quat::from_axis_angle(Vec3::unit_z(), -0.1),
//                 );
//                 clip_b.add_animated_prop("@Transform.rotation", rot.clone());
//                 clip_b.add_animated_prop("/Node1@Transform.rotation", rot.clone());
//                 clip_b.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

//                 let mut clips = app_builder
//                     .resources_mut()
//                     .get_mut::<Assets<Clip>>()
//                     .unwrap();
//                 let clip_a = clips.add(clip_a);
//                 let clip_b = clips.add(clip_b);

//                 animator.add_layer(clip_a, 0.5);
//                 animator.add_layer(clip_b, 0.5);
//             }

//             let mut entities = vec![];
//             entities.push(
//                 world_builder
//                     .spawn(base.clone())
//                     .with(Name::from_str("Root"))
//                     .with(animator)
//                     .current_entity
//                     .unwrap(),
//             );
//             world_builder.with_children(|world_builder| {
//                 entities.push(
//                     world_builder
//                         .spawn(base.clone())
//                         .with(Name::from_str("Node1"))
//                         .current_entity()
//                         .unwrap(),
//                 );

//                 world_builder.with_children(|world_builder| {
//                     entities.push(
//                         world_builder
//                             .spawn(base.clone())
//                             .with(Name::from_str("Node2"))
//                             .current_entity()
//                             .unwrap(),
//                     );

//                     world_builder.with_children(|world_builder| {
//                         entities.push(
//                             world_builder
//                                 .spawn(base.clone())
//                                 .with(Name::from_str("Node3"))
//                                 .current_entity()
//                                 .unwrap(),
//                         );
//                     });
//                 });
//             });

//             app_builder.set_world(world);

//             let mut schedule = bevy_ecs::Schedule::default();
//             schedule.add_stage("update");
//             schedule.add_stage_after("update", "post_update");
//             schedule.add_system_to_stage("update", animator_update_system);
//             schedule.add_system_to_stage("update", animator_transform_update_system);
//             schedule.add_system_to_stage("post_update", parent_update_system);
//             //schedule.add_system_to_stage("update", transform_propagate_system);

//             schedule.initialize(&mut app_builder.app.world, &mut app_builder.app.resources);
//             schedule.run(&mut app_builder.app.world, &mut app_builder.app.resources);

//             Self {
//                 app: app_builder.app,
//                 entities,
//                 schedule,
//             }
//         }

//         fn run(&mut self) {
//             self.schedule
//                 .run(&mut self.app.world, &mut self.app.resources);
//         }
//     }

//     #[test]
//     #[cfg(feature = "extra-profiling-tests")]
//     fn test_bench_update() {
//         // ? NOTE: Mimics a basic system update behavior good for pref since criterion will pollute the
//         // ? annotations with many expensive instructions
//         let mut test_bench = AnimatorTestBench::new();
//         test_bench.run();
//         test_bench.run();

//         // let mut schedule = bevy_ecs::Schedule::default();
//         // schedule.add_stage("update");
//         // schedule.add_system_to_stage("update", animator_transform_update_system);
//         // schedule.initialize(&mut test_bench.app.world, &mut test_bench.app.resources);

//         // let mut transform_system: Box<dyn System<Input = (), Output = ()>> =
//         //     Box::new(animator_transform_update_system.system());

//         // transform_system.initialize(&mut test_bench.app.world, &mut test_bench.app.resources);

//         // fn animator_transform_update_system(
//         //     clips: Res<Assets<Clip>>,
//         //     mut animators_query: Query<(&Animator, &mut KeyframeCache, &mut AnimatorBlending)>,
//         //     transform_query: Query<(&mut Transform,)>,
//         // );

//         let type_access = <TypeAccess<ArchetypeComponent>>::new(vec![], vec![]);
//         for _ in 0..100_000 {
//             // // Time tick
//             // {
//             //     let mut time = test_bench.app.resources.get_mut::<Time>().unwrap();
//             //     time.delta_seconds += 0.016;
//             //     time.delta_seconds_f64 += 0.016;
//             // }

//             //schedule.run(&mut test_bench.app.world, &mut test_bench.app.resources);

//             //transform_system.run((), &mut test_bench.app.world, &mut test_bench.app.resources);

//             // Fetching
//             let clips = &*test_bench.app.resources.get::<Assets<Clip>>().unwrap();
//             let clips =
//                 unsafe { Res::new(std::ptr::NonNull::new(clips as *const _ as *mut _).unwrap()) };
//             let animators_query = unsafe {
//                 <Query<(&Animator, &mut KeyframeCache, &mut AnimatorBlending)>>::new(
//                     &test_bench.app.world,
//                     &type_access,
//                 )
//             };
//             let transform_query =
//                 unsafe { <Query<(&mut Transform,)>>::new(&test_bench.app.world, &type_access) };

//             // Running
//             animator_transform_update_system(clips, animators_query, transform_query);
//         }
//     }
// }
