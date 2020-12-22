use anyhow::Result;
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{Asset, AssetEvent, Assets, Handle};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::{Reflect, ReflectComponent, TypeUuid};
use bevy_transform::prelude::*;
use fnv::FnvBuildHasher;
use smallvec::{smallvec, SmallVec};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use crate::blending::AnimatorBlending;
use crate::curve::Curve;
use crate::hierarchy::Hierarchy;
use crate::lerping::Lerp;

// Starting from Intel's Sandy Bridge, spatial prefetcher is now pulling pairs of 64-byte cache
// lines at a time, so we have to align to 128 bytes rather than 64.
//
// Sources:
// - https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-optimization-manual.pdf
// - https://github.com/facebook/folly/blob/1b5288e6eea6df074758f877c849b6e73bbb9fbb/folly/lang/Align.h#L107
//
// ARM's big.LITTLE architecture has asymmetric cores and "big" cores have 128 byte cache line size
// Sources:
// - https://www.mono-project.com/news/2016/09/12/arm64-icache/
//
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const CACHE_SIZE: usize = 128;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const CACHE_SIZE: usize = 64;

/// Defines the number of keyframes that fits inside a cache line;
const KEYFRAMES_PER_CACHE: usize = CACHE_SIZE / std::mem::size_of::<u16>();

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Curves<T> {
    entity_indexes: SmallVec<[u16; 8]>,
    /// Pair of curve and it's index
    curves: Vec<(usize, Curve<T>)>,
}

impl<T> Curves<T> {
    fn calculate_duration(&self) -> f32 {
        self.curves
            .iter()
            .map(|(_, c)| c.duration())
            .fold(0.0f32, |acc, d| acc.max(d))
    }

    /// Number of curves inside
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.curves.len()
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (u16, &(usize, Curve<T>))> {
        self.entity_indexes.iter().copied().zip(self.curves.iter())
    }
}

#[derive(Debug)]
pub struct CurvesUntyped {
    /// Cached calculated curves duration
    duration: f32,
    untyped: Box<dyn Any + Send + Sync + 'static>,
}

impl CurvesUntyped {
    fn new<T: Send + Sync + 'static>(curves: Curves<T>) -> Self {
        CurvesUntyped {
            duration: curves.calculate_duration(),
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

// TODO: impl Serialize, Deserialize using bevy reflect for that
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
    /// Each curve and keyframe cache index mapped by property name  
    properties: HashMap<String, (usize, CurvesUntyped), FnvBuildHasher>,
    /// Number of animated properties
    len: usize,
    /// Number of cache lines currently been used to organize the keyframe
    /// caching into buckets to be accessed by many different threads at the same time
    cache: usize,
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
            len: 0,
            cache: 0,
        }
    }
}

