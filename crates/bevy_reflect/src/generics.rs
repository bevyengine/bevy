use crate::{Reflect, Type, TypePath};
use alloc::{borrow::Cow, boxed::Box};
use bevy_platform::sync::Arc;
use core::ops::Deref;
use derive_more::derive::From;

macro_rules! impl_generic_type_methods {
    // Generates the type methods based off a single field.
    ($field:ident) => {
        impl_generic_type_methods!(self => {
            self.$field.as_ref()
        });
    };
    // Generates the type methods based off a custom expression.
    ($self:ident => $expr:expr) => {
        /// The underlying Rust [type].
        ///
        /// Note: If the type was not originally provided (possibly due to reflection opt-outs),
        /// then this method will return `None`.
        ///
        /// [type]: crate::type_info::Type
        pub fn ty(&$self) -> core::option::Option<&$crate::type_info::Type> {
            $expr
        }

        /// The [`TypeId`] of this type.
        ///
        /// Note: If the type was not originally provided (possibly due to reflection opt-outs),
        /// then this method will return `None`.
        ///
        /// [`TypeId`]: core::any::TypeId
        pub fn type_id(&self) -> core::option::Option<::core::any::TypeId> {
            Some(self.ty()?.id())
        }

        /// The [stable, full type path] of this type.
        ///
        /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
        ///
        /// Note: If the type was not originally provided (possibly due to reflection opt-outs),
        /// then this method will return `None`.
        ///
        /// [stable, full type path]: TypePath
        /// [`type_path_table`]: Self::type_path_table
        pub fn type_path(&self) -> core::option::Option<&'static str> {
            Some(self.ty()?.path())
        }

        /// A representation of the type path of this type.
        ///
        /// Provides dynamic access to all methods on [`TypePath`].
        ///
        /// Note: If the type was not originally provided (possibly due to reflection opt-outs),
        /// then this method will return `None`.
        ///
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn type_path_table(&self) -> core::option::Option<&$crate::type_path::TypePathTable> {
            Some(&self.ty()?.type_path_table())
        }

        /// Check if the given type matches this one.
        ///
        /// This only compares the [`TypeId`] of the types
        /// and does not verify they share the same [`TypePath`]
        /// (though it implies they do).
        ///
        /// Note: If the type was not originally provided (possibly due to reflection opt-outs),
        /// then this method will return `None`.
        ///
        /// [`TypeId`]: core::any::TypeId
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn is<T: ::core::any::Any>(&self) -> Option<bool> {
            Some(self.ty()?.is::<T>())
        }
    };
}

/// The generic parameters of a type.
///
/// This is automatically generated via the [`Reflect` derive macro]
/// and stored on the [`TypeInfo`] returned by [`Typed::type_info`]
/// for types that have generics.
///
/// It supports both type parameters and const parameters
/// so long as they implement [`TypePath`].
///
/// If the type has no generics, this will be empty.
///
/// If the type is marked with `#[reflect(type_path = false)]`,
/// the generics will only their capture the parameter names and their [`Type`] information.
///
/// [`Reflect` derive macro]: bevy_reflect_derive::Reflect
/// [`TypeInfo`]: crate::type_info::TypeInfo
/// [`Typed::type_info`]: crate::Typed::type_info
#[derive(Clone, Default, Debug)]
pub struct Generics(Box<[GenericInfo]>);

impl Generics {
    /// Creates an empty set of generics.
    pub fn new() -> Self {
        Self(Box::new([]))
    }

    /// Finds the generic parameter with the given name.
    ///
    /// Returns `None` if no such parameter exists.
    pub fn get_named(&self, name: &str) -> Option<&GenericInfo> {
        // For small sets of generics (the most common case),
        // a linear search is often faster using a `HashMap`.
        self.0.iter().find(|info| info.name() == name)
    }

    /// Adds the given generic parameter to the set.
    pub fn with(mut self, info: impl Into<GenericInfo>) -> Self {
        self.0 = IntoIterator::into_iter(self.0)
            .chain(core::iter::once(info.into()))
            .collect();
        self
    }
}

impl<T: Into<GenericInfo>> FromIterator<T> for Generics {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}

impl Deref for Generics {
    type Target = [GenericInfo];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An enum representing a generic parameter.
#[derive(Clone, Debug, From)]
pub enum GenericInfo {
    /// A type parameter.
    ///
    /// An example would be `T` in `struct Foo<T, U>`.
    Type(TypeParamInfo),
    /// A const parameter.
    ///
    /// An example would be `N` in `struct Foo<const N: usize>`.
    Const(ConstParamInfo),
}

impl GenericInfo {
    /// The name of the generic parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        match self {
            Self::Type(info) => info.name(),
            Self::Const(info) => info.name(),
        }
    }

    /// Whether the generic parameter is a const parameter.
    pub fn is_const(&self) -> bool {
        match self {
            Self::Type(_) => false,
            Self::Const(_) => true,
        }
    }

    impl_generic_type_methods!(self => {
        match self {
            Self::Type(info) => info.ty(),
            Self::Const(info) => info.ty(),
        }
    });
}

/// Type information for a generic type parameter.
///
/// An example of a type parameter would be `T` in `struct Foo<T>`.
#[derive(Clone, Debug)]
pub struct TypeParamInfo {
    name: Cow<'static, str>,
    ty: Option<Type>,
    default: Option<Type>,
}

