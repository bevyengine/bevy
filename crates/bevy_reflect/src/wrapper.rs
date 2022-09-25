use std::any::{Any, TypeId};

use crate::{FromReflect, Reflect};

pub trait Wrapper: Reflect {
    fn get(&self) -> &dyn Reflect;
    fn get_mut(&mut self) -> &mut dyn Reflect;
}

#[derive(Clone, Debug)]
pub struct WrapperInfo {
    type_name: &'static str,
    type_id: TypeId,
    inner_type_name: &'static str,
    inner_type_id: TypeId,
}

impl WrapperInfo {
    /// Create a new [`WrapperInfo`].
    pub fn new<TWrapper: Wrapper, Tinner: FromReflect>() -> Self {
        Self {
            type_name: std::any::type_name::<TWrapper>(),
            type_id: TypeId::of::<TWrapper>(),
            inner_type_name: std::any::type_name::<Tinner>(),
            inner_type_id: TypeId::of::<Tinner>(),
        }
    }

    /// The [type name] of the wrapper.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the wrapper.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the wrapper type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The [type name] of the inner type.
    ///
    /// [type name]: std::any::type_name
    pub fn inner_type_name(&self) -> &'static str {
        self.inner_type_name
    }

    /// The [`TypeId`] of the inner type.
    pub fn inner_type_id(&self) -> TypeId {
        self.inner_type_id
    }

    /// Check if the given type matches the wrapper inner type.
    pub fn inner_is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.inner_type_id
    }
}
