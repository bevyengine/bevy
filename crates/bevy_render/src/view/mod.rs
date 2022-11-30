pub mod visibility;
pub mod window;

pub use visibility::*;
pub use window::*;

use crate::{
    camera::ExtractedCamera,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    prelude::Image,
    rangefinder::ViewRangefinder3d,
    render_asset::RenderAssets,
    render_resource::{DynamicUniformBuffer, ShaderType, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, TextureCache},
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, UVec4, Vec3, Vec4};
use bevy_reflect::Reflect;
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use wgpu::{
    Color, Extent3d, Operations, RenderPassColorAttachment, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ComputedVisibility>()
            .register_type::<ComputedVisibilityFlags>()
            .register_type::<Msaa>()
            .register_type::<RenderLayers>()
            .register_type::<Visibility>()
            .register_type::<VisibleEntities>()
            .init_resource::<Msaa>()
            // NOTE: windows.is_changed() handles cases where a window was resized
            .add_plugin(ExtractResourcePlugin::<Msaa>::default())
            .add_plugin(VisibilityPlugin);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ViewUniforms>()
                .add_system_to_stage(RenderStage::Prepare, prepare_view_uniforms)
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_view_targets.after(WindowSystem::Prepare),
                );
        }
    }
}

/// Configuration resource for [Multi-Sample Anti-Aliasing](https://en.wikipedia.org/wiki/Multisample_anti-aliasing).
///
/// # Example
/// ```
/// # use bevy_app::prelude::App;
/// # use bevy_render::prelude::Msaa;
/// App::new()
///     .insert_resource(Msaa { samples: 4 })
///     .run();
/// ```
#[derive(Resource, Clone, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct Msaa {
    /// The number of samples to run for Multi-Sample Anti-Aliasing. Higher numbers result in
    /// smoother edges.
    /// Defaults to 4.
    ///
    /// Note that WGPU currently only supports 1 or 4 samples.
    /// Ultimately we plan on supporting whatever is natively supported on a given device.
    /// Check out this issue for more info: <https://github.com/gfx-rs/wgpu/issues/1832>
    pub samples: u32,
}

impl Default for Msaa {
    fn default() -> Self {
        Self { samples: 4 }
    }
}

#[derive(Component)]
pub struct ExtractedView {
    pub projection: Mat4,
    pub transform: GlobalTransform,
    pub hdr: bool,
    // uvec4(origin.x, origin.y, width, height)
    pub viewport: UVec4,
}

impl ExtractedView {
    /// Creates a 3D rangefinder for a view
    pub fn rangefinder3d(&self) -> ViewRangefinder3d {
        ViewRangefinder3d::from_view_matrix(&self.transform.compute_matrix())
    }
}

#[derive(Clone, ShaderType)]
pub struct ViewUniform {
    view_proj: Mat4,
    inverse_view_proj: Mat4,
    view: Mat4,
    inverse_view: Mat4,
    projection: Mat4,
    inverse_projection: Mat4,
    world_position: Vec3,
    // viewport(x_origin, y_origin, width, height)
    viewport: Vec4,
}

#[derive(Resource, Default)]
pub struct ViewUniforms {
    pub uniforms: DynamicUniformBuffer<ViewUniform>,
}

#[derive(Component)]
pub struct ViewUniformOffset {
    pub offset: u32,
}

#[derive(Component)]
pub struct ViewTarget {
    main_textures: MainTargetTextures,
    main_texture_format: TextureFormat,
    /// 0 represents `main_textures.a`, 1 represents `main_textures.b`
    main_texture: AtomicUsize,
    out_texture: TextureView,
    out_texture_format: TextureFormat,
}

pub struct PostProcessWrite<'a> {
    pub source: &'a TextureView,
    pub destination: &'a TextureView,
}

impl ViewTarget {
    pub const TEXTURE_FORMAT_HDR: TextureFormat = TextureFormat::Rgba16Float;

    /// Retrieve this target's color attachment. This will use [`Self::sampled_main_texture`] and resolve to [`Self::main_texture`] if
    /// the target has sampling enabled. Otherwise it will use [`Self::main_texture`] directly.
    pub fn get_color_attachment(&self, ops: Operations<Color>) -> RenderPassColorAttachment {
        match &self.main_textures.sampled {
            Some(sampled_texture) => RenderPassColorAttachment {
                view: sampled_texture,
                resolve_target: Some(self.main_texture()),
                ops,
            },
            None => self.get_unsampled_color_attachment(ops),
        }
    }

