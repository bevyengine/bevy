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
use bevy_utils::HashSet;
//use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::ptr::null_mut;

use super::lerping::LerpValue;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

/// Used to make fast string comparisons, don't serialize
#[derive(Debug, PartialEq, Eq)]
struct HashCode(u64);

// TODO: impl Serialize, Deserialize
#[derive(Debug, TypeUuid)]
#[uuid = "4c76e6c3-706d-4a74-af8e-4f48033e0733"]
pub struct Clip {
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    duration: f32,
    /// Entity identification parent index on this same vec and name
    entities: Vec<(u16, String)>,
    /// Attribute is made by the entity index and a string that combines
    /// component name followed by their attributes spaced by a period,
    /// like so: `"Transform.translation.x"`
    attribute: Vec<(u16, String, HashCode)>,
    curves: Vec<CurveUntyped>,
}

// fn clip_default_warp() -> bool {
//     true
// }

impl Default for Clip {
    fn default() -> Self {
        Self {
            warp: true,
            duration: 0.0,
            // NOTE: Since the root has no parent in this context it points to a place outside the vec bounds
            entities: vec![(u16::MAX, String::default())],
            attribute: vec![],
            curves: vec![],
        }
    }
}

impl Clip {
    /// Property to be animated must be in the following format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    pub fn add_animated_prop(&mut self, property_path: &str, mut curve: CurveUntyped) {
        // Clip an only have some amount of curves and entities
        // this limitation was added to save memory (but you can increase it if you want)
        assert!(
            (self.curves.len() as u16) <= u16::MAX,
            "curve limit reached"
        );

        let path =
            property_path.split_at(property_path.rfind('@').expect("property path missing @"));

        let mut entity_created = false;
        let mut entity = 0; // Start search from root
        for name in path.0.split('/') {
            // Ignore the first '/' or '///'
            if name.is_empty() {
                continue;
            }

            if let Some(e) = self
                .entities
                .iter()
                .position(|(p, n)| (*p, n.as_str()) == (entity, name))
            {
                // Found entity
                entity = e as u16;
            } else {
                // Add entity
                let e = self.entities.len();
                self.entities.push((entity, name.to_string()));
                entity_created = true;
                // Soft limit added to save memory, identical to the curve limit
                entity = u16::try_from(e).expect("entities limit reached");
            }
        }

        let property = path.1.split_at(1).1;

        // Faster comparison
        let mut hasher = DefaultHasher::default();
        property.hash(&mut hasher);
        let hash = HashCode(hasher.finish());

        // If some entity was created it means this property is a new one
        if !entity_created {
            for (i, attr) in self.attribute.iter().enumerate() {
                if attr.0 != entity {
                    continue;
                }

                // NOTE: "...@Transform.translation" and "...@Transform.translation.x" can't live with each other
                // but since Properties aren't implemented for f32, Vec2, Vec3, Vec4 and Quat
                // (all relevant animated type values) the check rule could be simplified (a lot)
                //
                // The test case `test::unhandled_properties_implementation_for_types` will ensure that

                if hash == attr.2 && property == attr.1 {
                    // Found a property are equal the one been inserted
                    // Replace curve, the property was already added, this is very important
                    // because it guarantees that each property will have unique access to some
                    // attribute during the update stages

                    let inserted_duration = curve.duration();
                    std::mem::swap(&mut self.curves[i], &mut curve);
                    self.update_duration(curve.duration(), inserted_duration);

                    return;
                }
            }
        }

