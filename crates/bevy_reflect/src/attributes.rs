use crate::Reflect;
use alloc::borrow::Cow;
use bevy_utils::HashMap;
use core::fmt::{Debug, Formatter};
use core::hash::Hash;

#[derive(Debug, Default)]
pub struct CustomAttributes {
    attributes: HashMap<Cow<'static, str>, CustomAttribute>,
}

impl CustomAttributes {
    pub fn with_attribute<T: Reflect>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        value: T,
    ) -> Self {
        self.attributes
            .insert(name.into(), CustomAttribute::new(value));

        self
    }

    pub fn contains<K>(&self, name: &K) -> bool
    where
        Cow<'static, str>: core::borrow::Borrow<K>,
        K: Eq + Hash + ?Sized,
    {
        self.attributes.contains_key(name)
    }

    pub fn get<K>(&self, name: &K) -> Option<&CustomAttribute>
    where
        Cow<'static, str>: core::borrow::Borrow<K>,
        K: Eq + Hash + ?Sized,
    {
        self.attributes.get(name)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&str, &CustomAttribute)> {
        self.attributes.iter().map(|(k, v)| (k.as_ref(), v))
    }

    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

pub struct CustomAttribute {
    value: Box<dyn Reflect>,
}

impl CustomAttribute {
    pub fn new<T: Reflect>(value: T) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    pub fn value<T: Reflect>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }

    pub fn reflect_value(&self) -> &dyn Reflect {
        &*self.value
    }
}

impl Debug for CustomAttribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.value.debug(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;

    #[test]
    fn should_create_custom_attributes() {
        let attributes = CustomAttributes::default()
            .with_attribute("min", 0.0_f32)
            .with_attribute("max", 1.0_f32);

        let value = attributes.get("max").unwrap().value::<f32>();

        assert_eq!(Some(&1.0), value);
    }

    #[test]
    fn should_debug_custom_attributes() {
        let attributes = CustomAttributes::default()
            .with_attribute("label", String::from("My awesome custom attribute!"));

        let debug = format!("{:?}", attributes);

        assert_eq!(
            r#"CustomAttributes { attributes: {"label": "My awesome custom attribute!"} }"#,
            debug
        );

        #[derive(Reflect)]
        struct Foo {
            value: i32,
        }

        let attributes = CustomAttributes::default().with_attribute("foo", Foo { value: 42 });

        let debug = format!("{:?}", attributes);

        assert_eq!(
            r#"CustomAttributes { attributes: {"foo": bevy_reflect::attributes::tests::Foo { value: 42 }} }"#,
            debug
        );
    }
}
