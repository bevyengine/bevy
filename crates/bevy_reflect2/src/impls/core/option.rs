use crate::{self as bevy_reflect2, impl_reflect};

impl_reflect! {
    #[reflect(Default)]
    #[type_path = "core::option"]
    enum Option<T> {
        None,
        Some(T),
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_reflect2, FieldKind, Kind, Reflect, Typed};
    use alloc::{format, string::String};

    #[test]
    fn option_type_info() {
        assert_eq!(
            Option::<i32>::TYPE.type_path().full(),
            "core::option::Option<i32>"
        );
        let Kind::Enum(enum_type) = Option::<i32>::TYPE.kind() else {
            panic!("expected enum kind");
        };
        let variants = enum_type.variants();
        assert_eq!(variants[0].name, "None");
        assert_eq!(variants[1].name, "Some");
        assert_eq!(variants[1].field_kind, FieldKind::Unnamed);
        assert_eq!(
            variants[1].fields[0].ty.get().type_id(),
            i32::TYPE.type_id()
        );
    }

    #[test]
    fn option_access() {
        let mut value = Some(123usize);
        let reflect = &mut value as &mut dyn Reflect;
        assert_eq!(reflect.enum_ref().unwrap().variant_type().name, "Some");
        *reflect
            .enum_mut()
            .unwrap()
            .field_mut(0)
            .unwrap()
            .downcast_mut::<usize>()
            .unwrap() = 321;
        assert_eq!(value, Some(321));

        let none = None::<usize>;
        let reflect = &none as &dyn Reflect;
        assert_eq!(reflect.enum_ref().unwrap().variant(), 0);
        assert!(reflect.enum_ref().unwrap().field(0).is_none());
    }

    #[test]
    fn option_builtins() {
        let a = &Some(123) as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(&Some(123)), Some(true));
        assert_eq!(a.reflect_partial_eq(&Some(321)), Some(false));
        assert_eq!(a.reflect_partial_eq(&None::<i32>), Some(false));
        assert_eq!(
            a.reflect_partial_cmp(&None::<i32>),
            Some(core::cmp::Ordering::Greater)
        );
        assert_eq!(
            a.reflect_hash(),
            (&Some(123) as &dyn Reflect).reflect_hash()
        );
        assert_ne!(
            a.reflect_hash(),
            (&Some(321) as &dyn Reflect).reflect_hash()
        );
        assert_eq!(format!("{a:?}"), "Some(123)");
        assert_eq!(format!("{:?}", &None::<i32> as &dyn Reflect), "None");

        // Structural clone, even though `NotClone` does not implement `Clone`
        #[derive(crate::Reflect, PartialEq, Debug)]
        struct NotClone(String);
        let value = Some(NotClone(String::from("hello")));
        let clone = (&value as &dyn Reflect).reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Option<NotClone>>(), Some(&value));

        let default = Option::<i32>::TYPE.default().unwrap();
        assert_eq!(default.downcast_ref::<Option<i32>>(), Some(&None));
    }
}