impl Clip {
    /// Property to be animated must be in the following format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    ///
    /// **NOTE** This is a expensive function;
    ///
    /// **NOTE** You can safely ignore the return value as it's only used for assertions during tests;
    /// The return value is the assigned curve index in cache line bucket.
    pub fn add_animated_prop<T>(&mut self, property_path: &str, mut curve: Curve<T>) -> usize
    where
        T: Lerp + Clone + Send + Sync + 'static,
    {
        // Split in entity and attribute path,
        // NOTE: use rfind because it's expected the latter to be generally shorter
        let path =
            property_path.split_at(property_path.rfind('@').expect("property path missing @"));

        let (entity_index, _) = self.hierarchy.get_or_insert_entity(path.0);
        let target_name = path.1.split_at(1).1;

        if let Some((cache_index, curves_untyped)) = self.properties.get_mut(target_name) {
            let curves = curves_untyped
                .downcast_mut::<T>()
                .expect("properties can't have the same name and different curve types");

            // If some entity was created it means this property is a new one so we can safely skip the attribute testing
            if let Some(i) = curves
                .entity_indexes
                .iter()
                .position(|index| *index == entity_index)
            {
                let (curve_index, curve_untyped) = &mut curves.curves[i];
                let curve_index = *curve_index;

                // Found a property equal to the one been inserted, next replace the curve
                std::mem::swap(curve_untyped, &mut curve);

                // Update curve duration in two stages
                let duration = curves.calculate_duration();

                // Drop the down casted ref and update the parent curve
                std::mem::drop(curves);
                curves_untyped.duration = duration;

                // Drop the curves untyped (which as a mut borrow) and update the total duration
                std::mem::drop(curves_untyped);
                self.duration = self.calculate_duration();

                return curve_index;
            } else {
                *cache_index += 1;
                if (*cache_index % KEYFRAMES_PER_CACHE) == 0 {
                    // No more spaces left in the current cache line
                    *cache_index = self.cache * KEYFRAMES_PER_CACHE;
                    self.cache += 1;
                }

                self.len += 1;

                // Append newly added curve
                let duration = curve.duration();

                curves.curves.push((*cache_index, curve));
                curves.entity_indexes.push(entity_index);
                std::mem::drop(curves);

                self.len += 1;
                self.duration = self.duration.max(duration);
                curves_untyped.duration = curves_untyped.duration.max(duration);

                return *cache_index;
            }
        }

        let cache_index = self.cache * KEYFRAMES_PER_CACHE;
        self.cache += 1;
        self.len += 1;

        self.duration = self.duration.max(curve.duration());
        self.properties.insert(
            target_name.to_string(),
            (
                cache_index,
                CurvesUntyped::new(Curves {
                    curves: vec![(cache_index, curve)],
                    entity_indexes: smallvec![entity_index],
                }),
            ),
        );

        cache_index
    }

    /// Number of animated properties in this clip
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Size of the keyframe index array
    #[inline(always)]
    fn keyframes_len(&self) -> usize {
        self.cache * KEYFRAMES_PER_CACHE
    }

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
            .map(|(_, (_, c))| c.duration)
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
        self.properties
            .get(property_name)
            .map(|(_, curve_untyped)| curve_untyped)
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Reflect)]
pub struct Layer {
    pub weight: f32,
    pub clip: usize,
    pub time: f32,
    pub time_scale: f32,
    keyframes: Vec<u16>,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            weight: 1.0,
            clip: 0,
            time: 0.0,
            time_scale: 1.0,
            keyframes: vec![],
        }
    }
}

impl Layer {
    #[inline(always)]
    pub fn keyframes(&self) -> &[u16] {
        &self.keyframes[..]
    }

    #[inline(always)]
    pub fn keyframes_mut(&mut self) -> &mut [u16] {
        &mut self.keyframes[..]
    }

    #[inline(always)]
    pub unsafe fn keyframes_unsafe(&self) -> &mut [u16] {
        std::slice::from_raw_parts_mut(self.keyframes.as_ptr() as *mut _, self.keyframes.len())
    }
}

#[derive(Default, Debug)]
struct Bind {
    entity_indexes: Vec<u16>,
}

#[derive(Debug, Reflect)]
#[reflect(Component)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,

    #[reflect(ignore)]
    bind_clips: Vec<Option<Bind>>,
    #[reflect(ignore)]
    hierarchy: Hierarchy,
    #[reflect(ignore)]
    missing_entities: bool,
    #[reflect(ignore)]
    entities: Vec<Option<Entity>>,

    pub time_scale: f32,
    pub layers: Vec<Layer>,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            clips: vec![],
            bind_clips: vec![],
            hierarchy: Hierarchy::default(),
            missing_entities: false,
            entities: vec![],
            time_scale: 1.0,
            layers: vec![],
        }
    }
}

impl Animator {
    pub fn add_clip(&mut self, clip: Handle<Clip>) -> usize {
        if let Some(i) = self.clips.iter().position(|c| *c == clip) {
            i
        } else {
            // TODO: assert too many clips ...
            let i = self.clips.len();
            self.clips.push(clip);
            i
        }
    }

    // TODO: remove clip

