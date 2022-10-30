use std::any::{Any, TypeId};

use crate::{
    utility::NonGenericTypeInfoCell, DynamicInfo, Reflect, ReflectMut, ReflectRef, TypeInfo, Typed,
};

pub trait Wrapper: Reflect {
    fn get(&self) -> &dyn Reflect;

    fn get_mut(&mut self) -> &mut dyn Reflect;

    fn clone_dynamic(&self) -> DynamicWrapper {
        DynamicWrapper {
            name: self.type_name().to_string(),
            value: self.get().clone_value(),
        }
    }
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
    pub fn new<TWrapper: Wrapper, TInner: 'static>() -> Self {
        Self {
            type_name: std::any::type_name::<TWrapper>(),
            type_id: TypeId::of::<TWrapper>(),
            inner_type_name: std::any::type_name::<TInner>(),
            inner_type_id: TypeId::of::<TInner>(),
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

pub struct DynamicWrapper {
    name: String,
    value: Box<dyn Reflect>,
}
impl DynamicWrapper {
    pub fn new(name: String, value: Box<dyn Reflect>) -> DynamicWrapper {
        DynamicWrapper { name, value }
    }

    /// Returns the type name of the wrapper.
    ///
    /// The value returned by this method is the same value returned by
    /// [`Reflect::type_name`].
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the type name of the wrapper.
    ///
    /// The value set by this method is the value returned by
    /// [`Reflect::type_name`].
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Appends an element with value `value` to the tuple struct.
    pub fn insert_boxed(&mut self, value: Box<dyn Reflect>) {
        self.value = value;
    }

    /// Appends a typed element with value `value` to the tuple struct.
    pub fn insert<T: Reflect>(&mut self, value: T) {
        self.insert_boxed(Box::new(value));
    }
}

impl Wrapper for DynamicWrapper {
    fn get(&self) -> &dyn Reflect {
        &*self.value
    }

    fn get_mut(&mut self) -> &mut dyn Reflect {
        &mut *self.value
    }

    fn clone_dynamic(&self) -> DynamicWrapper {
        DynamicWrapper {
            name: self.name.clone(),
            value: self.value.clone_value(),
        }
    }
}

impl Typed for DynamicWrapper {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Dynamic(DynamicInfo::new::<Self>()))
    }
}

impl Reflect for DynamicWrapper {
    fn type_name(&self) -> &str {
        self.name()
    }

    fn get_type_info(&self) -> &'static crate::TypeInfo {
        <Self as Typed>::type_info()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        match value.reflect_ref() {
            ReflectRef::Wrapper(wrapper) => self.get_mut().apply(wrapper.get()),
            _ => panic!("Attempted to apply a non-wrapper type to a wrapper type."),
        }
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Wrapper(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Wrapper(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(Wrapper::clone_dynamic(self))
    }
}
