use core::fmt::Formatter;
use std::any::Any;

use crate::__macro_exports::RegisterForReflection;
use crate::serde::Serializable;
use crate::utility::GenericTypePathCell;
use crate::{
    ApplyError, MaybeTyped, PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned,
    ReflectRef, ReflectRemote, TypeInfo, TypePath,
};

/// A trait used to access `Self` as a [`dyn PartialReflect`].
///
/// This is used to provide by [`ReflectBox<T>`] in order to remotely reflect
/// the inner type, `T`.
///
/// This trait can be implemented on custom trait objects to allow them to be remotely reflected
/// by [`ReflectBox`].
///
/// [`dyn PartialReflect`]: PartialReflect
#[doc(hidden)]
pub trait ToPartialReflect: Send + Sync + 'static {
    /// Get a reference to `Self` as a [`&dyn PartialReflect`](PartialReflect).
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect;
    /// Get a mutable reference to `Self` as a [`&mut dyn PartialReflect`](PartialReflect).
    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect;
    /// Take `Self` as a [`Box<dyn PartialReflect>`](PartialReflect).
    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect>;
}

impl<T: PartialReflect> ToPartialReflect for T {
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
        self
    }

    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }
}

impl ToPartialReflect for dyn PartialReflect {
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
        self
    }

    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }
}

/// A trait used to access `Self` as a [`dyn Reflect`].
///
/// This is used to provide by [`ReflectBox<T>`] in order to remotely reflect
/// the inner type, `T`.
///
/// This trait can be implemented on custom trait objects to allow them to be remotely reflected
/// by [`ReflectBox`].
///
/// [`dyn Reflect`]: Reflect
#[doc(hidden)]
pub trait ToReflect: ToPartialReflect {
    /// Get a reference to `Self` as a [`&dyn Reflect`](Reflect).
    fn to_reflect_ref(&self) -> &dyn Reflect;
    /// Get a mutable reference to `Self` as a [`&mut dyn Reflect`](Reflect).
    fn to_reflect_mut(&mut self) -> &mut dyn Reflect;
    /// Take `Self` as a [`Box<dyn Reflect>`](Reflect).
    fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect>;
}

impl<T: Reflect> ToReflect for T {
    fn to_reflect_ref(&self) -> &dyn Reflect {
        self
    }

    fn to_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }
}

impl ToPartialReflect for dyn Reflect {
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
        self.as_partial_reflect()
    }

    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.as_partial_reflect_mut()
    }

    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
        self.into_partial_reflect()
    }
}

impl ToReflect for dyn Reflect {
    fn to_reflect_ref(&self) -> &dyn Reflect {
        self
    }

    fn to_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }
}

/// A [remote wrapper] type around [`Box<T>`] where `T` is either a type that implements [`PartialReflect`]/[`Reflect`]
/// or a [`dyn PartialReflect`]/[`dyn Reflect`] trait object.
///
/// `Box<T>` is not itself reflectable due to high likelihood of accidentally double-boxing it
/// (i.e. accidentally calling `Box::new` on an already-reflectable `Box<dyn Reflect>`).
///
/// [`ReflectBox`] should never be created or used directly outside remote reflection,
/// in order to avoid double-boxing.
///
/// # Examples
///
/// ```
/// # use bevy_reflect::Reflect;
/// use bevy_reflect::boxed::ReflectBox;
///
/// #[derive(Reflect)]
/// struct Sword {
///     attack: u32
/// }
///
/// #[derive(Reflect)]
/// // Keep in mind that `ReflectBox` does not implement `FromReflect`.
/// // Because of this, we will need to opt-out of the automatic `FromReflect` impl:
/// #[reflect(from_reflect = false)]
/// struct Player {
///     // Tell the `Reflect` derive to remotely reflect our `Box<dyn Reflect>`
///     // using the `ReflectBox<dyn Reflect>` wrapper type:
///     #[reflect(remote = ReflectBox<dyn Reflect>)]
///     weapon: Box<dyn Reflect>
/// }
///
/// let player = Player {
///     // We can create our `Box<dyn Reflect>` as normal:
///     weapon: Box::new(Sword { attack: 100 })
/// };
///
/// // Now we can reflect `weapon`!
/// // Under the hood, this `dyn Reflect` is actually a `ReflectBox<dyn Reflect>`,
/// // and is automatically delegating all reflection methods to the inner `dyn Reflect`.
/// // It will automatically convert to and from `ReflectBox<dyn Reflect>` as needed.
/// let weapon: &dyn Reflect = player.weapon.as_reflect();
/// assert!(weapon.reflect_partial_eq(&Sword { attack: 100 }).unwrap_or_default());
/// ```
///
/// [remote wrapper]: ReflectRemote
/// [`dyn PartialReflect`]: PartialReflect
/// [`dyn Reflect`]: Reflect
#[repr(transparent)]
pub struct ReflectBox<T: ?Sized + ToPartialReflect + TypePath>(Box<T>);

