use anyhow::Result;
use bevy_asset::{Assets, Handle};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::Location;
use bevy_math::prelude::*;
use bevy_property::Properties;
use bevy_property::PropertyType;
use bevy_transform::prelude::*;
use bevy_type_registry::{TypeRegistry, TypeUuid};
use serde::{Deserialize, Serialize};
use std::any::TypeId;

use super::lerping::LerpValue;

// Naive implementation for skeleton animation

// TODO: Curve/Curve need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

#[derive(Default, Debug, Serialize, Deserialize, TypeUuid)]
#[uuid = "4c76e6c3-706d-4a74-af8e-4f48033e0733"]
pub struct Clip {
    #[serde(default = "clip_default_warp")]
    pub warp: bool,
    pub length: f32,
    /// Property to be animated will be in the format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    ///
    /// *NOTE*: Keep sorted to improve efficiency
    properties: Vec<String>,
    values: Vec<Value>,
}

fn clip_default_warp() -> bool {
    true
}

impl Clip {
    pub fn add_animated_prop(&mut self, property_path: String, value: Value) {
        self.properties.push(property_path);
        self.values.push(value);
    }

    pub fn iter(&mut self) -> impl Iterator<Item = (&String, &Value)> {
        self.properties.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut String, &mut Value)> {
        self.properties.iter_mut().zip(self.values.iter_mut())
    }

    pub fn optimize(&mut self) {
        // // SAFE: No string gets dropped and are only used during for sorting
        // let props: &[String] = unsafe { &*(&self.properties[..] as *const _) };

        // let mut indexes = props
        //     .iter()
        //     .enumerate()
        //     .map(|(i, n)| (i, i, n))
        //     .collect::<Vec<_>>();

        // indexes.sort_by(|a, b| a.2.partial_cmp(b.2).unwrap());
        // indexes.iter_mut().enumerate().for_each(|(i, (j, k, _))| {
        //     *k = i;
        // });
        // // It's necessary to sort
        // indexes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // for (i, j, _) in indexes {
        //     self.properties[..].swap(i, j);
        //     self.values[..].swap(i, j);
        // }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Float(Curve<f32>),
    Vec3(Curve<Vec3>),
    Vec4(Curve<Vec4>),
    Quat(Curve<Quat>),
}

impl Value {
    pub fn samples_mut(&mut self) -> impl Iterator<Item = &mut f32> {
        match self {
            Value::Float(c) => c.samples.iter_mut(),
            Value::Vec3(c) => c.samples.iter_mut(),
            Value::Vec4(c) => c.samples.iter_mut(),
            Value::Quat(c) => c.samples.iter_mut(),
        }
    }

    pub fn type_name(&self) -> &str {
        use std::any::type_name;
        match self {
            Value::Float(_) => type_name::<f32>(),
            Value::Vec3(_) => type_name::<Vec3>(),
            Value::Vec4(_) => type_name::<Vec4>(),
            Value::Quat(_) => type_name::<Quat>(),
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
            T::lerp(self.values[i], self.values[index], t)
        };

        (index, value)
    }
}

#[derive(Default, Debug, Clone, Properties)]
struct State {
    clip: usize,
    time: f32,
    keyframe: Vec<usize>,
}

// #[derive(Default, Debug, Clone, Properties)]
// struct KeyframeState {
//     position: usize,
//     rotation: usize,
//     scale: usize,
// }

// #[derive(Default, Clone, Debug, Properties)]
// struct Lerp {
//     n: f32,
//     time: f32,
//     duration: f32,
// }

#[derive(Debug, Properties)]
pub struct Animator {
    animations: Vec<Handle<Clip>>,
    #[property(ignore)]
    binds: Vec<ValueBind>,
    pub time_scale: f32,
    // hierarchy: Vec<()>,
    current: State,
    // next: Option<State>, // TODO: Keep memory allocated to be reused
    // transition: Lerp,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            animations: vec![],
            binds: vec![],
            time_scale: 1.0,
            current: State::default(),
            // next: None,
            // transition: Lerp::default(),
        }
    }
}

