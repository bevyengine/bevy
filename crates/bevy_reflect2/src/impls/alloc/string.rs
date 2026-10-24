use crate::{self as bevy_reflect2, impl_reflect_primitive};

impl_reflect_primitive!(
    #[primitive(String)]
    ::alloc::string::String(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);

#[cfg(test)]
mod tests {
    use crate::Reflect;
    use alloc::string::String;

    #[test]
    fn string() {
        let a = &String::from("Hello") as &dyn Reflect;
        let b = &String::from("Hello") as &dyn Reflect;
        let c = &String::from("World") as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(b), Some(true));
        assert_eq!(a.reflect_partial_eq(c), Some(false));
        assert_eq!(a.ty().type_path().full(), "alloc::string::String");
        assert_eq!(alloc::format!("{a:?}"), "\"Hello\"");
    }
}
