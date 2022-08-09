use bevy_ecs::prelude::Component;
use bevy_render::color::Color;
use bevy_render::render_resource::{LoadOp, Operations, RenderPassColorAttachment, TextureView};

#[derive(Clone, Debug)]
pub struct ColorAttachment {
    pub view: TextureView,
    pub sampled_target: Option<TextureView>,
}

impl ColorAttachment {
    pub fn load(&self, load_op: LoadOp<Color>) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: self.sampled_target.as_ref().unwrap_or(&self.view),
            resolve_target: if self.sampled_target.is_some() {
                Some(&self.view)
            } else {
                None
            },
            ops: Operations {
                load: match load_op {
                    LoadOp::Clear(v) => LoadOp::Clear(v.into()),
                    LoadOp::Load => LoadOp::Load,
                },
                store: true,
            },
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct ColorAttachments {
    pub attachments: Vec<ColorAttachment>,
}

impl ColorAttachments {
    pub fn load(&self, load_op: LoadOp<Color>) -> Vec<Option<RenderPassColorAttachment>> {
        self.attachments
            .iter()
            .map(|a| Some(a.load(load_op)))
            .collect()
    }
}
