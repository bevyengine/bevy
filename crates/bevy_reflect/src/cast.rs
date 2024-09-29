use crate as bevy_reflect;
use crate::__macro_exports::RegisterForReflection;
use crate::{MaybeTyped, PartialReflect, Reflect};
use bevy_reflect_derive::impl_type_path;

pub trait CastPartialReflect {
    fn as_partial_reflect(&self) -> &dyn PartialReflect;
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect;
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect>;
}

impl<T: PartialReflect> CastPartialReflect for T {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }
}

impl CastPartialReflect for dyn PartialReflect {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }
}

impl CastPartialReflect for dyn Reflect {
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self.as_partial_reflect()
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.as_partial_reflect_mut()
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self.into_partial_reflect()
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

impl<T: Reflect> CastReflect for T {
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }
}

impl CastReflect for dyn Reflect {
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
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

impl RegisterForReflection for Box<dyn Reflect> {}
impl RegisterForReflection for Box<dyn PartialReflect> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::{Struct, TupleStruct};
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
}
