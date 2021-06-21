use bevy_ecs::entity::Entity;
use std::borrow::Cow;

use crate::render_resource::{Buffer, Sampler, TextureView};

#[derive(Debug, Clone)]
pub enum SlotValue {
    Buffer(Buffer),
    TextureView(TextureView),
    Sampler(Sampler),
    Entity(Entity),
}

impl SlotValue {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SlotType {
    Buffer,
    TextureView,
    Sampler,
    Entity,
}

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
        SlotLabel::Name(value.clone())
    }
}

impl From<usize> for SlotLabel {
    fn from(value: usize) -> Self {
        SlotLabel::Index(value)
    }
}

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
    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn get_slot(&self, label: impl Into<SlotLabel>) -> Option<&SlotInfo> {
        let label = label.into();
        let index = self.get_slot_index(&label)?;
        self.slots.get(index)
    }

    pub fn get_slot_mut(&mut self, label: impl Into<SlotLabel>) -> Option<&mut SlotInfo> {
        let label = label.into();
        let index = self.get_slot_index(&label)?;
        self.slots.get_mut(index)
    }

    pub fn get_slot_index(&self, label: impl Into<SlotLabel>) -> Option<usize> {
        let label = label.into();
        match label {
            SlotLabel::Index(index) => Some(index),
            SlotLabel::Name(ref name) => self
                .slots
                .iter()
                .enumerate()
                .find(|(_i, s)| s.name == *name)
                .map(|(i, _s)| i),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &SlotInfo> {
        self.slots.iter()
    }
}
