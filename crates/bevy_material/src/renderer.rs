use bevy_app::App;
use bevy_reflect::TypePath;

use crate::material::Material;

pub enum QueueError {}

pub trait MaterialPipeline: TypePath + Sized + 'static {
    type MaterialProperties: Send + Sync + 'static;
    type ShaderKey;
    type PipelineInfo<'a, M: Material<Self>>;

    fn material_plugin<M: Material<Self>>(app: &mut App);
}