impl<T: ?Sized + ToPartialReflect + TypePath> ReflectRemote for ReflectBox<T> {
    type Remote = Box<T>;

    fn as_remote(&self) -> &Self::Remote {
        &self.0
    }

    fn as_remote_mut(&mut self) -> &mut Self::Remote {
        &mut self.0
    }

    fn into_remote(self) -> Self::Remote {
        self.0
    }

    fn as_wrapper(remote: &Self::Remote) -> &Self {
        // SAFETY: The wrapper type should be repr(transparent) over the remote type
        #[allow(unsafe_code)]
        unsafe {
            core::mem::transmute::<&Self::Remote, &Self>(remote)
        }
    }

    fn as_wrapper_mut(remote: &mut Self::Remote) -> &mut Self {
        // SAFETY: The wrapper type should be repr(transparent) over the remote type
        #[allow(unsafe_code)]
        unsafe {
            core::mem::transmute::<&mut Self::Remote, &mut Self>(remote)
        }
    }

    fn into_wrapper(remote: Self::Remote) -> Self {
        // SAFETY: The wrapper type should be repr(transparent) over the remote type
        #[allow(unsafe_code)]
        unsafe {
            // Unfortunately, we have to use `transmute_copy` to avoid a compiler error:
            // ```
            // error[E0512]: cannot transmute between types of different sizes, or dependently-sized types
            // |
            // |                 std::mem::transmute::<A, B>(a)
            // |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^
            // |
            // = note: source type: `A` (this type does not have a fixed size)
            // = note: target type: `B` (this type does not have a fixed size)
            // ```
            core::mem::transmute_copy::<Self::Remote, Self>(
                // `ManuallyDrop` is used to prevent double-dropping `self`
                &core::mem::ManuallyDrop::new(remote),
            )
        }
    }
}

/// All methods in this implementation are delegated to the inner type, `T`.
impl<T: ?Sized + ToPartialReflect + TypePath> PartialReflect for ReflectBox<T> {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.0.to_partial_reflect_ref().get_represented_type_info()
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self.0.to_partial_reflect_box()
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self.0.to_partial_reflect_ref()
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.0.to_partial_reflect_mut()
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        self.0.to_partial_reflect_box().try_into_reflect()
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        self.0.to_partial_reflect_ref().try_as_reflect()
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        self.0.to_partial_reflect_mut().try_as_reflect_mut()
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        self.0.to_partial_reflect_mut().apply(value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        self.0.to_partial_reflect_mut().try_apply(value)
    }

    fn reflect_kind(&self) -> ReflectKind {
        self.0.to_partial_reflect_ref().reflect_kind()
    }

    fn reflect_ref(&self) -> ReflectRef {
        self.0.to_partial_reflect_ref().reflect_ref()
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        self.0.to_partial_reflect_mut().reflect_mut()
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        self.0.to_partial_reflect_box().reflect_owned()
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        self.0.to_partial_reflect_ref().clone_value()
    }

