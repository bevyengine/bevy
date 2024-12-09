use bevy_app::Plugin;
use bevy_ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy_reflect::TypePath;
use core::hash::Hash;

use crate::{material::Material, shaders::Shaders};

pub trait MaterialPipeline: TypePath + Sized + 'static {
    type MaterialProperties: Send + Sync;
    type ShaderKey: Hash + Eq + Send + Sync;
    type PipelineContext<'a, M: Material<Self>>;

    fn default_shaders() -> Shaders<Self>;

    fn material_plugin<M: Material<Self>>() -> impl Plugin;
}
