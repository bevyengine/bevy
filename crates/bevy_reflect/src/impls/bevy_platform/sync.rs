use bevy_reflect_derive::{impl_reflect_opaque, impl_type_path};

impl_reflect_opaque!(::bevy_platform::sync::Arc<T: Send + Sync + ?Sized>(Clone));
impl_type_path!(::bevy_platform::sync::Mutex<T>);

#[cfg(test)]
mod tests {
    use crate::Typed;

    #[test]
    fn should_capture_generic_info() {
        let generics = <::bevy_platform::sync::Arc<u32>>::type_info()
            .as_opaque()
            .unwrap()
            .generics();

        assert_eq!(generics.len(), 1);

        let t = generics.get_named("T").unwrap();
        assert_eq!(t.name(), "T");
        assert!(t.is::<u32>());
        assert!(!t.is_const());
    }
}