    fn reflect_hash(&self) -> Option<u64> {
        self.0.to_partial_reflect_ref().reflect_hash()
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        self.0.to_partial_reflect_ref().reflect_partial_eq(value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "ReflectBox(")?;
        self.0.to_partial_reflect_ref().debug(f)?;
        write!(f, ")")
    }

    fn serializable(&self) -> Option<Serializable> {
        self.0.to_partial_reflect_ref().serializable()
    }

    fn is_dynamic(&self) -> bool {
        self.0.to_partial_reflect_ref().is_dynamic()
    }
}

/// All methods in this implementation are delegated to the inner type, `T`.
impl<T: ?Sized + ToReflect + TypePath> Reflect for ReflectBox<T> {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self.0.to_reflect_box().into_any()
    }

    fn as_any(&self) -> &dyn Any {
        self.0.to_reflect_ref().as_any()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.0.to_reflect_mut().as_any_mut()
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self.0.to_reflect_box()
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self.0.to_reflect_ref()
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self.0.to_reflect_mut()
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        self.0.to_reflect_mut().set(value)
    }
}

/// For most reflection methods, [`ReflectBox`] will delegate its behavior to the type, `T`, that it wraps.
/// However, due to the static nature of [`TypePath`], [`ReflectBox`] must provide its own implementation.
///
/// # Examples
///
/// ```
/// # use bevy_reflect::boxed::ReflectBox;
/// # use bevy_reflect::{PartialReflect, TypePath};
/// // Notice that we don't delegate to the `TypePath` implementation for `String`:
/// let type_path = <ReflectBox<String> as TypePath>::type_path();
/// assert_eq!(type_path, "bevy_reflect::boxed::ReflectBox<alloc::string::String>");
///
/// // The same is true for trait object types like `dyn PartialReflect`:
/// let type_path = <ReflectBox<dyn PartialReflect> as TypePath>::type_path();
/// assert_eq!(type_path, "bevy_reflect::boxed::ReflectBox<dyn bevy_reflect::PartialReflect>");
/// ```
impl<T: ?Sized + ToPartialReflect + TypePath> TypePath for ReflectBox<T> {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            "bevy_reflect::boxed::ReflectBox<".to_owned() + T::type_path() + ">"
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "ReflectBox<".to_owned() + T::short_type_path() + ">")
    }

    fn type_ident() -> Option<&'static str> {
        Some("ReflectBox")
    }

    fn crate_name() -> Option<&'static str> {
        Some("bevy_reflect")
    }

    fn module_path() -> Option<&'static str> {
        Some("bevy_reflect::boxed")
    }
}

impl<T: ?Sized + ToPartialReflect + TypePath> MaybeTyped for ReflectBox<T> {}
impl<T: ?Sized + ToPartialReflect + TypePath> RegisterForReflection for ReflectBox<T> {}

#[cfg(feature = "functions")]
mod func {
    use super::*;
    use crate::func::args::{Arg, FromArg, GetOwnership, Ownership};
    use crate::func::{ArgError, IntoReturn, Return};