    pub fn add_layer(&mut self, clip: Handle<Clip>, weight: f32) -> usize {
        let clip = self.add_clip(clip);
        let layer_index = self.layers.len();
        self.layers.push(Layer {
            clip,
            weight,
            ..Default::default()
        });
        layer_index
    }

    // TODO: remove layer at index

    // TODO: cleanup, clears hierarchy entities and binds

    #[inline(always)]
    pub fn entities(&self) -> &[Option<Entity>] {
        &self.entities[..]
    }

    #[inline(always)]
    pub fn clips(&self) -> &[Handle<Clip>] {
        &self.clips[..]
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
    type Item = (usize, &'a Layer, &'a Handle<Clip>, &'a [u16]);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Get next layer or stop the iterator
            let layer = self.animator.layers.get(self.index)?;
            let index = self.index;
            self.index += 1;

            let clip_index = layer.clip as usize;
            if let Some(clip_handle) = self.animator.clips.get(clip_index) {
                if let Some(Some(bind)) = self.animator.bind_clips.get(clip_index) {
                    return Some((index, layer, clip_handle, &bind.entity_indexes[..]));
                }

                // Missing clip bind continue to the next layer
                // TODO: log error
            }

            // Invalid clip continue to the next layer
            // TODO: log error
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

/// State info for the system `animator_binding_system`
#[derive(Default)]
pub(crate) struct BindingState {
    clips_event_reader: EventReader<AssetEvent<Clip>>,
    clips_modified: HashSet<Handle<Clip>, FnvBuildHasher>,
}

pub(crate) fn animator_update_system(
    commands: &mut Commands,
    mut state: Local<BindingState>,
    clip_events: Res<Events<AssetEvent<Clip>>>,
    time: Res<Time>,
    clips: Res<Assets<Clip>>,
    mut animators_query: Query<(Entity, &mut Animator, Option<&AnimatorBlending>)>,
    children_query: Query<&Children>,
    name_query: Query<(&Parent, &Name)>,
    parent_or_name_changed_query: Query<
        (Option<&Parent>, &Name),
        Or<(Changed<Parent>, Changed<Name>)>,
    >,
    entity_deleted_query: Query<Entity>,
    parent_or_name_removed_query: Query<Entity, Or<(Without<Parent>, Without<Name>)>>,
) {
    let __span = tracing::info_span!("animator_update_system");
    let __guard = __span.enter();

    // ? NOTE: Changing a clip on fly is supported, but is expensive so use with caution
    // TODO: Put a warning somewhere in the docs on how expensive is changing a clip on the fly
    // Query all clips that changed and remove their binds from the animator
    state.clips_modified.clear();
    for event in state.clips_event_reader.iter(&clip_events) {
        match event {
            AssetEvent::Removed { handle } => state.clips_modified.insert(handle.clone()),
            AssetEvent::Modified { handle } => state.clips_modified.insert(handle.clone()),
            _ => false,
        };
    }

    for (animator_entity, mut animator, animator_blending) in animators_query.iter_mut() {
        let animator = &mut *animator;

        if animator_blending.is_none() {
            commands.insert_one(animator_entity, AnimatorBlending::default());
        }

        // Time scales by component
        let delta_time = time.delta_seconds() * animator.time_scale;

        let w_total = animator
            .layers
            .iter()
            .fold(0.0, |w, layer| w + layer.weight);

        let norm = 1.0 / w_total;

        // Normalize all states weights
        for layer in &mut animator.layers {
            layer.weight *= norm;
        }

        // Invalidate entities on parent or name changed events
        for (entity_index, ((parent_index, name), _)) in animator.hierarchy.iter().enumerate() {
            // Ignore the root entity as we don't care about it's parent nor it's name
            if entity_index == 0 {
                continue;
            }

            let mut remove_entity = false;
            let entity = animator.entities[entity_index];

            if let Some(entity) = entity {
                if entity_deleted_query.get(entity).is_err() {
                    // Entity deleted
                    remove_entity = true;
                } else if parent_or_name_removed_query.get(entity).is_ok() {
                    // Parent or Name component where removed from entity
                    remove_entity = true;
                } else if let Ok((entity_parent, entity_name)) =
                    parent_or_name_changed_query.get(entity)
                {
                    // Parent or Name changed
                    // ? NOTE: Use a changed events for both parent and name because name
                    // ? comparison isn't fast, as result it won't notice changes before the
                    // ? POST_UPDATE stage

                    let parent = animator.entities[*parent_index as usize];
                    if entity_parent.map(|p| p.0) != parent || name != entity_name {
                        remove_entity = true;
                    }
                }
            }

            // Remove entity
            if remove_entity {
                let animator_entities = &mut animator.entities;
                animator
                    .hierarchy
                    .depth_first(entity_index as u16, &mut |index, _| {
                        animator_entities[index as usize] = None;
                    });
                animator.missing_entities = true;
            }
        }

        // TODO: Figure out how expensive this might be with a couple of entities missing
        // Look for missing entities if any
        if animator.missing_entities {
            let mut missing = false;
            let count = animator.hierarchy.len();

            // Prepare the entities table cache
            animator.entities.resize(count, None);
            // Assign the root entity as the first element
            animator.entities[0] = Some(animator_entity);

            // Find missing entities that where just added in the hierarchy
            for entity_index in 0..count {
                missing |= animator
                    .hierarchy
                    .find_entity(
                        entity_index as u16,
                        &mut animator.entities,
                        &children_query,
                        &name_query,
                    )
                    .is_none();
            }

            // Update missing entities state
            animator.missing_entities = missing;

            // TODO: Better warning messages
            //warn!("missing entities");
        }

        // Make run for the binds
        animator
            .bind_clips
            .resize_with(animator.clips.len(), || None);

        for (clip_index, clip_handle) in animator.clips.iter().enumerate() {
            // Invalidate clip binds on clip change
            // NOTE: The clip might be gone so it's necessary to update it here
            if state.clips_modified.contains(clip_handle) {
                animator.bind_clips[clip_index] = None;
                // TODO: warning clip modified while assigned to an animator
            }

            if let Some(clip) = clips.get(clip_handle) {
                let bind_slot = &mut animator.bind_clips[clip_index];
                if bind_slot.is_none() {
                    // Merge newly added clip hierarchy into the animator global hierarchy
                    let mut bind = Bind::default();
                    animator
                        .hierarchy
                        .merge(&clip.hierarchy, &mut bind.entity_indexes);

                    // Prepare the entities table cache
                    animator.entities.resize(animator.hierarchy.len(), None);
                    // Assign the root entity as the first element
                    animator.entities[0] = Some(animator_entity);

                    let mut missing = false;
                    // Find missing entities that where just added in the hierarchy
                    for entity_index in &bind.entity_indexes {
                        missing |= animator
                            .hierarchy
                            .find_entity(
                                *entity_index,
                                &mut animator.entities,
                                &children_query,
                                &name_query,
                            )
                            .is_none();
                    }

                    // Set missing entities if any
                    animator.missing_entities |= missing;

                    *bind_slot = Some(bind);
                };

                for layer in &mut animator.layers {
                    if layer.clip as usize != clip_index {
                        continue;
                    }

                    // Update time
                    let mut time = layer.time + delta_time * layer.time_scale;

                    // Ensure keyframes capacity
                    layer.keyframes.resize(clip.keyframes_len(), 0);

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration() {
                            time = (time / clip.duration()).fract() * clip.duration();
                            // Reset keyframes indexes (speedup sampling)
                            layer.keyframes.iter_mut().for_each(|k| *k = 0);
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

    std::mem::drop(__guard);
}

// ! FIXME: Theres no way of free memory used by the `Local<AnimatorBlending>` in each system

pub trait AnimatedComponent: Component + Sized {
    fn animator_update_system(
        clips: Res<Assets<Clip>>,
        animator_blending: Local<AnimatorBlending>,
        animators_query: Query<&Animator>,
        component_query: Query<&mut Self>,
    );
}

pub trait AnimatedAsset: Asset + Sized {
    fn animator_update_system(
        clips: Res<Assets<Clip>>,
        animator_blending: Local<AnimatorBlending>,
        animators_query: Query<&Animator>,
        assets: ResMut<Assets<Self>>,
        component_query: Query<&mut Handle<Self>>,
    );
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use bevy_asset::AddAsset;
    use bevy_math::prelude::{Quat, Vec3};
    use bevy_pbr::prelude::StandardMaterial;
    use bevy_render::prelude::Mesh;

    struct AnimatorTestBench {
        app: bevy_app::App,
        entities: Vec<Entity>,
    }

    impl AnimatorTestBench {
        fn new() -> Self {
            let mut app_builder = bevy_app::App::build();
            app_builder
                .add_plugin(bevy_reflect::ReflectPlugin::default())
                .add_plugin(bevy_core::CorePlugin::default())
                .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
                .add_plugin(bevy_asset::AssetPlugin::default())
                .add_asset::<Mesh>()
                .add_asset::<StandardMaterial>()
                .add_plugin(bevy_transform::TransformPlugin::default())
                .add_plugin(crate::AnimationPlugin::default());

            let mut world = World::new();
            let mut world_builder = world.build();
            let base = (
                GlobalTransform::default(),
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            );

            // Create animator and assign some clips
            let mut animator = Animator::default();
            {
                let mut clip_a = Clip::default();
                clip_a.add_animated_prop(
                    "@Transform.translation",
                    Curve::from_linear(0.0, 1.0, Vec3::unit_x(), -Vec3::unit_x()),
                );
                let rot = Curve::from_constant(Quat::identity());
                clip_a.add_animated_prop("@Transform.rotation", rot.clone());
                clip_a.add_animated_prop("/Node1@Transform.rotation", rot.clone());
                clip_a.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

                let mut clip_b = Clip::default();
                clip_b.add_animated_prop(
                    "@Transform.translation",
                    Curve::from_constant(Vec3::zero()),
                );
                let rot = Curve::from_linear(
                    0.0,
                    1.0,
                    Quat::from_axis_angle(Vec3::unit_z(), 0.1),
                    Quat::from_axis_angle(Vec3::unit_z(), -0.1),
                );
                clip_b.add_animated_prop("@Transform.rotation", rot.clone());
                clip_b.add_animated_prop("/Node1@Transform.rotation", rot.clone());
                clip_b.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

                let mut clips = app_builder
                    .resources_mut()
                    .get_mut::<Assets<Clip>>()
                    .unwrap();
                let clip_a = clips.add(clip_a);
                let clip_b = clips.add(clip_b);

                animator.add_layer(clip_a, 0.5);
                animator.add_layer(clip_b, 0.5);
            }

            let mut entities = vec![];
            entities.push(
                world_builder
                    .spawn(base.clone())
                    .with(Name::from_str("Root"))
                    .with(animator)
                    .current_entity
                    .unwrap(),
            );
            world_builder.with_children(|world_builder| {
                entities.push(
                    world_builder
                        .spawn(base.clone())
                        .with(Name::from_str("Node1"))
                        .current_entity()
                        .unwrap(),
                );

                world_builder.with_children(|world_builder| {
                    entities.push(
                        world_builder
                            .spawn(base.clone())
                            .with(Name::from_str("Node2"))
                            .current_entity()
                            .unwrap(),
                    );

                    world_builder.with_children(|world_builder| {
                        entities.push(
                            world_builder
                                .spawn(base.clone())
                                .with(Name::from_str("Node3"))
                                .current_entity()
                                .unwrap(),
                        );
                    });
                });
            });

            app_builder.set_world(world);
            app_builder.app.update();

            Self {
                app: app_builder.app,
                entities,
            }
        }

        fn run(&mut self) {
            self.app.update();
        }

        fn animator(&mut self) -> Mut<Animator> {
            self.app
                .world
                .get_mut::<Animator>(self.entities[0])
                .unwrap()
        }
    }

    #[test]
    fn entity_deleted() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        // delete "Node1"
        test_bench
            .app
            .world
            .despawn(test_bench.entities[1])
            .unwrap();

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        // Re-add "Node1"
        test_bench.entities[1] = test_bench
            .app
            .world
            .build()
            .spawn((
                GlobalTransform::default(),
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                Name::from_str("Node1"),
                Parent(test_bench.entities[0]),
            ))
            .current_entity
            .unwrap();

        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[2])
            .unwrap() = Parent(test_bench.entities[1]);

        // Tick
        // ! FIXME: May take one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_parent_changed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        // Change parent of node "Node1"
        let new_parent = test_bench
            .app
            .world
            .build()
            .spawn(())
            .current_entity
            .unwrap();

        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[1])
            .unwrap() = Parent(new_parent);

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        // Re-parent "Node1"
        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[1])
            .unwrap() = Parent(test_bench.entities[0]);

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_parent_component_removed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        test_bench
            .app
            .world
            .remove_one::<Parent>(test_bench.entities[1])
            .unwrap();

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        // Re-parent "Node1"
        test_bench
            .app
            .world
            .insert_one(test_bench.entities[1], Parent(test_bench.entities[0]))
            .unwrap();

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_renamed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        *test_bench
            .app
            .world
            .get_mut::<Name>(test_bench.entities[1])
            .unwrap() = Name::from_str("Spine1");

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        *test_bench
            .app
            .world
            .get_mut::<Name>(test_bench.entities[1])
            .unwrap() = Name::from_str("Node1");

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_name_component_removed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        test_bench
            .app
            .world
            .remove_one::<Name>(test_bench.entities[1])
            .unwrap();

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        test_bench
            .app
            .world
            .insert_one(test_bench.entities[1], Name::from_str("Node1"))
            .unwrap();

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn clip_changed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        test_bench.run();

        // Modify clip
        {
            let clip_a_handle = test_bench.animator().clips()[0].clone();
            let mut clips = test_bench.app.resources.get_mut::<Assets<Clip>>().unwrap();
            let clip_a = clips.get_mut(clip_a_handle).unwrap();
            clip_a.add_animated_prop(
                "/Node3/@Transform.translation",
                Curve::from_linear(0.0, 1.0, Vec3::unit_z(), -Vec3::unit_z()),
            );
        }

        test_bench.run();

        assert_eq!(test_bench.animator().entities().len(), 4);

        // Spawn entity
        test_bench.app.world.build().spawn((
            GlobalTransform::default(),
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            Name::from_str("Node3"),
            Parent(test_bench.entities[0]),
        ));

        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn clip_property_index_are_grouped_into_cache_line_buckets() {
        let curve = Curve::from_constant(0.0);
        let mut clip = Clip::default();
        assert_eq!(clip.add_animated_prop("a@T.t", curve.clone()), 0);
        assert_eq!(clip.add_animated_prop("b@T.t", curve.clone()), 1);
        assert_eq!(
            clip.add_animated_prop("a@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE
        );
        assert_eq!(clip.add_animated_prop("c@T.t", curve.clone()), 2);
        assert_eq!(
            clip.add_animated_prop("b@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE + 1
        );
        assert_eq!(
            clip.add_animated_prop("c@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE + 2
        );
        assert_eq!(clip.add_animated_prop("d@T.t", curve.clone()), 3);

        for i in 4..KEYFRAMES_PER_CACHE {
            assert_eq!(
                clip.add_animated_prop(&format!("node_{}@T.t", i), curve.clone()),
                i
            );
        }

        assert_eq!(
            clip.add_animated_prop("za@T.t", curve.clone()),
            KEYFRAMES_PER_CACHE * 2
        );
        assert_eq!(
            clip.add_animated_prop("za@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE + 3
        );
        assert_eq!(
            clip.add_animated_prop("zb@T.t", curve.clone()),
            KEYFRAMES_PER_CACHE * 2 + 1
        );
    }
}
