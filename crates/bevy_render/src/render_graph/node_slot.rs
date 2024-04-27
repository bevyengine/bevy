use bevy_ecs::entity::Entity;
use std::{borrow::Cow, fmt};

use crate::render_resource::{Buffer, Sampler, TextureView};

/// A value passed between render [`Nodes`](super::Node).
/// Corresponds to the [`SlotType`] specified in the [`RenderGraph`](super::RenderGraph).
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

impl fmt::Display for SlotType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SlotType::Buffer => "Buffer",
            SlotType::TextureView => "TextureView",
            SlotType::Sampler => "Sampler",
            SlotType::Entity => "Entity",
        };

        f.write_str(s)
    }
}

/// A [`SlotLabel`] is used to reference a slot by either its name or index
/// inside the [`RenderGraph`](super::RenderGraph).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SlotLabel {
    Index(usize),
    Name(Cow<'static, str>),
}

impl From<&SlotLabel> for SlotLabel {
    fn from(value: &SlotLabel) -> Self {
        value.clone()
    }
}

impl From<String> for SlotLabel {
    fn from(value: String) -> Self {
        SlotLabel::Name(value.into())
    }
}

impl From<&'static str> for SlotLabel {
    fn from(value: &'static str) -> Self {
        SlotLabel::Name(value.into())
    }
}

impl From<Cow<'static, str>> for SlotLabel {
    fn from(value: Cow<'static, str>) -> Self {
        SlotLabel::Name(value)
    }
}

impl From<usize> for SlotLabel {
    fn from(value: usize) -> Self {
        SlotLabel::Index(value)
    }
}

/// The internal representation of a slot, which specifies its [`SlotType`] and name.
#[derive(Clone, Debug)]
pub struct SlotInfo {
    pub name: Cow<'static, str>,
    pub slot_type: SlotType,
}

impl SlotInfo {
    pub fn new(name: impl Into<Cow<'static, str>>, slot_type: SlotType) -> Self {
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
    slots: Vec<SlotInfo>,
}

impl<T: IntoIterator<Item = SlotInfo>> From<T> for SlotInfos {
    fn from(slots: T) -> Self {
        SlotInfos {
            slots: slots.into_iter().collect(),
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
    pub fn get_slot(&self, label: impl Into<SlotLabel>) -> Option<&SlotInfo> {
        let label = label.into();
        let index = self.get_slot_index(label)?;
        self.slots.get(index)
    }

    /// Retrieves the [`SlotInfo`] for the provided label mutably.
    pub fn get_slot_mut(&mut self, label: impl Into<SlotLabel>) -> Option<&mut SlotInfo> {
        let label = label.into();
        let index = self.get_slot_index(label)?;
        self.slots.get_mut(index)
    }

    /// Retrieves the index (inside input or output slots) of the slot for the provided label.
    pub fn get_slot_index(&self, label: impl Into<SlotLabel>) -> Option<usize> {
        let label = label.into();
        match label {
            SlotLabel::Index(index) => Some(index),
            SlotLabel::Name(ref name) => self.slots.iter().position(|s| s.name == *name),
        }
    }

    /// Returns an iterator over the slot infos.
    pub fn iter(&self) -> impl Iterator<Item = &SlotInfo> {
        self.slots.iter()
    }
}