    impl<T: ?Sized + ToReflect + TypePath> GetOwnership for ReflectBox<T> {
        fn ownership() -> Ownership {
            Ownership::Owned
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> GetOwnership for &'_ ReflectBox<T> {
        fn ownership() -> Ownership {
            Ownership::Ref
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> GetOwnership for &'_ mut ReflectBox<T> {
        fn ownership() -> Ownership {
            Ownership::Mut
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> FromArg for ReflectBox<T> {
        type This<'a> = ReflectBox<T>;

        fn from_arg(arg: Arg) -> Result<Self::This<'_>, ArgError> {
            arg.take_owned()
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> FromArg for &'static ReflectBox<T> {
        type This<'a> = &'a ReflectBox<T>;

        fn from_arg(arg: Arg) -> Result<Self::This<'_>, ArgError> {
            arg.take_ref()
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> FromArg for &'static mut ReflectBox<T> {
        type This<'a> = &'a mut ReflectBox<T>;

        fn from_arg(arg: Arg) -> Result<Self::This<'_>, ArgError> {
            arg.take_mut()
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> IntoReturn for ReflectBox<T> {
        fn into_return<'into_return>(self) -> Return<'into_return>
        where
            Self: 'into_return,
        {
            Return::Owned(Box::new(self))
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> IntoReturn for &ReflectBox<T> {
        fn into_return<'into_return>(self) -> Return<'into_return>
        where
            Self: 'into_return,
        {
            Return::Ref(self)
        }
    }

    impl<T: ?Sized + ToReflect + TypePath> IntoReturn for &mut ReflectBox<T> {
        fn into_return<'into_return>(self) -> Return<'into_return>
        where
            Self: 'into_return,
        {
            Return::Mut(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::serde::{ReflectDeserializer, ReflectSerializer};
    use crate::{DynamicStruct, FromReflect, GetTypeRegistration, Struct, TypeRegistry, Typed};
    use serde::de::DeserializeSeed;
    use static_assertions::assert_not_impl_any;

    #[test]
    fn box_should_not_be_reflect() {
        assert_not_impl_any!(Box<i32>: PartialReflect, Reflect, FromReflect, TypePath, Typed, GetTypeRegistration);
    }

    #[test]
    fn should_reflect_box() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct Container {
            #[reflect(remote = ReflectBox<dyn PartialReflect>)]
            partial_reflect: Box<dyn PartialReflect>,
            #[reflect(remote = ReflectBox<dyn Reflect>)]
            full_reflect: Box<dyn Reflect>,
            #[reflect(remote = ReflectBox<i32>)]
            concrete: Box<i32>,
        }

        let mut container: Box<dyn Struct> = Box::new(Container {
            partial_reflect: Box::new(123),
            full_reflect: Box::new(456),
            concrete: Box::new(789),
        });

        let partial_reflect = container.field("partial_reflect").unwrap();
        assert!(partial_reflect.reflect_partial_eq(&123).unwrap_or_default());

        let full_reflect = container.field("full_reflect").unwrap();
        assert!(full_reflect.reflect_partial_eq(&456).unwrap_or_default());

        let concrete = container.field("concrete").unwrap();
        assert!(concrete.reflect_partial_eq(&789).unwrap_or_default());

        let mut patch = DynamicStruct::default();
        patch.insert("partial_reflect", ReflectBox(Box::new(321)));
        patch.insert("full_reflect", ReflectBox(Box::new(654)));
        patch.insert("concrete", ReflectBox(Box::new(987)));

        container.apply(&patch);

        let partial_reflect = container.field("partial_reflect").unwrap();
        assert!(partial_reflect.reflect_partial_eq(&321).unwrap_or_default());

        let full_reflect = container.field("full_reflect").unwrap();
        assert!(full_reflect.reflect_partial_eq(&654).unwrap_or_default());

        let concrete = container.field("concrete").unwrap();
        assert!(concrete.reflect_partial_eq(&987).unwrap_or_default());
    }

    #[test]
    fn should_debug_reflect_box() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct Container {
            #[reflect(remote = ReflectBox<dyn PartialReflect>)]
            partial_reflect: Box<dyn PartialReflect>,
            #[reflect(remote = ReflectBox<dyn Reflect>)]
            full_reflect: Box<dyn Reflect>,
            #[reflect(remote = ReflectBox<i32>)]
            concrete: Box<i32>,
        }

        let container: Box<dyn PartialReflect> = Box::new(Container {
            partial_reflect: Box::new(123),
            full_reflect: Box::new(456),
            concrete: Box::new(789),
        });

        let debug = format!("{:?}", container.as_partial_reflect());
        assert_eq!(
            debug,
            "bevy_reflect::boxed::tests::Container { partial_reflect: ReflectBox(123), full_reflect: ReflectBox(456), concrete: ReflectBox(789) }"
        );

        // Double-boxing should be near impossible for a user to create intentionally or unintentionally.
        // However, we'll still test it here so that we can keep track of its behavior.
        let double_box: Box<dyn PartialReflect> =
            Box::new(ReflectBox(Box::new(ReflectBox(Box::new(123)))));

        let deref_debug = format!("{:?}", &*double_box);
        assert_eq!(deref_debug, "ReflectBox(ReflectBox(123))");

        let as_partial_reflect_debug = format!("{:?}", double_box.as_partial_reflect());
        assert_eq!(as_partial_reflect_debug, "ReflectBox(123)");
    }

    #[test]
    fn should_avoid_double_boxing() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct Container {
            #[reflect(remote = ReflectBox<dyn PartialReflect>)]
            value: Box<dyn PartialReflect>,
        }

        let container = Box::new(Container {
            value: Box::new(Some(123)),
        });

        let cloned = container.clone_value().clone_value();

        let ReflectRef::Struct(cloned) = cloned.reflect_ref() else {
            panic!("expected `ReflectRef::Struct`");
        };

        let value = cloned.field("value").unwrap();
        let debug = format!("{:?}", value);
        assert_eq!(debug, "DynamicEnum(Some(123))");

        // Cloning a `ReflectBox` directly. Users should never have an instance of `ReflectBox`.
        let value = ReflectBox(Box::new(Some(123)));
        let cloned = value.clone_value();
        let debug = format!("{:?}", cloned);
        assert_eq!(debug, "DynamicEnum(Some(123))");

        // Cloning a boxed `ReflectBox`. Users should never have an instance of `ReflectBox`.
        let value = Box::new(ReflectBox(Box::new(Some(123))));
        let cloned = value.clone_value();
        let debug = format!("{:?}", cloned);
        assert_eq!(debug, "DynamicEnum(Some(123))");
    }

    #[test]
    fn should_allow_custom_trait_objects() {
        trait Equippable: Reflect {}

        impl TypePath for dyn Equippable {
            fn type_path() -> &'static str {
                todo!()
            }

            fn short_type_path() -> &'static str {
                todo!()
            }
        }

        impl ToPartialReflect for dyn Equippable {
            fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
                self.as_partial_reflect()
            }

            fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
                self.as_partial_reflect_mut()
            }

            fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
                self.into_partial_reflect()
            }
        }

        impl ToReflect for dyn Equippable {
            fn to_reflect_ref(&self) -> &dyn Reflect {
                self.as_reflect()
            }

            fn to_reflect_mut(&mut self) -> &mut dyn Reflect {
                self.as_reflect_mut()
            }

            fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect> {
                self.into_reflect()
            }
        }

        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct Player {
            #[reflect(remote = ReflectBox<dyn Equippable>)]
            weapon: Box<dyn Equippable>,
        }
    }

    #[test]
    fn should_serialize_reflect_box() {
        let input = ReflectBox(Box::new(123));

        let registry = TypeRegistry::new();

        let reflect_serializer = ReflectSerializer::new(&input, &registry);
        let serialized = ron::to_string(&reflect_serializer).unwrap();
        assert_eq!(serialized, r#"{"i32":123}"#);

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let output = reflect_deserializer
            .deserialize(&mut ron::Deserializer::from_str(&serialized).unwrap())
            .unwrap();
        assert!(
            output
                .as_partial_reflect()
                .reflect_partial_eq(&input)
                .unwrap_or_default(),
            "serialization roundtrip should be lossless"
        );
    }

    #[test]
    fn should_serialize_reflect_box_struct() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct {
            #[reflect(remote = ReflectBox<dyn PartialReflect>)]
            partial_reflect: Box<dyn PartialReflect>,
            #[reflect(remote = ReflectBox<dyn Reflect>)]
            full_reflect: Box<dyn Reflect>,
            #[reflect(remote = ReflectBox<i32>)]
            concrete: Box<i32>,
        }

        let input = MyStruct {
            partial_reflect: Box::new(123),
            full_reflect: Box::new(456),
            concrete: Box::new(789),
        };

        let mut registry = TypeRegistry::new();
        registry.register::<MyStruct>();

        let reflect_serializer = ReflectSerializer::new(&input, &registry);
        let serialized = ron::to_string(&reflect_serializer).unwrap();
        assert_eq!(
            serialized,
            r#"{"bevy_reflect::boxed::tests::MyStruct":(partial_reflect:{"i32":123},full_reflect:{"i32":456},concrete:{"i32":789})}"#
        );

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let output = reflect_deserializer
            .deserialize(&mut ron::Deserializer::from_str(&serialized).unwrap())
            .unwrap();
        assert!(
            output
                .as_partial_reflect()
                .reflect_partial_eq(&input)
                .unwrap_or_default(),
            "serialization roundtrip should be lossless"
        );
    }

    #[test]
    fn should_serialize_reflect_box_tuple_struct() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyTupleStruct(
            #[reflect(remote = ReflectBox <dyn PartialReflect>)] Box<dyn PartialReflect>,
            #[reflect(remote = ReflectBox <dyn Reflect>)] Box<dyn Reflect>,
            #[reflect(remote = ReflectBox <i32>)] Box<i32>,
        );

        let input = MyTupleStruct(Box::new(123), Box::new(456), Box::new(789));

        let mut registry = TypeRegistry::new();
        registry.register::<MyTupleStruct>();

        let reflect_serializer = ReflectSerializer::new(&input, &registry);
        let serialized = ron::to_string(&reflect_serializer).unwrap();
        assert_eq!(
            serialized,
            r#"{"bevy_reflect::boxed::tests::MyTupleStruct":({"i32":123},{"i32":456},{"i32":789})}"#
        );

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let output = reflect_deserializer
            .deserialize(&mut ron::Deserializer::from_str(&serialized).unwrap())
            .unwrap();
        assert!(
            output
                .as_partial_reflect()
                .reflect_partial_eq(&input)
                .unwrap_or_default(),
            "serialization roundtrip should be lossless"
        );
    }

