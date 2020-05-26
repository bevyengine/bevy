use crate::{AsProperties, Properties, Property};
use smallvec::{SmallVec, Array};
use std::any::Any;
use serde::Serialize;

impl<T, I> Property for SmallVec<T>
where
    T: Clone + Send + Sync + Serialize + 'static + Array<Item=I>,
    I: Send + Sync + Clone + Serialize + 'static
{
    #[inline]
    fn any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn clone_prop(&self) -> Box<dyn Property> {
        Box::new(self.clone())
    }

    #[inline]
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }

    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = prop.clone();
        }
    }
}

impl<T> AsProperties for SmallVec<T> where T: Array {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}