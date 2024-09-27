use crate::type_info::impl_type_methods;
use crate::{Reflect, Type, TypePath};
use alloc::borrow::Cow;
use alloc::sync::Arc;
use core::ops::Deref;

#[derive(Clone, Default, Debug)]
pub struct Generics(Box<[GenericInfo]>);

impl Generics {
    pub fn new() -> Self {
        Self(Box::new([]))
    }

    pub fn get_named(&self, name: &str) -> Option<&GenericInfo> {
        // For small sets of generics (the most common case),
        // a linear search is often faster using a `HashMap`.
        self.0.iter().find(|info| info.name() == name)
    }
}

impl FromIterator<GenericInfo> for Generics {
    fn from_iter<T: IntoIterator<Item = GenericInfo>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Deref for Generics {
    type Target = [GenericInfo];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub enum GenericInfo {
    Type(TypeParamInfo),
    Const(ConstParamInfo),
}

impl GenericInfo {
    pub fn name(&self) -> &Cow<'static, str> {
        match self {
            Self::Type(info) => info.name(),
            Self::Const(info) => info.name(),
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Type(_) => false,
            Self::Const(_) => true,
        }
    }

    impl_type_methods!(self => {
        match self {
            Self::Type(info) => info.ty(),
            Self::Const(info) => info.ty(),
        }
    });
}

#[derive(Clone, Debug)]
pub struct TypeParamInfo {
    name: Cow<'static, str>,
    ty: Type,
    default: Option<Type>,
}

impl TypeParamInfo {
    pub fn new<T: TypePath + ?Sized>(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            ty: Type::of::<T>(),
            default: None,
        }
    }

    pub fn with_default<T: TypePath + ?Sized>(mut self) -> Self {
        self.default = Some(Type::of::<T>());
        self
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn default(&self) -> Option<&Type> {
        self.default.as_ref()
    }

    impl_type_methods!(ty);
}

#[derive(Clone, Debug)]
pub struct ConstParamInfo {
    name: Cow<'static, str>,
    ty: Type,
    // Rust currently only allows certain primitive types in const generic position,
    // meaning that `Reflect` is guaranteed to be implemented for the default value.
    default: Option<Arc<dyn Reflect>>,
}

impl ConstParamInfo {
    pub fn new<T: TypePath + ?Sized>(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            ty: Type::of::<T>(),
            default: None,
        }
    }

    pub fn with_default<T: Reflect + 'static>(mut self, default: T) -> Self {
        self.default = Some(Arc::new(default));
        self
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn default(&self) -> Option<&dyn Reflect> {
        self.default.as_deref()
    }

    impl_type_methods!(ty);
}

macro_rules! impl_generic_info_methods {
    ($field:ident) => {
        pub fn with_generics(mut self, generics: crate::generics::Generics) -> Self {
            self.$field = generics;
            self
        }

        pub fn generics(&self) -> &crate::generics::Generics {
            &self.$field
        }
    };
}

pub(crate) use impl_generic_info_methods;

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::{Reflect, Typed};
    use core::fmt::Debug;

    #[test]
    fn should_maintain_order() {
        #[derive(Reflect)]
        struct Test<T, U: Debug, const N: usize>([(T, U); N]);

        let generics = <Test<f32, String, 10> as Typed>::type_info()
            .as_tuple_struct()
            .unwrap()
            .generics();

        assert_eq!(generics.len(), 3);

        let mut iter = generics.iter();

        let t = iter.next().unwrap();
        assert_eq!(t.name(), "T");
        assert!(t.ty().is::<f32>());
        assert!(!t.is_const());

        let u = iter.next().unwrap();
        assert_eq!(u.name(), "U");
        assert!(u.ty().is::<String>());
        assert!(!u.is_const());

        let n = iter.next().unwrap();
        assert_eq!(n.name(), "N");
        assert!(n.ty().is::<usize>());
        assert!(n.is_const());

        assert!(iter.next().is_none());
    }

    #[test]
    fn should_get_by_name() {
        #[derive(Reflect)]
        enum Test<T, U: Debug, const N: usize> {
            Array([(T, U); N]),
        }

        let generics = <Test<f32, String, 10> as Typed>::type_info()
            .as_enum()
            .unwrap()
            .generics();

        let t = generics.get_named("T").unwrap();
        assert_eq!(t.name(), "T");
        assert!(t.ty().is::<f32>());
        assert!(!t.is_const());

        let u = generics.get_named("U").unwrap();
        assert_eq!(u.name(), "U");
        assert!(u.ty().is::<String>());
        assert!(!u.is_const());

        let n = generics.get_named("N").unwrap();
        assert_eq!(n.name(), "N");
        assert!(n.ty().is::<usize>());
        assert!(n.is_const());
    }

    #[test]
    fn should_store_defaults() {
        #[derive(Reflect)]
        struct Test<T, U: Debug = String, const N: usize = 10>([(T, U); N]);

        let generics = <Test<f32> as Typed>::type_info()
            .as_tuple_struct()
            .unwrap()
            .generics();

        let GenericInfo::Type(u) = generics.get_named("U").unwrap() else {
            panic!("expected a type parameter");
        };
        assert_eq!(u.default().unwrap(), &Type::of::<String>());

        let GenericInfo::Const(n) = generics.get_named("N").unwrap() else {
            panic!("expected a const parameter");
        };
        assert_eq!(n.default().unwrap().downcast_ref::<usize>().unwrap(), &10);
    }
}