    #[test]
    fn should_serialize_nested_reflect_box() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct {
            #[reflect(remote = ReflectBox<dyn PartialReflect>)]
            partial_reflect: Box<dyn PartialReflect>,
            #[reflect(remote = ReflectBox<dyn Reflect>)]
            full_reflect: Box<dyn Reflect>,
            #[reflect(remote = ReflectBox<i32>)]
            concrete: Box<i32>,
        }

        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyNestedStruct {
            #[reflect(remote = ReflectBox<MyStruct>)]
            inner: Box<MyStruct>,
        }

        let input = MyNestedStruct {
            inner: Box::new(MyStruct {
                partial_reflect: Box::new(123),
                full_reflect: Box::new(456),
                concrete: Box::new(789),
            }),
        };

        let mut registry = TypeRegistry::new();
        registry.register::<MyStruct>();
        registry.register::<MyNestedStruct>();

        let reflect_serializer = ReflectSerializer::new(&input, &registry);
        let serialized = ron::to_string(&reflect_serializer).unwrap();
        assert_eq!(
            serialized,
            r#"{"bevy_reflect::boxed::tests::MyNestedStruct":(inner:{"bevy_reflect::boxed::tests::MyStruct":(partial_reflect:{"i32":123},full_reflect:{"i32":456},concrete:{"i32":789})})}"#
        );

        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let output = reflect_deserializer
            .deserialize(&mut ron::Deserializer::from_str(&serialized).unwrap())
            .unwrap();
        assert!(
            output
                .as_partial_reflect()
                .reflect_partial_eq(&input)
                .unwrap_or_default(),
            "serialization roundtrip should be lossless"
        );
    }
}
