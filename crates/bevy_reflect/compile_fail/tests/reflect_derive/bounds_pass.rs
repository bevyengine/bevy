//@check-pass
use bevy_reflect::prelude::*;

#[derive(Default)]
struct NonReflect;

struct NonReflectNonDefault;

mod structs {
    use super::*;

    #[derive(Reflect)]
    struct ReflectGeneric<T> {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    struct FromReflectGeneric<T> {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    #[reflect(Default)]
    struct DefaultGeneric<T> {
        foo: Option<T>,
        #[reflect(ignore)]
        _ignored: NonReflectNonDefault,
    }

    impl<T> Default for DefaultGeneric<T> {
        fn default() -> Self {
            Self {
                foo: None,
                _ignored: NonReflectNonDefault,
            }
        }
    }

    #[derive(Reflect)]
    struct ReflectBoundGeneric<T: Clone> {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    struct FromReflectBoundGeneric<T: Clone> {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    #[reflect(Default)]
    struct DefaultBoundGeneric<T: Clone> {
        foo: Option<T>,
        #[reflect(ignore)]
        _ignored: NonReflectNonDefault,
    }

    impl<T: Clone> Default for DefaultBoundGeneric<T> {
        fn default() -> Self {
            Self {
                foo: None,
                _ignored: NonReflectNonDefault,
            }
        }
    }

    #[derive(Reflect)]
    struct ReflectGenericWithWhere<T>
    where
        T: Clone,
    {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    struct FromReflectGenericWithWhere<T>
    where
        T: Clone,
    {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    #[reflect(Default)]
    struct DefaultGenericWithWhere<T>
    where
        T: Clone,
    {
        foo: Option<T>,
        #[reflect(ignore)]
        _ignored: NonReflectNonDefault,
    }

    impl<T> Default for DefaultGenericWithWhere<T>
    where
        T: Clone,
    {
        fn default() -> Self {
            Self {
                foo: None,
                _ignored: NonReflectNonDefault,
            }
        }
    }

    #[derive(Reflect)]
    #[rustfmt::skip]
    struct ReflectGenericWithWhereNoTrailingComma<T>
        where
            T: Clone
    {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    #[rustfmt::skip]
    struct FromReflectGenericWithWhereNoTrailingComma<T>
        where
            T: Clone
    {
        foo: T,
        #[reflect(ignore)]
        _ignored: NonReflect,
    }

    #[derive(Reflect)]
    #[reflect(Default)]
    #[rustfmt::skip]
    struct DefaultGenericWithWhereNoTrailingComma<T>
        where
            T: Clone
    {
        foo: Option<T>,
        #[reflect(ignore)]
        _ignored: NonReflectNonDefault,
    }

    impl<T> Default for DefaultGenericWithWhereNoTrailingComma<T>
    where
        T: Clone,
    {
        fn default() -> Self {
            Self {
                foo: None,
                _ignored: NonReflectNonDefault,
            }
        }
    }
}

mod tuple_structs {
    use super::*;

    #[derive(Reflect)]
    struct ReflectGeneric<T>(T, #[reflect(ignore)] NonReflect);

    #[derive(Reflect)]
    struct FromReflectGeneric<T>(T, #[reflect(ignore)] NonReflect);

    #[derive(Reflect)]
    #[reflect(Default)]
    struct DefaultGeneric<T>(Option<T>, #[reflect(ignore)] NonReflectNonDefault);

    impl<T> Default for DefaultGeneric<T> {
        fn default() -> Self {
            Self(None, NonReflectNonDefault)
        }
    }

    #[derive(Reflect)]
    struct ReflectBoundGeneric<T: Clone>(T, #[reflect(ignore)] NonReflect);

    #[derive(Reflect)]
    struct FromReflectBoundGeneric<T: Clone>(T, #[reflect(ignore)] NonReflect);

    #[derive(Reflect)]
    #[reflect(Default)]
    struct DefaultBoundGeneric<T: Clone>(Option<T>, #[reflect(ignore)] NonReflectNonDefault);

    impl<T: Clone> Default for DefaultBoundGeneric<T> {
        fn default() -> Self {
            Self(None, NonReflectNonDefault)
        }
    }

    #[derive(Reflect)]
    struct ReflectGenericWithWhere<T>(T, #[reflect(ignore)] NonReflect)
    where
        T: Clone;

    #[derive(Reflect)]
    struct FromReflectGenericWithWhere<T>(T, #[reflect(ignore)] NonReflect)
    where
        T: Clone;

    #[derive(Reflect)]
    #[reflect(Default)]
    struct DefaultGenericWithWhere<T>(Option<T>, #[reflect(ignore)] NonReflectNonDefault)
    where
        T: Clone;

    impl<T> Default for DefaultGenericWithWhere<T>
    where
        T: Clone,
    {
        fn default() -> Self {
            Self(None, NonReflectNonDefault)
        }
    }
}

mod enums {
    use super::*;

    #[derive(Reflect)]
    enum ReflectGeneric<T> {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    enum FromReflectGeneric<T> {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    enum ReflectBoundGeneric<T: Clone> {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    enum FromReflectBoundGeneric<T: Clone> {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    enum ReflectGenericWithWhere<T>
    where
        T: Clone,
    {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    enum FromReflectGenericWithWhere<T>
    where
        T: Clone,
    {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    #[rustfmt::skip]
    enum ReflectGenericWithWhereNoTrailingComma<T>
        where
            T: Clone
    {
        Foo(T, #[reflect(ignore)] NonReflect),
    }

    #[derive(Reflect)]
    #[rustfmt::skip]
    enum FromReflectGenericWithWhereNoTrailingComma<T>
        where
            T: Clone
    {
        Foo(T, #[reflect(ignore)] NonReflect),
    }
}