impl Animator {
    pub fn add_clip(&mut self, clip: Handle<Clip>) {
        if self.animations.contains(&clip) {
            return;
        }
        self.animations.push(clip);
    }

    pub fn clips_len(&self) -> usize {
        self.animations.len()
    }
}

#[derive(Debug)]
struct ValueBind {
    dirty: bool,
    valid: Vec<bool>,
    entities: Vec<Entity>,
    components: Vec<TypeId>,
    properties: Vec<isize>,
}

impl Default for ValueBind {
    fn default() -> Self {
        Self {
            dirty: true,
            valid: vec![],
            entities: vec![],
            components: vec![],
            properties: vec![],
        }
    }
}

/// Fetches entities and properties to animate
pub(crate) fn animator_fetch(world: &mut World, resources: &mut Resources) {
    // Fetch useful resources
    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let clips = resources.get::<Assets<Clip>>().unwrap();

    // Build queries
    let animators_query = unsafe { world.query_unchecked::<(&mut Animator, &Children), ()>() };

    // TODO: Parallelize use ComputeTaskPool resource?
    for (mut animator, root_children) in animators_query {
        // Trick for proper borrow rules
        let animator = &mut *animator;

        // TODO: Needs to be updated when the hierarch change
        // TODO: Needs to be updated when a clip changes
        let animations_count = animator.animations.len();
        if animations_count == animator.binds.len() {
            if animator.binds.iter().all(|item| !item.dirty) {
                break;
            }
        }

        animator
            .binds
            .resize_with(animations_count, || Default::default());

        for (clip_index, clip_handle) in animator.animations.iter().enumerate() {
            let instance = &mut animator.binds[clip_index];
            if !instance.dirty {
                continue;
            }

            if let Some(clip) = clips.get(clip_handle) {
                // Get instance reference to modify during the function
                instance.dirty = false;

                let mut paths = clip
                    .properties
                    .iter()
                    .map(|p| {
                        let mut split = p.split('@');
                        (
                            split.next().unwrap_or("").split('/'),
                            split.next().unwrap_or("").split('.'),
                        )
                    })
                    .collect::<Vec<_>>();

                // NOTE: `clear` then `resize` is an good way of reuse memory
                // as much as possible but it doesn't free it so it may come
                // to need to manually trim these Vec's whenever they get too large

                let valid = &mut instance.valid;
                valid.clear();
                valid.resize(paths.len(), false);

                let entities = &mut instance.entities;
                entities.clear();
                entities.resize(paths.len(), Entity::new(u32::MAX));

                let mut hierarchy = paths.iter_mut().map(|p| p.0.next()).collect::<Vec<_>>();

                // 1. Find entities
                let mut stack = vec![];
                // First uneducated search for all hierarchy
                for parent in root_children.iter().copied() {
                    if let Ok((entity, name, children)) =
                        world.query_one::<(Entity, &Name, &Children)>(parent)
                    {
                        let name = Some(name.0.as_str());
                        for (i, level) in hierarchy.iter_mut().enumerate() {
                            if *level != name {
                                continue;
                            }

                            *level = paths[i].0.next();
                            entities[i] = entity;
                            valid[i] = true;
                            stack.push((i, children));
                        }
                    }
                }
                // Fast educated search into the nested hierarchy
                // TODO: Many identical queries for the same entity will happen here!
                while let Some((i, parents)) = stack.pop() {
                    for parent in parents.iter().copied() {
                        if let Ok((entity, name, children)) =
                            world.query_one::<(Entity, &Name, &Children)>(parent)
                        {
                            if hierarchy[i] == Some(name.0.as_str()) {
                                hierarchy[i] = paths[i].0.next();
                                entities[i] = entity; // Replaces the entity for the lowest
                                valid[i] = true;
                                stack.push((i, children));
                                break;
                            }
                        }
                    }
                }

                let components = &mut instance.components;
                components.clear();
                components.resize(paths.len(), TypeId::of::<()>());

                let properties = &mut instance.properties;
                properties.clear();
                properties.resize(paths.len(), 0);

                // 2. Find components and properties in all entities that completed the path
                for (i, level) in hierarchy.iter().enumerate() {
                    if valid[i] {
                        // Not reached the entity at the path
                        if level.is_some() {
                            valid[i] = false;
                            continue;
                        }

                        let component = paths[i].1.next().unwrap_or("?");
                        if let Some(component_reg) = type_registry
                            .component
                            .read()
                            .get_with_short_name(component)
                        {
                            if let Some(location) = world.get_entity_location(entities[i]) {
                                components[i] = component_reg.ty;

                                let root_properties = component_reg.get_component_properties(
                                    &world.archetypes[location.archetype as usize],
                                    location.index,
                                );

                                // Find the target property to animate
                                let mut properties_lookup = Some(root_properties.clone());
                                let mut target_property = None;

                                for property_name in &mut paths[i].1 {
                                    if let Some(property) =
                                        properties_lookup.map(|p| p.prop(property_name)).flatten()
                                    {
                                        // NOTE: Vec, HashMap and other types can't be animated
                                        // and needed to be filtered out
                                        if property.property_type() != PropertyType::Value {
                                            target_property = None;
                                            valid[i] = false;
                                            break;
                                        }

                                        properties_lookup = property.as_properties();
                                        target_property = Some(property);
                                    } else {
                                        // Failed to find property
                                        target_property = None;
                                        valid[i] = false;
                                        break;
                                    }
                                }

                                if let Some(target_property) = target_property {
                                    // Check for the expected type
                                    if target_property.type_name() == clip.values[i].type_name() {
                                        properties[i] = {
                                            let root_ptr = root_properties.as_ptr();
                                            let target_ptr = target_property.as_ptr();
                                            // ! NOTE: Requires rust 1.47.0
                                            // TODO: Need safety check or SAFE argument
                                            unsafe { target_ptr.offset_from(root_ptr) }
                                        };
                                    } else {
                                        valid[i] = false;
                                    }
                                // locations[i] = location;
                                } else {
                                    valid[i] = false;
                                }
                            }
                        }
                    }
                }

                // for (i, valid) in valid.iter().copied().enumerate() {
                //     if valid {
                //         println!("found: {}", &clip.properties[i]);
                //     } else {
                //         println!("missing: {}", &clip.properties[i]);
                //     }
                // }
            }
        }
    }
}

