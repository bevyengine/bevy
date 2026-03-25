use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::bevy_platform::sync::Arc<T: Send + Sync + ?Sized>(Clone));
