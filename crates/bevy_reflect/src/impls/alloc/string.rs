use crate::{
    std_traits::ReflectDefault,
    type_registry::{ReflectDeserialize, ReflectSerialize},
};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::alloc::string::String(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use bevy_reflect::PartialReflect;

    #[test]
    fn should_partial_eq_string() {
        let a: &dyn PartialReflect = &String::from("Hello");
        let b: &dyn PartialReflect = &String::from("Hello");
        let c: &dyn PartialReflect = &String::from("World");
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }
}