        self.duration = self.duration.max(curve.duration());
        self.attribute.push((entity, property.to_owned(), hash));
        self.curves.push(curve);
    }

    /// Number of animated properties in this clip
    pub fn len(&self) -> u16 {
        self.curves.len() as u16
    }

    /// Returns the property curve property path.
    ///
    /// The clip stores a property path in a specific way to improve search performance
    /// thus it needs to rebuilt the curve property path in the human readable format
    pub fn get_property_path(&self, index: u16) -> String {
        let (entity, name, _) = &self.attribute[index as usize];
        let mut path = format!("@{}", name);

        let mut first = true;
        let mut entity = *entity;
        while let Some((parent, name)) = self.entities.get(entity as usize) {
            if first {
                path = format!("{}{}", name, path);
                first = false;
            } else {
                path = format!("{}/{}", name, path);
            }
            entity = *parent;
        }

        path
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    fn update_duration(&mut self, removed_duration: f32, inserted_duration: f32) {
        if removed_duration == inserted_duration { // Precise match
             // Nothing left todo
        } else if float_cmp::approx_eq!(f32, removed_duration, self.duration, ulps = 2) {
            // TODO: Review this approximated comparison

            // Heavy path, the clip duration needs
            self.duration = self
                .curves
                .iter()
                .map(|c| c.duration())
                .fold(0.0, |acc, x| acc.max(x));
        } else {
            self.duration = self.duration.max(inserted_duration);
        }
    }
}

// TODO: impl Serialize, Deserialize
#[derive(Debug)]
pub enum CurveUntyped {
    Float(Curve<f32>),
    Vec3(Curve<Vec3>),
    Vec4(Curve<Vec4>),
    Quat(Curve<Quat>),
    //Handle(Curve<HandleUntyped>), // TODO: Requires (de)serialize!
}

macro_rules! untyped_fn {
    ($v:vis fn $name:ident ( &self, $( $arg:ident : $arg_ty:ty ,)* ) $(-> $ret:ty)* ) => {
        $v fn $name(&self, $( $arg : $arg_ty ),*) $(-> $ret)* {
            match self {
                CurveUntyped::Float(v) => v.$name($($arg,)*),
                CurveUntyped::Vec3(v) => v.$name($($arg,)*),
                CurveUntyped::Vec4(v) => v.$name($($arg,)*),
                CurveUntyped::Quat(v) => v.$name($($arg,)*),
            }
        }
    };
}

impl CurveUntyped {
    untyped_fn!(pub fn duration(&self,) -> f32);
    untyped_fn!(pub fn value_type(&self,) -> TypeId);

    pub fn add_time_offset(&mut self, time: f32) {
        match self {
            CurveUntyped::Float(v) => v.add_offset_time(time),
            CurveUntyped::Vec3(v) => v.add_offset_time(time),
            CurveUntyped::Vec4(v) => v.add_offset_time(time),
            CurveUntyped::Quat(v) => v.add_offset_time(time),
        }
    }
}

// TODO: impl Serialize, Deserialize
#[derive(Default, Debug)]
pub struct Curve<T> {
    // TODO: Linear and Spline variants
    samples: Vec<f32>,
    values: Vec<T>,
}

impl<T> Curve<T>
where
    T: LerpValue + Clone + 'static,
{
    pub fn new(samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Result?

        // Make sure both have the same length
        assert!(
            samples.len() == values.len(),
            "samples and values must have the same length"
        );

        // Make sure the
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a < b),
            "time samples must be on ascending order"
        );
        Self { samples, values }
    }

    pub fn from_linear(t0: f32, t1: f32, v0: T, v1: T) -> Self {
        Self {
            samples: if t1 >= t0 { vec![t0, t1] } else { vec![t1, t0] },
            values: vec![v0, v1],
        }
    }

    pub fn duration(&self) -> f32 {
        self.samples.last().copied().unwrap_or(0.0)
    }

    /// Easer to use sampling method that don't have time restrictions or needs
    /// the keyframe index, but is more expensive always `O(n)`. Which means
    /// sampling takes longer to evaluate as much as time get closer to curve duration
    /// and it get worse with more keyframes.
    pub fn sample(&self, time: f32) -> T {
        self.sample_forward(0, time).1
    }

    /// Samples the curve starting from some keyframe index, this make the common case `O(1)`,
    /// use only when time advancing forwards
    pub fn sample_forward(&self, mut index: usize, time: f32) -> (usize, T) {
        // Adjust for the current keyframe index
        let last_index = self.samples.len() - 1;
        loop {
            if index > last_index {
                return (last_index, self.values.last().unwrap().clone());
            }

            if self.samples[index] < time {
                index += 1;
            } else {
                break;
            }
        }

        let value = if index == 0 {
            self.values[0].clone()
        } else {
            // Lerp the value
            let i = index - 1;
            let previous_time = self.samples[i];
            let t = (time - previous_time) / (self.samples[index] - previous_time);
            //println!("{} => ({}, {}) => {}", time, i, index, t);
            //let t = t.max(0.0).min(1.0);
            debug_assert!(t >= 0.0 && t <= 1.0); // Checks if it's required to normalize t
            T::lerp(&self.values[i], &self.values[index], t)
        };

        (index, value)
    }

    pub fn add_offset_time(&mut self, time_offset: f32) {
        self.samples.iter_mut().for_each(|t| *t += time_offset);
    }

    // pub fn insert(&mut self, time_sample: f32, value: T) {
    // }

    // pub fn remove(&mut self, index: usize) {
    // }

    pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
        self.samples.iter().copied().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (f32, &mut T)> {
        self.samples.iter().copied().zip(self.values.iter_mut())
    }

    pub fn value_type(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

#[derive(Default, Debug, Clone, Properties)]
pub struct Layer {
    pub weight: f32,
    pub clip: u16,
    pub time: f32,
    //pub time_scale: f32, // TODO: Add time_scale
    keyframe: Vec<usize>,
}

#[derive(Debug, Properties)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,
    #[property(ignore)]
    binds: Vec<Option<Binds>>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
    // TODO: Transitions to simplify layer blending
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            clips: vec![],
            binds: vec![],
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

    pub fn clips_len(&self) -> u16 {
        self.clips.len() as u16
    }
}

