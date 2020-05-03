use super::Node;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};
use crate::{Rect, render::UI_PIPELINE_HANDLE, QUAD_HANDLE};
use bevy_asset::Handle;

#[derive(EntityArchetype)]
#[module(meta = false)]
pub struct UiEntity {
    pub node: Node,
    pub rect: Rect,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    // pub renderable: Renderable,
}

impl Default for UiEntity {
    fn default() -> Self {
        UiEntity {
            node: Default::default(),
            rect: Default::default(),
            mesh: QUAD_HANDLE,
            // renderable: Renderable {
            //     pipelines: vec![
            //         UI_PIPELINE_HANDLE
            //     ],
            //     ..Default::default()
            // }
        }
    }
}