use std::{
    any::Any,
    hash::{Hash, Hasher},
};

use smol_str::SmolStr;

use crate::{
    utility::{reflect_hasher, GenericTypePathCell, NonGenericTypeInfoCell},
    FromReflect, FromType, GetTypeRegistration, Reflect, ReflectFromPtr, ReflectFromReflect,
    ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath, TypeRegistration, Typed, ValueInfo,
};

impl Reflect for SmolStr {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        let value = value.as_any();

        if let Some(value) = value.downcast_ref::<SmolStr>() {
            *self = value.clone();
        } else if let Some(value) = value.downcast_ref::<String>() {
            *self = SmolStr::new(value);
        } else {
            panic!(
                "Value is not a {} nor a {}.",
                std::any::type_name::<Self>(),
                std::any::type_name::<String>()
            );
        }
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Value(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Value(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Value(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&std::any::Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        let value = value.as_any();
        if let Some(value) = value.downcast_ref::<Self>() {
            Some(std::cmp::PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }
}

impl Typed for SmolStr {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Value(ValueInfo::new::<Self>()))
    }
}

impl TypePath for SmolStr {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "smol_str::SmolStr".to_owned())
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "SmolStr".to_owned())
    }

    fn type_ident() -> Option<&'static str> {
        Some("SmolStr")
    }

    fn crate_name() -> Option<&'static str> {
        Some("smol_str")
    }

    fn module_path() -> Option<&'static str> {
        Some("SmolStr")
    }
}

impl GetTypeRegistration for SmolStr {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
        registration
    }
}

impl FromReflect for SmolStr {
    fn from_reflect(reflect: &dyn crate::Reflect) -> Option<Self> {
        Some(reflect.as_any().downcast_ref::<SmolStr>()?.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::{FromReflect, Reflect};
    use smol_str::SmolStr;

    #[test]
    fn should_partial_eq_smolstr() {
        let a: &dyn Reflect = &SmolStr::new("A");
        let a2: &dyn Reflect = &SmolStr::new("A");
        let b: &dyn Reflect = &SmolStr::new("B");
        assert_eq!(Some(true), a.reflect_partial_eq(a2));
        assert_eq!(Some(false), a.reflect_partial_eq(b));
    }

    #[test]
    fn smolstr_should_from_reflect() {
        let smolstr = SmolStr::new("hello_world.rs");
        let output = <SmolStr as FromReflect>::from_reflect(&smolstr);
        assert_eq!(Some(smolstr), output);
    }
}
