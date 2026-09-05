use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_camera::CompositingSpace;
use bevy_ecs::prelude::*;
use bevy_mesh::VertexBufferLayout;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    view::{ResolvedCompositingSpace, ViewUniform},
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

/// How a UI fragment encodes its output for the view it renders into,
/// shared by every UI pipeline key.
///
/// UI draws into the view's main texture after tonemapping, so each
/// fragment encodes to match that texture's compositing space.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct UiWriterEncodeKey {
    /// The view's resolved [`CompositingSpace`]. Linear views leave this
    /// `None`, since linear output needs no encode.
    pub compositing_space: Option<CompositingSpace>,
}

impl UiWriterEncodeKey {
    /// Builds the key from a view's [`ResolvedCompositingSpace`].
    pub fn from_resolved_space(resolved: Option<&ResolvedCompositingSpace>) -> Self {
        Self {
            compositing_space: ResolvedCompositingSpace::space(resolved)
                .filter(|space| !space.is_linear()),
        }
    }

    /// Appends the shader defs for the compositing space.
    pub fn push_shader_defs(&self, shader_defs: &mut Vec<ShaderDefVal>) {
        match self.compositing_space {
            Some(CompositingSpace::Srgb) => shader_defs.push("COMPOSITING_SPACE_SRGB".into()),
            Some(CompositingSpace::Oklab) => shader_defs.push("COMPOSITING_SPACE_OKLAB".into()),
            Some(CompositingSpace::Linear) | None => {}
        }
    }
}

#[derive(Resource)]
pub struct UiPipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub image_layout: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
}

pub fn init_ui_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let view_layout = BindGroupLayoutDescriptor::new(
        "ui_view_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<ViewUniform>(true),
        ),
    );

    let image_layout = BindGroupLayoutDescriptor::new(
        "ui_image_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    commands.insert_resource(UiPipeline {
        view_layout,
        image_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "ui.wesl"),
    });
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct UiPipelineKey {
    pub target_format: TextureFormat,
    pub anti_alias: bool,
    pub writer_encode: UiWriterEncodeKey,
}

impl SpecializedRenderPipeline for UiPipeline {
    type Key = UiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // color
                VertexFormat::Float32x4,
                // mode
                VertexFormat::Uint32,
                // border radius x values (top left, top right, bottom right, bottom left)
                VertexFormat::Float32x4,
                // border radius y values (top left, top right, bottom right, bottom left)
                VertexFormat::Float32x4,
                // border thickness
                VertexFormat::Float32x4,
                // border size
                VertexFormat::Float32x2,
                // position relative to the center
                VertexFormat::Float32x2,
            ],
        );
        let mut shader_defs = if key.anti_alias {
            vec!["ANTI_ALIAS".into()]
        } else {
            Vec::new()
        };
        key.writer_encode.push_shader_defs(&mut shader_defs);

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            layout: vec![self.view_layout.clone(), self.image_layout.clone()],
            label: Some("ui_pipeline".into()),
            ..default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs_for(key: UiWriterEncodeKey) -> Vec<ShaderDefVal> {
        let mut defs = Vec::new();
        key.push_shader_defs(&mut defs);
        defs
    }

    #[test]
    fn no_writer_encode_for_default_or_linear() {
        assert!(defs_for(UiWriterEncodeKey::default()).is_empty());

        let linear = ResolvedCompositingSpace(Some(CompositingSpace::Linear));
        let key = UiWriterEncodeKey::from_resolved_space(Some(&linear));
        assert_eq!(key, UiWriterEncodeKey::default());
        assert!(defs_for(key).is_empty());
    }

    #[test]
    fn compositing_space_appends_exactly_its_def() {
        let srgb = ResolvedCompositingSpace(Some(CompositingSpace::Srgb));
        assert_eq!(
            defs_for(UiWriterEncodeKey::from_resolved_space(Some(&srgb))),
            vec![ShaderDefVal::from("COMPOSITING_SPACE_SRGB")]
        );
        let oklab = ResolvedCompositingSpace(Some(CompositingSpace::Oklab));
        assert_eq!(
            defs_for(UiWriterEncodeKey::from_resolved_space(Some(&oklab))),
            vec![ShaderDefVal::from("COMPOSITING_SPACE_OKLAB")]
        );
    }
}
