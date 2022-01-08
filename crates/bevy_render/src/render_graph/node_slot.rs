use bevy_ecs::entity::Entity;
use bevy_utils::HashMap;

use crate::render_graph::context::SlotError;
use crate::render_resource::{Buffer, Sampler, TextureView};

/// A value passed between render [`Nodes`](super::Node).
/// Corresponds to the [SlotType] specified in the [`RenderGraph`](super::RenderGraph).
///
/// Slots can have four different types of values:
/// [`Buffer`], [`TextureView`], [`Sampler`] and [`Entity`].
///
/// These values do not contain the actual render data, but only the ids to retrieve them.
#[derive(Debug, Clone)]
pub enum SlotValue {
    /// A GPU-accessible [`Buffer`].
    Buffer(Buffer),
    /// A [`TextureView`] describes a texture used in a pipeline.
    TextureView(TextureView),
    /// A texture [`Sampler`] defines how a pipeline will sample from a [`TextureView`].
    Sampler(Sampler),
    /// An entity from the ECS.
    Entity(Entity),
}

impl SlotValue {
    /// Returns the [`SlotType`] of this value.
    pub fn slot_type(&self) -> SlotType {
        match self {
            SlotValue::Buffer(_) => SlotType::Buffer,
            SlotValue::TextureView(_) => SlotType::TextureView,
            SlotValue::Sampler(_) => SlotType::Sampler,
            SlotValue::Entity(_) => SlotType::Entity,
        }
    }
}

#[derive(Default, Clone)]
pub struct SlotValues {
    values: Vec<(&'static str, SlotValue)>,
}

impl SlotValues {
    pub fn new(iter: impl IntoIterator<Item = (&'static str, SlotValue)>) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }

    pub fn empty() -> Self {
        Self { values: Vec::new() }
    }

    pub fn get_infos(&self) -> SlotInfos {
        self.values
            .iter()
            .map(|(label, value)| SlotInfo::new(*label, value.slot_type()))
            .into()
    }

    pub fn get_value(&self, label: &&'static str) -> Result<&SlotValue, SlotError> {
        match self.values.iter().find(|(l, _v)| l == label) {
            None => Err(SlotError::InvalidSlot(label)),
            Some((_, v)) => Ok(v),
        }
    }
}

impl<L: Into<&'static str>, V: Into<SlotValue>, T: IntoIterator<Item = (L, V)>> From<T>
    for SlotValues
{
    fn from(t: T) -> Self {
        SlotValues::new(t.into_iter().map(|(l, v)| (l.into(), v.into())))
    }
}

impl From<Buffer> for SlotValue {
    fn from(value: Buffer) -> Self {
        SlotValue::Buffer(value)
    }
}

impl From<TextureView> for SlotValue {
    fn from(value: TextureView) -> Self {
        SlotValue::TextureView(value)
    }
}

impl From<Sampler> for SlotValue {
    fn from(value: Sampler) -> Self {
        SlotValue::Sampler(value)
    }
}

impl From<Entity> for SlotValue {
    fn from(value: Entity) -> Self {
        SlotValue::Entity(value)
    }
}

/// Describes the render resources created (output) or used (input) by
/// the render [`Nodes`](super::Node).
///
/// This should not be confused with [`SlotValue`], which actually contains the passed data.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SlotType {
    /// A GPU-accessible [`Buffer`].
    Buffer,
    /// A [`TextureView`] describes a texture used in a pipeline.
    TextureView,
    /// A texture [`Sampler`] defines how a pipeline will sample from a [`TextureView`].
    Sampler,
    /// An entity from the ECS.
    Entity,
}

/// The internal representation of a slot, which specifies its [`SlotType`] and name.
#[derive(Clone, Debug)]
pub struct SlotInfo {
    pub name: &'static str,
    pub slot_type: SlotType,
}

impl SlotInfo {
    pub fn new(name: impl Into<&'static str>, slot_type: SlotType) -> Self {
        SlotInfo {
            name: name.into(),
            slot_type,
        }
    }
}

/// A collection of input or output [`SlotInfos`](SlotInfo) for
/// a [`NodeState`](super::NodeState).
#[derive(Default, Debug)]
pub struct SlotInfos {
    slots: HashMap<&'static str, SlotInfo>,
}

impl<T: IntoIterator<Item = SlotInfo>> From<T> for SlotInfos {
    fn from(slots: T) -> Self {
        SlotInfos {
            slots: slots.into_iter().map(|info| (info.name, info)).collect(),
        }
    }
}

impl SlotInfos {
    /// Returns the count of slots.
    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns true if there are no slots.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Retrieves the [`SlotInfo`] for the provided label.
    pub fn get_slot(&self, label: &'static str) -> Option<&SlotInfo> {
        self.slots.get(label)
    }

    /// Retrieves the [`SlotInfo`] for the provided label mutably.
    pub fn get_slot_mut(&mut self, label: &'static str) -> Option<&mut SlotInfo> {
        self.slots.get_mut(label)
    }

    /// Returns an iterator over the slot infos.
    pub fn iter(&self) -> impl Iterator<Item = &SlotInfo> {
        self.slots.values()
    }

    /// Returns slots which this has but the other doesn't and those that the other has but this doesn't
    pub fn get_conflicts(&self, _with: &Self) -> (Vec<SlotInfo>, Vec<SlotInfo>) {
        todo!()
    }
}
