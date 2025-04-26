use alloc::borrow::Cow;
use bevy_derive::Deref;
use derive_more::derive::From;

#[derive(Clone, Deref)]
pub struct SlotValue(String);

#[derive(Debug, Clone, Eq, PartialEq, From)]
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

#[derive(Clone, Debug)]
pub struct SlotInfo {
    pub name: Cow<'static, str>,
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
