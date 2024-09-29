use crate::__macro_exports::RegisterForReflection;
use crate::utility::GenericTypeInfoCell;
use crate::{
    GetTypeRegistration, MaybeTyped, OpaqueInfo, PartialReflect, Reflect, TypeInfo, TypePath,
    TypeRegistration, Typed,
};
use alloc::boxed::Box;
use bevy_reflect_derive::impl_type_path;

pub trait CastPartialReflect: Send + Sync + 'static {
    fn as_partial_reflect(&self) -> &dyn PartialReflect;
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect;
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect>;
}

impl<T: CastPartialReflect> CastPartialReflect for Box<T> {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        T::as_partial_reflect(self)
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        T::as_partial_reflect_mut(self)
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        T::into_partial_reflect(*self)
    }
}

impl CastPartialReflect for Box<dyn PartialReflect> {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self.as_ref()
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.as_mut()
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        *self
    }
}

impl CastPartialReflect for Box<dyn Reflect> {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self.as_ref().as_partial_reflect()
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.as_mut().as_partial_reflect_mut()
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self.into_reflect().into_partial_reflect()
    }
}

pub trait CastReflect: CastPartialReflect {
    fn as_reflect(&self) -> &dyn Reflect;
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect>;
}

impl<T: CastReflect> CastReflect for Box<T> {
    fn as_reflect(&self) -> &dyn Reflect {
        T::as_reflect(self)
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        T::as_reflect_mut(self)
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        T::into_reflect(*self)
    }
}

impl CastReflect for Box<dyn Reflect> {
    fn as_reflect(&self) -> &dyn Reflect {
        self.as_ref()
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self.as_mut()
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        *self
    }
}

impl_type_path!(::alloc::boxed::Box<T: ?Sized>);

impl MaybeTyped for Box<dyn Reflect> {}
impl MaybeTyped for Box<dyn PartialReflect> {}

impl<T: TypePath + Send + Sync> Typed for Box<T> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl RegisterForReflection for Box<dyn Reflect> {}
impl RegisterForReflection for Box<dyn PartialReflect> {}

impl<T: TypePath + Send + Sync> GetTypeRegistration for Box<T> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Self>()
    }
}

macro_rules! impl_cast_partial_reflect {
    ($(<$($id:ident),* $(,)?>)? for $ty:ty $(where $($tt:tt)*)?) => {
        impl $(<$($id),*>)? $crate::cast::CastPartialReflect for $ty $(where $($tt)*)? {
            fn as_partial_reflect(&self) -> &dyn $crate::PartialReflect {
                self
            }

            fn as_partial_reflect_mut(&mut self) -> &mut dyn $crate::PartialReflect {
                self
            }

            fn into_partial_reflect(self: Box<Self>) -> Box<dyn $crate::PartialReflect> {
                self
            }
        }
    };
}

pub(crate) use impl_cast_partial_reflect;

macro_rules! impl_casting_traits {
    ($(<$($id:ident),* $(,)?>)? for $ty:ty $(where $($tt:tt)*)?) => {

        $crate::cast::impl_cast_partial_reflect!($(<$($id),*>)? for $ty $(where $($tt)*)?);

        impl $(<$($id),*>)? $crate::cast::CastReflect for $ty $(where $($tt)*)? {
            fn as_reflect(&self) -> &dyn $crate::Reflect {
                self
            }

            fn as_reflect_mut(&mut self) -> &mut dyn $crate::Reflect {
                self
            }

            fn into_reflect(self: Box<Self>) -> Box<dyn $crate::Reflect> {
                self
            }
        }
    };
}

pub(crate) use impl_casting_traits;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Struct, Tuple, TupleStruct};
    use static_assertions::assert_not_impl_all;

    #[test]
    fn should_not_reflect_box() {
        assert_not_impl_all!(Box<i32>: Reflect, PartialReflect);
        assert_not_impl_all!(Box<dyn PartialReflect>: Reflect, PartialReflect);
        assert_not_impl_all!(Box<dyn Reflect>: Reflect, PartialReflect);
    }

    #[test]
    fn should_reflect_boxed_struct_field() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct {
            value: Box<dyn Reflect>,
        }

        let my_struct: Box<dyn Struct> = Box::new(MyStruct {
            value: Box::new(123_i32),
        });

        let field = my_struct.field("value").unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&123));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }

    #[test]
    fn should_reflect_boxed_tuple_struct_field() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct MyStruct(Box<dyn Reflect>);

        let my_struct: Box<dyn TupleStruct> = Box::new(MyStruct(Box::new(123_i32)));

        let field = my_struct.field(0).unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&123));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }

    #[test]
    fn should_reflect_boxed_tuple_field() {
        let my_struct: Box<dyn Tuple> = Box::new((Box::new(10_i32),));

        let field = my_struct.field(0).unwrap();
        assert_eq!(field.try_downcast_ref::<i32>(), Some(&10));

        let field_info = field.get_represented_type_info().unwrap();
        assert!(field_info.ty().is::<i32>());
    }
}