    /// Retrieve an "unsampled" color attachment using [`Self::main_texture`].
    pub fn get_unsampled_color_attachment(
        &self,
        ops: Operations<Color>,
    ) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: self.main_texture(),
            resolve_target: None,
            ops,
        }
    }

    /// The "main" unsampled texture.
    pub fn main_texture(&self) -> &TextureView {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.a
        } else {
            &self.main_textures.b
        }
    }

    /// The "main" sampled texture.
    pub fn sampled_main_texture(&self) -> Option<&TextureView> {
        self.main_textures.sampled.as_ref()
    }

    #[inline]
    pub fn main_texture_format(&self) -> TextureFormat {
        self.main_texture_format
    }

    /// Returns `true` if and only if the main texture is [`Self::TEXTURE_FORMAT_HDR`]
    #[inline]
    pub fn is_hdr(&self) -> bool {
        self.main_texture_format == ViewTarget::TEXTURE_FORMAT_HDR
    }

    /// The final texture this view will render to.
    #[inline]
    pub fn out_texture(&self) -> &TextureView {
        &self.out_texture
    }

    /// The format of the final texture this view will render to
    #[inline]
    pub fn out_texture_format(&self) -> TextureFormat {
        self.out_texture_format
    }

    /// This will start a new "post process write", which assumes that the caller
    /// will write the [`PostProcessWrite`]'s `source` to the `destination`.
    ///
    /// `source` is the "current" main texture. This will internally flip this
    /// [`ViewTarget`]'s main texture to the `destination` texture, so the caller
    /// _must_ ensure `source` is copied to `destination`, with or without modifications.
    /// Failing to do so will cause the current main texture information to be lost.
    pub fn post_process_write(&self) -> PostProcessWrite {
        let old_is_a_main_texture = self.main_texture.fetch_xor(1, Ordering::SeqCst);
        // if the old main texture is a, then the post processing must write from a to b
        if old_is_a_main_texture == 0 {
            PostProcessWrite {
                source: &self.main_textures.a,
                destination: &self.main_textures.b,
            }
        } else {
            PostProcessWrite {
                source: &self.main_textures.b,
                destination: &self.main_textures.a,
            }
        }
    }
}

#[derive(Component)]
pub struct ViewDepthTexture {
    pub texture: Texture,
    pub view: TextureView,
}

fn prepare_view_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<ViewUniforms>,
    views: Query<(Entity, &ExtractedView)>,
) {
    view_uniforms.uniforms.clear();
    for (entity, camera) in &views {
        let projection = camera.projection;
        let inverse_projection = projection.inverse();
        let view = camera.transform.compute_matrix();
        let inverse_view = view.inverse();
        let view_uniforms = ViewUniformOffset {
            offset: view_uniforms.uniforms.push(ViewUniform {
                view_proj: projection * inverse_view,
                inverse_view_proj: view * inverse_projection,
                view,
                inverse_view,
                projection,
                inverse_projection,
                world_position: camera.transform.translation(),
                viewport: camera.viewport.as_vec4(),
            }),
        };

        commands.entity(entity).insert(view_uniforms);
    }

    view_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

#[derive(Clone)]
struct MainTargetTextures {
    a: TextureView,
    b: TextureView,
    sampled: Option<TextureView>,
}

#[allow(clippy::too_many_arguments)]
fn prepare_view_targets(
    mut commands: Commands,
    windows: Res<ExtractedWindows>,
    images: Res<RenderAssets<Image>>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    cameras: Query<(Entity, &ExtractedCamera, &ExtractedView)>,
) {
    let mut textures = HashMap::default();
    for (entity, camera, view) in cameras.iter() {
        if let Some(target_size) = camera.physical_target_size {
            if let (Some(out_texture_view), Some(out_texture_format)) = (
                camera.target.get_texture_view(&windows, &images),
                camera.target.get_texture_format(&windows, &images),
            ) {
                let size = Extent3d {
                    width: target_size.x,
                    height: target_size.y,
                    depth_or_array_layers: 1,
                };

                let main_texture_format = if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                };

                let main_textures = textures
                    .entry((camera.target.clone(), view.hdr))
                    .or_insert_with(|| {
                        let descriptor = TextureDescriptor {
                            label: None,
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: main_texture_format,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                        };
                        MainTargetTextures {
                            a: texture_cache
                                .get(
                                    &render_device,
                                    TextureDescriptor {
                                        label: Some("main_texture_a"),
                                        ..descriptor
                                    },
                                )
                                .default_view,
                            b: texture_cache
                                .get(
                                    &render_device,
                                    TextureDescriptor {
                                        label: Some("main_texture_b"),
                                        ..descriptor
                                    },
                                )
                                .default_view,
                            sampled: (msaa.samples > 1).then(|| {
                                texture_cache
                                    .get(
                                        &render_device,
                                        TextureDescriptor {
                                            label: Some("main_texture_sampled"),
                                            size,
                                            mip_level_count: 1,
                                            sample_count: msaa.samples,
                                            dimension: TextureDimension::D2,
                                            format: main_texture_format,
                                            usage: TextureUsages::RENDER_ATTACHMENT,
                                        },
                                    )
                                    .default_view
                            }),
                        }
                    });

                commands.entity(entity).insert(ViewTarget {
                    main_textures: main_textures.clone(),
                    main_texture_format,
                    main_texture: AtomicUsize::new(0),
                    out_texture: out_texture_view.clone(),
                    out_texture_format,
                });
            }
        }
    }
}
