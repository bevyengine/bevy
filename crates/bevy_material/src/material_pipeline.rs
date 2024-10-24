use bevy_app::Plugin;
use bevy_reflect::TypePath;
use core::hash::Hash;

use crate::material::Material;

pub trait MaterialPipeline: TypePath + Sized + 'static {
    type MaterialProperties: Send + Sync + 'static;
    type ShaderKey: Hash + Eq + Send + Sync + 'static;
    type PipelineInfo<'a, M: Material<Self>>;

    fn material_plugin<M: Material<Self>>() -> impl Plugin;
}
