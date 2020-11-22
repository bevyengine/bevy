use anyhow::Result;
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle, HandleUntyped};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::{Location, SystemId};
use bevy_math::prelude::*;
use bevy_property::Properties;
use bevy_property::PropertyType;
use bevy_transform::prelude::*;
use bevy_type_registry::{TypeRegistry, TypeUuid};
use bevy_utils::HashSet;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::convert::TryFrom;
use std::ptr::null_mut;

use super::lerping::LerpValue;

// TODO: Curve/Curve need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

#[derive(Debug, Serialize, Deserialize, TypeUuid)]
#[uuid = "4c76e6c3-706d-4a74-af8e-4f48033e0733"]
pub struct Clip {
    #[serde(default = "clip_default_warp")]
    pub warp: bool,
    duration: f32,
    /// Entity identification parent index on this same vec and name
    entities: Vec<(u16, String)>,
    /// Attribute is made by the entity index and a string that combines
    /// component name followed by their attributes spaced by a period,
    /// like so: `"Transform.translation.x"`
    attribute: Vec<(u16, String)>,
    curves: Vec<CurveUntyped>,
}

fn clip_default_warp() -> bool {
    true
}

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
    pub fn add_animated_prop(&mut self, property_path: &str, curve: CurveUntyped) {
        // Clip an only have some amount of curves and entities
        // this limitation was added to save memory (but you can increase it if you want)
        assert!(
            (self.curves.len() as u16) <= u16::MAX,
            "curve limit reached"
        );

        let path =
            property_path.split_at(property_path.rfind('@').expect("property path missing @"));

        let mut entity = 0; // Start search from root
        for name in path.0.split('/') {
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
                // Soft limit added to save memory, identical to the curve limit
                entity = u16::try_from(e).expect("entities limit reached");
                self.entities.push((entity, name.to_string()));
            }
        }

        // TODO: also replace the curve
        // TODO: we can only have (u16 - 1) max
        // TODO: "...@Transform.translation" and "...@Transform.translation.x" can't live with each other
        self.duration = self.duration.max(curve.duration());
        self.attribute.push((entity, path.1.to_string()));
        self.curves.push(curve);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CurveUntyped {
    Float(Curve<f32>),
    Vec3(Curve<Vec3>),
    Vec4(Curve<Vec4>),
    Quat(Curve<Quat>),
    //Handle(Curve<HandleUntyped>), // TODO: Requires (de)serialize!
}

impl CurveUntyped {
    pub fn duration(&self) -> f32 {
        match self {
            CurveUntyped::Float(v) => v.duration(),
            CurveUntyped::Vec3(v) => v.duration(),
            CurveUntyped::Vec4(v) => v.duration(),
            CurveUntyped::Quat(v) => v.duration(),
        }
    }

    // pub fn value_type_name(&self) -> &str {
    //     use std::any::type_name;
    //     match self {
    //         CurveUntyped::Float(_) => type_name::<f32>(),
    //         CurveUntyped::Vec3(_) => type_name::<Vec3>(),
    //         CurveUntyped::Vec4(_) => type_name::<Vec4>(),
    //         CurveUntyped::Quat(_) => type_name::<Quat>(),
    //     }
    // }

