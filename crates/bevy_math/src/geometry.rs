use bevy_reflect::Reflect;

/// A rect, as defined by its "side" locations
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Rect<T: Reflect + PartialEq> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T: Reflect + PartialEq> Rect<T> {
    pub fn all(value: T) -> Self
    where
        T: Clone,
    {
        Rect {
            left: value.clone(),
            right: value.clone(),
            top: value.clone(),
            bottom: value,
        }
    }
}

impl<T: Default + Reflect + PartialEq> Default for Rect<T> {
    fn default() -> Self {
        Self {
            left: Default::default(),
            right: Default::default(),
            top: Default::default(),
            bottom: Default::default(),
        }
    }
}