// TODO: Bind asset properties (edge case), offset

#[derive(Debug)]
struct Binds {
    metadata: Vec<BindMeta>,
    curves: Vec<u16>,
    attributes: Vec<Ptr>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Ptr(*mut u8);

// SAFETY: Store pointers to each attribute to be updated, a clip can't have two pointers
// with the same value. Each clip per Animator will be updated sequentially
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

/// Marker type for missing component
struct MissingComponentMarker;

/// Meta data is used to check the bind validity
#[derive(Debug)]
struct BindMeta {
    entity_index: u16,
    inner: Option<BindInner>,
    range: (u16, u16),
}

#[derive(Debug)]
struct BindInner {
    parent: Option<Entity>,
    entity: Entity,
    location: Location,
    component: TypeId,
}

#[derive(Default)]
struct ClipResourceProviderState {
    clip_event_reader: EventReader<AssetEvent<Clip>>,
}

// TODO: Refactor in hope to reuse more code (maybe just create more functions)
#[tracing::instrument(skip(world, resources))]
pub(crate) fn animator_binding_system(world: &mut World, resources: &mut Resources) {
    let system_id = SystemId(0); // ? NOTE: shouldn't be required to match the system id

    // Fetch resources
    let mut state = {
        let state = resources.get_local_mut::<ClipResourceProviderState>(system_id);
        if state.is_none() {
            std::mem::drop(state); // ? NOTE: Makes the borrow checker happy
            resources.insert_local(system_id, ClipResourceProviderState::default());
            resources
                .get_local_mut::<ClipResourceProviderState>(system_id)
                .unwrap()
        } else {
            state.unwrap()
        }
    };

    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let type_registry_read_guard = type_registry.component.read();

    let clips = resources.get::<Assets<Clip>>().unwrap();
    let clip_events = resources.get::<Events<AssetEvent<Clip>>>().unwrap();

    // Builds query
    // SAFETY: This is the only query with mutable reference the Animator component
    let animator_query = unsafe { world.query_unchecked::<(Entity, &mut Animator), ()>() };

    // ? NOTE: Changing a clip on fly is supported, but very expensive so use with caution
    // Query all clips that changed and remove their binds from the animator
    let mut modified = HashSet::default();
    for event in state.clip_event_reader.iter(&clip_events) {
        match event {
            AssetEvent::Removed { handle } => modified.insert(handle),
            AssetEvent::Modified { handle } => modified.insert(handle),
            _ => false,
        };
    }

    for (root, mut animator) in animator_query {
        let animator = &mut *animator; // Make sure the borrow checker is happy

        // Make room for binds or deallocate unused
        animator.binds.resize_with(animator.clips.len(), || None);

        // Used for entity translation
        let mut entities_table_cache = vec![];

        for (clip_handle, bind) in animator.clips.iter().zip(animator.binds.iter_mut()) {
            // Invalidate binds for clips that where modified
            if modified.contains(clip_handle) {
                *bind = None;
            }

            if let Some(clip) = clips.get(clip_handle) {
                // Prepare the entities table cache
                entities_table_cache.clear();
                entities_table_cache.resize(clip.entities.len(), None);
                // Assign the root entity as the first element
                entities_table_cache[0] = Some(root);

                if bind.is_none() {
                    // Build binds from scratch
                    // Allocate the minium enough of memory on right at the begin
                    let mut b = Binds {
                        metadata: Vec::with_capacity(clip.entities.len()),
                        curves: Vec::with_capacity(clip.attribute.len()),
                        attributes: Vec::with_capacity(clip.attribute.len()),
                    };

                    let mut range = (0, 0);
                    let mut prev_partial_info = None;
                    let mut prev_component = TypeId::of::<MissingComponentMarker>();
                    let mut prev_component_short_name = "";

                    for (curve_index, (entity_index, attr_path, _)) in
                        clip.attribute.iter().enumerate()
                    {
                        let mut commit = false;
                        let mut partial_info = None;
                        let mut component = TypeId::of::<MissingComponentMarker>();

                        // Query component by name
                        let mut path = attr_path.split('.');
                        let component_short_name =
                            path.next().expect("missing component short name");

                        if let Some(entity) = find_entity(
                            *entity_index,
                            &clip.entities,
                            &mut entities_table_cache,
                            &world,
                        ) {
                            let location = world.get_entity_location(entity).unwrap();

                            // Commit is needed
                            partial_info = Some((entity, location));
                            if prev_partial_info != partial_info {
                                commit = true;
                            }

                            if let Some(component_reg) =
                                type_registry_read_guard.get_with_short_name(component_short_name)
                            {
                                component = component_reg.ty;

                                let root_props = component_reg.get_component_properties(
                                    &world.archetypes[location.archetype as usize],
                                    location.index,
                                );

                                b.curves.push(curve_index as u16);
                                b.attributes.push(find_property_ptr(
                                    path,
                                    root_props,
                                    clip.curves[curve_index].value_type(),
                                ));

                                // Group by component type
                                commit |= prev_component != component;
                            } else {
                                // Missing component
                                b.curves.push(curve_index as u16);
                                b.attributes.push(Ptr(null_mut()));

                                // Group by component name
                                commit |= prev_component_short_name != component_short_name;
                            }
                        } else {
                            // Missing entity
                            commit = prev_partial_info != partial_info;
                            commit |= prev_component_short_name != component_short_name;

                            b.curves.push(curve_index as u16);
                            b.attributes.push(Ptr(null_mut()));
                        }

                        if commit {
                            // Close range
                            range.1 = b.curves.len() as u16 - 1;

                            // Commit meta
                            b.metadata.push(BindMeta {
                                entity_index: *entity_index,
                                inner: prev_partial_info.map(|(entity, location)| BindInner {
                                    // An entity will always have parent unless is the root entity
                                    parent: entities_table_cache
                                        .get(clip.entities[*entity_index as usize].0 as usize)
                                        .copied()
                                        .flatten(),
                                    entity,
                                    location,
                                    component,
                                }),
                                range,
                            });

                            // Jump to the next range
                            prev_partial_info = partial_info;
                            prev_component = component;
                            prev_component_short_name = component_short_name;
                            range.0 = range.1;
                            range.1 += 1;
                        }
                    }

                    *bind = Some(b);

                    // No need to continue because binds are fresh
                    continue;
                }

                // Check bind state and update the necessary parts
                let bind = bind.as_mut().unwrap();
                for meta in bind.metadata.iter_mut() {
                    if meta.inner.is_none() {
                        // TODO: Handle missing entity
                        continue;
                    }

                    let inner = meta.inner.as_mut().unwrap();

                    // Handle `Parent` changed
                    if let Some(parent) = inner.parent {
                        // Not root
                        if let Ok(Parent(new_parent)) =
                            world.query_one_filtered::<&Parent, Changed<Parent>>(inner.entity)
                        {
                            if parent != *new_parent {
                                // TODO: Look again for the entity

                                // Invalidate bind ...
                                meta.inner = None;
                                for i in (meta.range.0)..(meta.range.1) {
                                    bind.attributes[i as usize] = Ptr(null_mut());
                                }
                                continue;
                            }
                        }
                    }

                    let mut loc = world.get_entity_location(inner.entity);
                    if loc.is_none() {
                        // Missing entity
                        if let Some(found_entity) = find_entity(
                            meta.entity_index,
                            &clip.entities,
                            &mut entities_table_cache,
                            &world,
                        ) {
                            // Found new entity
                            inner.entity = found_entity;
                            // Fetch it's location
                            loc = world.get_entity_location(found_entity);
                        } else {
                            // Entity wasn't found, invalidate all the bind attributes
                            meta.inner = None; // Clear inner data
                            for i in (meta.range.0)..(meta.range.1) {
                                bind.attributes[i as usize] = Ptr(null_mut());
                            }
                            continue;
                        }
                    }

                    let loc = loc.unwrap();
                    if inner.location == loc {
                        // Nothing left todo
                        continue;
                    }
                    if inner.component == TypeId::of::<MissingComponentMarker>() {
                        // TODO: Find component by name
                        continue;
                    }

                    // Entity archetype changed
                    if let Some(component_reg) = type_registry_read_guard.get(&inner.component) {
                        let root_props = component_reg.get_component_properties(
                            &world.archetypes[loc.archetype as usize],
                            loc.index,
                        );

                        // Update pointers to reflect the new entity memory location
                        for i in (meta.range.0)..(meta.range.1) {
                            let i = i as usize;
                            let curve_index = bind.curves[i] as usize;
                            let mut path = clip.attribute[curve_index].1.split('.');
                            path.next(); // Skip component

                            let ptr = find_property_ptr(
                                path,
                                root_props,
                                clip.curves[curve_index].value_type(),
                            );
                            debug_assert!(bind.attributes[i] != ptr, "pointers can't be the same");
                            bind.attributes[i] = ptr;
                        }
                    } else {
                        // Component is missing
                        for i in (meta.range.0)..(meta.range.1) {
                            bind.attributes[i as usize] = Ptr(null_mut());
                        }
                        inner.component = TypeId::of::<MissingComponentMarker>();
                    }

                    // Update bind location
                    inner.location = loc;
                }
            }
        }
    }
}

fn find_entity(
    entity_index: u16,
    entities_ids: &Vec<(u16, String)>,
    entities_table_cache: &mut Vec<Option<Entity>>,
    world: &World,
) -> Option<Entity> {
    if let Some(entity) = &entities_table_cache[entity_index as usize] {
        Some(*entity)
    } else {
        let (parent_index, entity_name) = &entities_ids[entity_index as usize];

        // Use recursion to find the entity parent
        find_entity(*parent_index, entities_ids, entities_table_cache, world).and_then(
            |parent_entity| {
                if let Ok(children) = world.get::<Children>(parent_entity) {
                    children
                        .iter()
                        .find(|entity| {
                            if let Ok(name) = world.get::<Name>(**entity) {
                                if &name.0 == entity_name {
                                    // Update cache
                                    entities_table_cache[entity_index as usize] = Some(**entity);
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                        .copied()
                } else {
                    None
                }
            },
        )
    }
}

fn find_property_at_path<'a, P: Iterator<Item = &'a str>>(
    path: P,
    root_props: &dyn Properties,
    type_id: TypeId,
) -> Option<&dyn Property> {
    let mut target = None;
    let mut current = Some(root_props);
    for property_name in path {
        if let Some(property) = current.map(|p| p.prop(property_name)).flatten() {
            // NOTE: Vec, HashMap and other types can't be animated
            // and needed to be filtered out
            if property.property_type() != PropertyType::Value {
                target = None;
                break;
            }

            target = Some(property);
            current = property.as_properties();
        } else {
            // Failed to find property
            target = None;
            break;
        }
    }

    target.and_then(|prop| {
        if prop.any().type_id() == type_id {
            Some(prop)
        } else {
            None
        }
    })
}

#[inline(always)]
fn find_property_ptr<'a, P: Iterator<Item = &'a str>>(
    path: P,
    root_props: &dyn Properties,
    type_id: TypeId,
) -> Ptr {
    Ptr(find_property_at_path(path, root_props, type_id)
        .map(|prop| {
            let ptr = prop.any() as *const _ as *mut u8;
            // Perform an extra assertion to make sure this pointer is inside the component
            // memory bounds, so no dangle pointers can be created
            // TODO: cfg extra asserts
            {
                let root_any = root_props.any();
                let root = root_any as *const _ as *mut u8;
                let size = std::mem::size_of_val(root_any) as isize;
                assert!(unsafe { root.offset_from(ptr).abs() } <= size);
            }
            ptr
        })
        .unwrap_or(null_mut()))
}

#[tracing::instrument(skip(world, resources))]
pub(crate) fn animator_update_system(world: &mut World, resources: &mut Resources) {
    let time = resources.get::<Time>().unwrap();
    let clips = resources.get::<Assets<Clip>>().unwrap();

    // SAFETY: This is the only query with mutable reference the Animator component
    let animators_query = unsafe { world.query_unchecked::<(&mut Animator,), ()>() };
    let delta_time = time.delta_seconds;

    for (mut animator,) in animators_query {
        let animator = &mut *animator;

        // Time scales by component
        let delta_time = delta_time * animator.time_scale;

        //for layer in &mut animator.layers {
        if let Some(layer) = animator.layers.first_mut() {
            let clip_index = layer.clip;
            if let Some(clip) = animator
                .clips
                .get(clip_index as usize)
                .map(|clip_handle| clips.get(clip_handle))
                .flatten()
            {
                if let Some(bind) = &animator.binds[clip_index as usize] {
                    let curves_count = clip.curves.len();

                    // Ensure capacity for cached keyframe index vec
                    if layer.keyframe.len() != curves_count {
                        layer.keyframe.clear();
                        layer
                            .keyframe
                            .resize_with(curves_count, || Default::default());
                    }

                    // Update time
                    let mut time = layer.time + delta_time;

                    // TODO: I notice some jitter during playback

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration {
                            time = (time / clip.duration).fract() * clip.duration;
                            // Reset all keyframes cached indexes
                            layer
                                .keyframe
                                .iter_mut()
                                .for_each(|x| *x = Default::default())
                        }
                    } else {
                        // Hold
                        time = time.min(clip.duration);
                    }

                    // Advance state time
                    layer.time = time;

                    // TODO: handle backwards time

                    for ((ptr, keyframe), curve_index) in bind
                        .attributes
                        .iter()
                        .zip(layer.keyframe.iter_mut())
                        .zip(bind.curves.iter())
                    {
                        // Skip invalid properties
                        if *ptr == Ptr(null_mut()) {
                            continue;
                        }

                        let curve = &clip.curves[*curve_index as usize];

                        // SAFETY: The `animator_binding_system` is responsible to invalidate and/or update
                        // any ptr that no longer pointers to the right component for the right entity;
                        // Also it will only allows to ptr to attributes by value inside the component this means
                        // no dangle pointers

                        match curve {
                            CurveUntyped::Float(v) => {
                                let (k, v) = v.sample_forward(*keyframe, time);
                                let attr = unsafe { &mut *(ptr.0 as *mut f32) };
                                *attr = v;
                                *keyframe = k;
                            }
                            CurveUntyped::Vec3(v) => {
                                let (k, v) = v.sample_forward(*keyframe, time);
                                let attr = unsafe { &mut *(ptr.0 as *mut Vec3) };
                                *attr = v;
                                *keyframe = k;
                            }
                            CurveUntyped::Vec4(v) => {
                                let (k, v) = v.sample_forward(*keyframe, time);
                                let attr = unsafe { &mut *(ptr.0 as *mut Vec4) };
                                *attr = v;
                                *keyframe = k;
                            }
                            CurveUntyped::Quat(v) => {
                                let (k, v) = v.sample_forward(*keyframe, time);
                                let attr = unsafe { &mut *(ptr.0 as *mut Quat) };
                                *attr = v;
                                *keyframe = k;
                            }
                        }

                        // TODO: Make sure to update the components states to 'Changed'
                    }
                }
            }
        }
    }
}

// pub struct ClipLoader;

// impl AssetLoader for ClipLoader {
//     fn load<'a>(
//         &'a self,
//         bytes: &'a [u8],
//         load_context: &'a mut bevy::asset::LoadContext,
//     ) -> bevy::utils::BoxedFuture<'a, Result<()>> {
//         Box::pin(async move {
//             let clip: Clip = ron::de::from_bytes(bytes)?;
//             load_context.set_default_asset(LoadedAsset::new(clip));
//             Ok(())
//         })
//     }

//     fn extensions(&self) -> &[&str] {
//         &["anim"]
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_evaluation() {
        let curve = Curve::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0, 2.0]);
        assert_eq!(curve.sample(0.5), 1.0);
        assert_eq!(curve.sample_forward(0, 0.75), (2, 1.5));
        // TODO: Backwards sampling
    }

    #[test]
    #[should_panic]
    fn curve_bad_length() {
        let _ = Curve::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]);
    }

    #[test]
    #[should_panic]
    fn curve_time_samples_not_sorted() {
        let _ = Curve::new(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn create_clip() {
        let mut clip = Clip::default();
        let curve = Curve::from_linear(0.0, 1.0, 0.0, 1.0);
        let prop = "/Root/Ball@Sphere.radius";
        clip.add_animated_prop(prop, CurveUntyped::Float(curve));
        assert_eq!(clip.get_property_path(0), prop);
    }

    #[test]
    fn clip_replace_property() {
        // NOTE: This test is very important because it guarantees that each property have unique
        // access to some attribute during the update stages
        let mut clip = Clip::default();
        let prop = "/Root/Ball@Sphere.radius";
        clip.add_animated_prop(
            prop,
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        clip.add_animated_prop(
            prop,
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.1, 2.0)),
        );
        assert_eq!(clip.len(), 1);
        assert_eq!(clip.duration(), 1.2);
    }

    #[test]
    fn unhandled_properties_implementation_for_types() {
        // This is take as granted to simplify the function `Clip::add_animated_prop`
        // you can give a look into it to see more about their implications

        macro_rules! check_for_properties {
            ($t:ty) => {{
                let v = <$t>::default();
                let p: &dyn Property = &v;
                assert!(
                    p.as_properties().is_none(),
                    "unhandled impl `Properties` for {}",
                    stringify!($t)
                );
            }};
        }

        check_for_properties!(f32);
        check_for_properties!(Vec2);
        check_for_properties!(Vec3);
        //check_for_properties!(Vec4); // TODO: Vec4 should implement at least Property
        check_for_properties!(Quat);
    }

    // ? NOTE: Don't remove this test, right now we can skip this test
    // ? because all the animated type values (Vec2, Vec3 ...) don't
    // ? implement the `Properties` type and thus none of their inner
    // ? can be accessed
    // #[test]
    // #[should_panic]
    // fn clip_add_nested_property() {
    //     let mut clip = Clip::default();
    //     let position = Curve::from_linear(0.0, 1.0, Vec3::zero(), Vec3::unit_y());
    //     let y = Curve::from_linear(0.0, 1.0, 0.0, 1.0);
    //     clip.add_animated_prop(
    //         "Root/Ball@Transform.translation",
    //         CurveUntyped::Vec3(position),
    //     );
    //     // NOTE: Right now "Transform.translation.y" can't be accessed because Vec3 doesn't impl Properties
    //     clip.add_animated_prop("Root/Ball@Transform.translation.y", CurveUntyped::Float(y));
    // }

    // TODO: Add tests for hierarch change (creation and deletion)
    // TODO: Add tests for entity archetype change
    // TODO: Add tests for entities with same Name
    // TODO: Add tests for animated handle (impl missing)
    // TODO: Add tests for animated asset properties (impl missing)
}