    pub fn value_type(&self) -> TypeId {
        match self {
            CurveUntyped::Float(_) => TypeId::of::<f32>(),
            CurveUntyped::Vec3(_) => TypeId::of::<Vec3>(),
            CurveUntyped::Vec4(_) => TypeId::of::<Vec4>(),
            CurveUntyped::Quat(_) => TypeId::of::<Quat>(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Curve<T> {
    // TODO: Linear and Spline variants
    samples: Vec<f32>,
    values: Vec<T>,
}

impl<T> Curve<T>
where
    T: LerpValue,
{
    pub fn duration(&self) -> f32 {
        self.samples.last().copied().unwrap_or(0.0)
    }

    pub fn new(samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Proper error handling
        assert!(samples.len() == values.len());
        Self { samples, values }
    }

    /// Samples the curve beginning from the keyframe at index
    pub fn sample(&self, mut index: usize, time: f32) -> (usize, T) {
        // Adjust for the current keyframe index
        let last_index = self.samples.len() - 1;
        loop {
            if index > last_index {
                return (last_index, *self.values.last().unwrap());
            }

            if self.samples[index] < time {
                index += 1;
            } else {
                break;
            }
        }

        let value = if index == 0 {
            self.values[0]
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
}

#[derive(Default, Debug, Clone, Properties)]
struct Layer {
    pub weight: f32,
    pub clip: usize,
    pub time: f32,
    keyframe: Vec<usize>,
}

#[derive(Debug, Properties)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,
    #[property(ignore)]
    binds: Vec<Option<Binds>>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
    // TODO: Transitions to control the animation layer blending byt time and other params
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
    pub fn add_clip(&mut self, clip: Handle<Clip>) {
        if self.clips.contains(&clip) {
            return;
        }
        self.clips.push(clip);
    }

    pub fn clips_len(&self) -> usize {
        self.clips.len()
    }
}

#[derive(Debug)]
struct Binds {
    metadata: Vec<BindMeta>,
    curves: Vec<u16>,
    attributes: Vec<Ptr>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Ptr(*mut u8);

// SAFETY: Store pointers to each attribute to be updated, a clip can't have two pointers
// with the same value. Each clip per Animator will be updated sequentially
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

/// Marker type for missing component
struct MissingComponentMarker;

/// Missing component
const MISSING_COMPONENT: TypeId = TypeId::of::<MissingComponentMarker>();

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

#[tracing::instrument(skip(world, resources))]
pub(crate) fn animator_binding_system(world: &mut World, resources: &mut Resources) {
    let system_id = SystemId(0); // ? NOTE: shouldn't be required to match the system id

    // Fetch resources
    let mut state = resources
        .get_local_mut::<ClipResourceProviderState>(system_id)
        .unwrap();

    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let type_registry_read_guard = type_registry.component.read();

    let clips = resources.get::<Assets<Clip>>().unwrap();
    let clip_events = resources.get::<Events<AssetEvent<Clip>>>().unwrap();

    // Builds query
    // SAFETY: Only query with mutable reference the Animator component
    let mut animator_query =
        unsafe { world.query_unchecked::<(Entity, &mut Animator, &Children), ()>() };

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

    for (root, mut animator, children) in animator_query {
        let mut animator = &mut *animator; // Make sure the borrow checker is happy

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
                    let b = Binds {
                        metadata: Vec::with_capacity(clip.entities.len()),
                        curves: Vec::with_capacity(clip.attribute.len()),
                        attributes: Vec::with_capacity(clip.attribute.len()),
                    };

                    let mut range = (0, 0);
                    let mut prev_partial_info = None;
                    let mut prev_component = MISSING_COMPONENT;
                    let mut prev_component_short_name = "";

                    for (curve_index, (entity_index, attr_path)) in
                        clip.attribute.iter().enumerate()
                    {
                        let mut commit = false;
                        let mut partial_info = None;
                        let mut component = MISSING_COMPONENT;

                        // Query component by name
                        let path = attr_path.split('.');
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
                                b.attributes.push(Ptr(find_target_property(path, root_props)
                                    .and_then(|prop| {
                                        if prop.any().type_id()
                                            == clip.curves[curve_index].value_type()
                                        {
                                            Some(prop.as_ptr() as *mut _)
                                        } else {
                                            // TODO: Log warn wrong type
                                            None
                                        }
                                    })
                                    .unwrap_or(null_mut())));

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
                            range.1 = b.curves.len() as u16;

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
                        }
                    }

                    *bind = Some(b);

                    // No need to continue because binds are fresh
                    continue;
                }

                // Check bind state and update the necessary parts
                let bind = bind.unwrap();
                for meta in bind.metadata.iter_mut() {
                    if meta.inner.is_none() {
                        // TODO: Handle missing entity
                        continue;
                    }

                    let inner = meta.inner.as_mut().unwrap();

                    // TODO: Handle `Parent` change ..

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
                            for i in (meta.range.0)..(meta.range.1) {
                                bind.attributes[i as usize] = Ptr(null_mut());
                            }
                        }
                    }

                    let loc = loc.unwrap();
                    if inner.location == loc {
                        // Nothing left todo
                        continue;
                    }

                    if inner.component == MISSING_COMPONENT {
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
                            let path = clip.attribute[curve_index].1.split('.');
                            path.next(); // Skip component
                            if let Some(prop) = find_target_property(path, root_props) {
                                // Found property
                                bind.attributes[i] = Ptr(prop.as_ptr() as *mut _);
                            } else {
                                bind.attributes[i] = Ptr(null_mut());
                            }
                        }
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
                if let Ok((children,)) = world.get::<(Children,)>(parent_entity) {
                    children
                        .iter()
                        .find(|entity| {
                            if let Ok((name,)) = world.get::<(Name,)>(**entity) {
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

fn find_target_property<'a, P: Iterator<Item = &'a str>>(
    path: P,
    mut root_props: &dyn Properties,
) -> Option<&dyn Properties> {
    let mut target_property = None;
    for property_name in path {
        if let Some(property) = root_props.prop(property_name) {
            // NOTE: Vec, HashMap and other types can't be animated
            // and needed to be filtered out
            if property.property_type() != PropertyType::Value {
                target_property = None;
                break;
            }

            root_props = property.as_properties().unwrap();
            target_property = Some(root_props);
        } else {
            // Failed to find property
            target_property = None;
            break;
        }
    }
    target_property
}

#[tracing::instrument(skip(world, resources))]
pub(crate) fn animator_update(world: &mut World, resources: &mut Resources) {
    let time = resources.get::<Time>().unwrap();
    let clips = resources.get::<Assets<Clip>>().unwrap();
    let type_registry = resources.get::<TypeRegistry>().unwrap();

    // Build queries
    let animators_query = unsafe { world.query_unchecked::<(&mut Animator,), ()>() };
    let delta_time = time.delta_seconds;

    // TODO: Parallelize
    for (mut animator,) in animators_query {
        let mut animator = &mut *animator;

        // Time scales by component
        let delta_time = delta_time * animator.time_scale;

        let current = &mut animator.current;
        let clip_index = current.clip;
        if let Some(current_clip) = animator
            .clips
            .get(clip_index)
            .map(|clip_handle| clips.get(clip_handle))
            .flatten()
        {
            let bind = &animator.binds[clip_index];
            let curves_count = current_clip.curves.len();

            // Ensure capacity for cached keyframe index vec
            if current.keyframe.len() != curves_count {
                current.keyframe.clear();
                current
                    .keyframe
                    .resize_with(curves_count, || Default::default());
            }

            // Update time
            let mut time = current.time + delta_time;

            // Warp mode
            if current_clip.warp {
                // Warp Around
                if time > current_clip.duration {
                    time = (time / current_clip.duration).fract() * current_clip.duration;
                    // Reset all keyframes cached indexes
                    current
                        .keyframe
                        .iter_mut()
                        .for_each(|x| *x = Default::default())
                }
            } else {
                // Hold
                time = time.min(current_clip.duration);
            }

            // Advance state time
            current.time = time;

            let mut entity = None;
            let mut component = None;
            let mut pointer = std::ptr::null_mut();
            let mut location = Location {
                archetype: u32::MAX,
                index: usize::MAX,
            };
            for i in 0..curves_count {
                if !bind.valid.get(i).unwrap_or(&false) {
                    continue;
                }

                let next_entity = bind.entities[i];
                if entity != Some(next_entity) {
                    if let Some(l) = world.get_entity_location(next_entity) {
                        location = l;
                        entity = Some(next_entity);
                        component = None; // Force component update
                    } else {
                        // Missing entity
                        continue;
                    }
                }

                let next_component = bind.components[i];
                if component != Some(next_component) {
                    if let Some(properties) = type_registry
                        .component
                        .read()
                        .get(&next_component)
                        .map(|reg| {
                            reg.get_component_properties(
                                &world.archetypes[location.archetype as usize],
                                location.index,
                            )
                        })
                    {
                        // TODO: Find a better way
                        pointer = properties.as_ptr() as *mut u8;
                        component = Some(next_component);
                    } else {
                        // Missing component
                        continue;
                    }
                }

                let attr = pointer.wrapping_offset(bind.properties[i]);
                let keyframe = &mut current.keyframe[i];

                match &current_clip.curves[i] {
                    CurveUntyped::Float(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut f32) };
                        *attr = v;
                        *keyframe = k;
                    }
                    CurveUntyped::Vec3(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut Vec3) };
                        *attr = v;
                        *keyframe = k;
                    }
                    CurveUntyped::Vec4(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut Vec4) };
                        *attr = v;
                        *keyframe = k;
                    }
                    CurveUntyped::Quat(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut Quat) };
                        *attr = v;
                        *keyframe = k;
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
