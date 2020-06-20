use super::Node;
use crate::{render::UI_PIPELINE_HANDLE, widget::Label};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{draw::Draw, mesh::Mesh, pipeline::RenderPipelines};
use bevy_sprite::{ColorMaterial, Quad, QUAD_HANDLE};

#[derive(EntityArchetype)]
pub struct UiEntity {
    pub node: Node,
    pub quad: Quad,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
}

impl Default for UiEntity {
    fn default() -> Self {
        UiEntity {
            mesh: QUAD_HANDLE,
            render_pipelines: RenderPipelines::from_handles(&[UI_PIPELINE_HANDLE]),
            node: Default::default(),
            quad: Default::default(),
            material: Default::default(),
            draw: Default::default(),
        }
    }
}

#[derive(EntityArchetype)]
pub struct LabelEntity {
    pub node: Node,
    pub quad: Quad,
    pub draw: Draw,
    pub label: Label,
}

impl Default for LabelEntity {
    fn default() -> Self {
        LabelEntity {
            label: Label::default(),
            node: Default::default(),
            quad: Default::default(),
            draw: Default::default(),
        }
    }
}
