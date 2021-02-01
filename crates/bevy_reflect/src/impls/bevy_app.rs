use crate::{impl_reflect_value, GetTypeRegistration, ReflectDeserialize, TypeRegistryArc};
use bevy_app::{AppBuilder, Plugin};
use bevy_ecs::Entity;

#[derive(Default)]
pub struct ReflectPlugin;

impl Plugin for ReflectPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<TypeRegistryArc>()
            .register_type::<bool>()
            .register_type::<u8>()
            .register_type::<u16>()
            .register_type::<u32>()
            .register_type::<u64>()
            .register_type::<u128>()
            .register_type::<usize>()
            .register_type::<i8>()
            .register_type::<i16>()
            .register_type::<i32>()
            .register_type::<i64>()
            .register_type::<i128>()
            .register_type::<isize>()
            .register_type::<f32>()
            .register_type::<f64>()
            .register_type::<String>();
        #[cfg(feature = "glam")]
        {
            app.register_type::<glam::Vec2>()
                .register_type::<glam::Vec3>()
                .register_type::<glam::Vec4>()
                .register_type::<glam::Mat3>()
                .register_type::<glam::Mat4>()
                .register_type::<glam::Quat>();
        }
        #[cfg(feature = "bevy_ecs")]
        {
            app.register_type::<bevy_ecs::Entity>();
        }
    }
}

impl_reflect_value!(Entity(Hash, PartialEq, Serialize, Deserialize));

pub trait RegisterTypeBuilder {
    fn register_type<T: GetTypeRegistration>(&mut self) -> &mut Self;
}

impl RegisterTypeBuilder for AppBuilder {
    fn register_type<T: GetTypeRegistration>(&mut self) -> &mut Self {
        {
            let registry = self.resources().get_mut::<TypeRegistryArc>().unwrap();
            registry.write().register::<T>();
        }
        self
    }
}
