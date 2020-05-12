use super::RenderGraphError;
use crate::{render_resource::RenderResource, shader::FieldBindType};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct ResourceSlot {
    pub resource: Option<RenderResource>,
    pub info: ResourceSlotInfo,
}

#[derive(Default, Debug, Clone)]
pub struct ResourceSlots {
    slots: Vec<ResourceSlot>,
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

impl From<usize> for SlotLabel {
    fn from(value: usize) -> Self {
        SlotLabel::Index(value)
    }
}

impl ResourceSlots {
    pub fn set(&mut self, label: impl Into<SlotLabel>, resource: RenderResource) {
        let mut slot = self.get_slot_mut(label).unwrap();
        slot.resource = Some(resource);
    }

    pub fn get(&self, label: impl Into<SlotLabel>) -> Option<RenderResource> {
        let slot = self.get_slot(label).unwrap();
        slot.resource.clone()
    }

    pub fn get_slot(&self, label: impl Into<SlotLabel>) -> Result<&ResourceSlot, RenderGraphError> {
        let label = label.into();
        let index = self.get_slot_index(&label)?;
        self.slots
            .get(index)
            .ok_or_else(|| RenderGraphError::InvalidNodeSlot(label))
    }

    pub fn get_slot_mut(
        &mut self,
        label: impl Into<SlotLabel>,
    ) -> Result<&mut ResourceSlot, RenderGraphError> {
        let label = label.into();
        let index = self.get_slot_index(&label)?;
        self.slots
            .get_mut(index)
            .ok_or_else(|| RenderGraphError::InvalidNodeSlot(label))
    }

    pub fn get_slot_index(&self, label: impl Into<SlotLabel>) -> Result<usize, RenderGraphError> {
        let label = label.into();
        match label {
            SlotLabel::Index(index) => Ok(index),
            SlotLabel::Name(ref name) => self
                .slots
                .iter()
                .enumerate()
                .find(|(_i, s)| s.info.name == *name)
                .map(|(i, _s)| i)
                .ok_or_else(|| RenderGraphError::InvalidNodeSlot(label)),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &ResourceSlot> {
        self.slots.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ResourceSlot> {
        self.slots.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }
}

impl From<&ResourceSlotInfo> for ResourceSlot {
    fn from(slot: &ResourceSlotInfo) -> Self {
        ResourceSlot {
            resource: None,
            info: slot.clone(),
        }
    }
}

impl From<&[ResourceSlotInfo]> for ResourceSlots {
    fn from(slots: &[ResourceSlotInfo]) -> Self {
        ResourceSlots {
            slots: slots
                .iter()
                .map(|s| s.into())
                .collect::<Vec<ResourceSlot>>(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourceSlotInfo {
    pub name: Cow<'static, str>,
    pub resource_type: FieldBindType,
}

impl ResourceSlotInfo {
    pub fn new(name: impl Into<Cow<'static, str>>, resource_type: FieldBindType) -> Self {
        ResourceSlotInfo {
            name: name.into(),
            resource_type,
        }
    }
}