pub(crate) fn animator_update(world: &mut World, resources: &mut Resources) {
    let time = resources.get::<Time>().unwrap();
    let clips = resources.get::<Assets<Clip>>().unwrap();
    let type_registry = resources.get::<TypeRegistry>().unwrap();

    // Build queries
    let animators_query = unsafe { world.query_unchecked::<(&mut Animator,), ()>() };

    // let delta_time = if keyboard.just_pressed(KeyCode::Right) {
    //     1.0 / 60.0
    // } else {
    //     0.0
    // };

    let delta_time = time.delta_seconds;

    // TODO: Parallelize
    for (mut animator,) in animators_query {
        let animator = &mut *animator;

        // Time scales by component
        let delta_time = delta_time * animator.time_scale;

        let current = &mut animator.current;
        let clip_index = current.clip;
        if let Some(current_clip) = animator
            .animations
            .get(clip_index)
            .map(|clip_handle| clips.get(clip_handle))
            .flatten()
        {
            let bind = &animator.binds[clip_index];
            let curves_count = current_clip.values.len();

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
                if time > current_clip.length {
                    time = (time / current_clip.length).fract() * current_clip.length;
                    // Reset all keyframes cached indexes
                    current
                        .keyframe
                        .iter_mut()
                        .for_each(|x| *x = Default::default())
                }
            } else {
                // Hold
                time = time.min(current_clip.length);
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
                if !bind.valid[i] {
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

                match &current_clip.values[i] {
                    Value::Float(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut f32) };
                        *attr = v;
                        *keyframe = k;
                    }
                    Value::Vec3(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut Vec3) };
                        *attr = v;
                        *keyframe = k;
                    }
                    Value::Vec4(v) => {
                        let (k, v) = v.sample(*keyframe, time);
                        let attr = unsafe { &mut *(attr as *mut Vec4) };
                        *attr = v;
                        *keyframe = k;
                    }
                    Value::Quat(v) => {
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
