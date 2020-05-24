use super::Node;
use crate::{
    render::UI_PIPELINE_HANDLE, sprite::Sprite, widget::Label, ColorMaterial, Rect, QUAD_HANDLE,
};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};

#[derive(EntityArchetype)]
pub struct UiEntity {
    pub node: Node,
    pub rect: Rect,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
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

#[derive(EntityArchetype)]
pub struct LabelEntity {
    pub node: Node,
    pub rect: Rect,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub renderable: Renderable,
    pub label: Label,
}

impl Default for LabelEntity {
    fn default() -> Self {
        LabelEntity {
            node: Default::default(),
            rect: Default::default(),
            mesh: QUAD_HANDLE,
            // NOTE: labels each get their own material.
            material: Handle::new(), // TODO: maybe abstract this out
            renderable: Renderable {
                pipelines: vec![UI_PIPELINE_HANDLE],
                ..Default::default()
            },
            label: Label::default(),
        }
    }
}

#[derive(EntityArchetype)]
pub struct SpriteEntity {
    pub sprite: Sprite,
    pub rect: Rect,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub renderable: Renderable,
}

impl Default for SpriteEntity {
    fn default() -> Self {
        SpriteEntity {
            sprite: Default::default(),
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
