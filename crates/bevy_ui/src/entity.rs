use super::Node;
use crate::{render::UI_PIPELINE_HANDLE, ColorMaterial, Rect, QUAD_HANDLE};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};

#[derive(EntityArchetype)]
#[module(meta = false)]
pub struct UiEntity {
    pub node: Node,
    pub rect: Rect,
    pub mesh: Handle<Mesh>,              // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub renderable: Renderable,
}

impl Default for UiEntity {
    fn default() -> Self {
        UiEntity {
            node: Default::default(),
            rect: Default::default(),
            mesh: QUAD_HANDLE,
            material: Default::default(),
            renderable: Renderable {
                pipelines: vec![UI_PIPELINE_HANDLE],
                ..Default::default()
            },
        }
    }
}
