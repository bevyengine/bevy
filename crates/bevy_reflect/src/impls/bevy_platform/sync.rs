use bevy_reflect_derive::{impl_reflect_opaque, impl_type_path};

impl_reflect_opaque!(::bevy_platform::sync::Arc<T: Send + Sync + ?Sized>(Clone));
impl_type_path!(::bevy_platform::sync::Mutex<T>);