impl TypeParamInfo {
    /// Creates a new type parameter with the given name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            ty: None,
            default: None,
        }
    }

    /// Add a type to the parameter.
    pub fn with_type<T: TypePath + ?Sized>(self) -> Self {
        Self {
            ty: Some(Type::of::<T>()),
            ..self
        }
    }

    /// Sets the default type for the parameter.
    pub fn with_default<T: TypePath + ?Sized>(mut self) -> Self {
        self.default = Some(Type::of::<T>());
        self
    }

    /// The name of the type parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// The default type for the parameter, if any.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{GenericInfo, Reflect, Typed};
    /// #[derive(Reflect)]
    /// struct Foo<T = f32>(T);
    ///
    /// let generics = Foo::<String>::type_info().generics();
    /// let GenericInfo::Type(info) = generics.get_named("T").unwrap() else {
    ///     panic!("expected a type parameter");
    /// };
    ///
    /// let default = info.default().unwrap();
    ///
    /// assert!(default.is::<f32>());
    /// ```
    pub fn default(&self) -> Option<&Type> {
        self.default.as_ref()
    }

    impl_generic_type_methods!(ty);
}

/// Type information for a const generic parameter.
///
/// An example of a const parameter would be `N` in `struct Foo<const N: usize>`.
#[derive(Clone, Debug)]
pub struct ConstParamInfo {
    name: Cow<'static, str>,
    ty: Option<Type>,
    // Rust currently only allows certain primitive types in const generic position,
    // meaning that `Reflect` is guaranteed to be implemented for the default value.
    default: Option<Arc<dyn Reflect>>,
}

impl ConstParamInfo {
    /// Creates a new const parameter with the given name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            ty: None,
            default: None,
        }
    }

    /// Add a type to the parameter.
    pub fn with_type<T: TypePath + ?Sized>(self) -> Self {
        Self {
            ty: Some(Type::of::<T>()),
            ..self
        }
    }

    /// Sets the default value for the parameter.
    pub fn with_default<T: Reflect + 'static>(mut self, default: T) -> Self {
        let arc = Arc::new(default);

        #[cfg(not(target_has_atomic = "ptr"))]
        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // SAFETY:
        // - Coercion from `T` to `dyn Reflect` is valid as `T: Reflect + 'static`
        // - `Arc::from_raw` receives a valid pointer from a previous call to `Arc::into_raw`
        let arc = unsafe { Arc::from_raw(Arc::into_raw(arc) as *const dyn Reflect) };

        self.default = Some(arc);
        self
    }

    /// The name of the const parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// The default value for the parameter, if any.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::{GenericInfo, Reflect, Typed};
    /// #[derive(Reflect)]
    /// struct Foo<const N: usize = 10>([u8; N]);
    ///
    /// let generics = Foo::<5>::type_info().generics();
    /// let GenericInfo::Const(info) = generics.get_named("N").unwrap() else {
    ///    panic!("expected a const parameter");
    /// };
    ///
    /// let default = info.default().unwrap();
    ///
    /// assert_eq!(default.downcast_ref::<usize>().unwrap(), &10);
    /// ```
    pub fn default(&self) -> Option<&dyn Reflect> {
        self.default.as_deref()
    }

    impl_generic_type_methods!(ty);
}

macro_rules! impl_generic_info_methods {
    // Implements both getter and setter methods for the given field.
    ($field:ident) => {
        $crate::generics::impl_generic_info_methods!(self => &self.$field);

        /// Sets the generic parameters for this type.
        pub fn with_generics(mut self, generics: crate::generics::Generics) -> Self {
            self.$field = generics;
            self
        }
    };
    // Implements only a getter method for the given expression.
    ($self:ident => $expr:expr) => {
        /// Gets the generic parameters for this type.
        pub fn generics(&$self) -> &crate::generics::Generics {
            $expr
        }
    };
}

pub(crate) use impl_generic_info_methods;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Reflect, Typed};
    use alloc::string::String;
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
        assert!(t.is::<f32>().unwrap());
        assert!(!t.is_const());

        let u = iter.next().unwrap();
        assert_eq!(u.name(), "U");
        assert!(u.is::<String>().unwrap());
        assert!(!u.is_const());

        let n = iter.next().unwrap();
        assert_eq!(n.name(), "N");
        assert!(n.is::<usize>().unwrap());
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
        assert!(t.is::<f32>().unwrap());
        assert!(!t.is_const());

        let u = generics.get_named("U").unwrap();
        assert_eq!(u.name(), "U");
        assert!(u.is::<String>().unwrap());
        assert!(!u.is_const());

        let n = generics.get_named("N").unwrap();
        assert_eq!(n.name(), "N");
        assert!(n.is::<usize>().unwrap());
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

    #[test]
    fn should_store_untyped_generics_with_type_path_opt_out() {
        #[derive(Reflect)]
        #[reflect(type_path = false)]
        struct Test<T: Default, const N: usize = 10>(#[reflect(ignore)] T);

        impl<T: Default + 'static, const N: usize> TypePath for Test<T, N> {
            fn type_path() -> &'static str {
                ::core::any::type_name::<Self>()
            }

            fn short_type_path() -> &'static str {
                "Test<T, N>"
            }
        }

        let generics = <Test<f32> as Typed>::type_info()
            .as_tuple_struct()
            .unwrap()
            .generics();

        let t = generics.get_named("T").unwrap();
        assert_eq!(t.name(), "T");
        assert_eq!(t.ty(), None);
        assert!(!t.is_const());

        let n = generics.get_named("N").unwrap();
        assert_eq!(n.name(), "N");
        assert_eq!(n.ty(), None);
        assert!(n.is_const());
    }
}
