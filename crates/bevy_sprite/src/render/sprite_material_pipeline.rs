use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetApp, AssetEvent, AssetId, AssetServer, Assets, Handle};
use bevy_core_pipeline::{
    core_2d::Transparent2d,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    query::ROQueryItem,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Local, Query, Res, ResMut, Resource, SystemParamItem, SystemState,
    },
    world::{FromWorld, World},
};
use bevy_math::{Affine3A, Quat, Rect, Vec2, Vec4};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        AsBindGroupError, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        BlendState, BufferUsages, BufferVec, ColorTargetState, ColorWrites, FragmentState,
        FrontFace, ImageDataLayout, IndexFormat, MultisampleState, OwnedBindingResource,
        PipelineCache, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor,
        SamplerBindingType, Shader, ShaderRef, ShaderStages, SpecializedRenderPipeline,
        SpecializedRenderPipelines, TextureFormat, TextureSampleType, TextureViewDescriptor,
        VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{
        BevyDefault, DefaultImageSampler, FallbackImage, GpuImage, Image, ImageSampler,
        TextureFormatPixelInfo,
    },
    view::{
        ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
        ViewVisibility, VisibleEntities,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd, HashMap, HashSet};
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

use crate::{
    sprite_material::{SpriteMaterial, SpriteMaterialKey},
    ImageBindGroups, SpriteAssetEvents, SpritePipelineKey, SpriteSystem, SpriteWithMaterial,
    TextureAtlas, TextureAtlasSpriteWithMaterial,
};

pub const SPRITE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(270703992682389545502295514338037592175);

const SPRITE_VERTEX_OUTPUT_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(246203203511184449262050107766870630166);

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given
/// [`UiMaterial`] asset type (which includes [`UiMaterial`] types).
pub struct SpriteMaterialPlugin<M: SpriteMaterial>(PhantomData<M>);

impl<M: SpriteMaterial> Default for SpriteMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: SpriteMaterial> Plugin for SpriteMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SPRITE_VERTEX_OUTPUT_SHADER_HANDLE,
            "sprite_vertex_output.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SPRITE_MATERIAL_SHADER_HANDLE,
            "sprite_material.wgsl",
            Shader::from_wgsl
        );
        app.init_asset::<M>()
            .add_plugins(ExtractComponentPlugin::<Handle<M>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent2d, DrawSprite<M>>()
                .init_resource::<ExtractedSpriteMaterials<M>>()
                .init_resource::<ExtractedSpriteWithMaterials<M>>()
                .init_resource::<RenderMaterials<M>>()
                .init_resource::<SpriteMeta<M>>()
                .init_resource::<SpecializedRenderPipelines<SpriteMaterialPipeline<M>>>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_sprite_materials::<M>,
                        extract_sprite_with_materials::<M>.in_set(SpriteSystem::ExtractSprites),
                    ),
                )
                .add_systems(
                    Render,
                    (
                        prepare_sprite_materials::<M>.in_set(RenderSet::PrepareAssets),
                        queue_sprites::<M>.in_set(RenderSet::Queue),
                        prepare_sprites::<M>.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<SpriteMaterialPipeline<M>>();
        }
    }
}

#[derive(Resource)]
pub struct SpriteMaterialPipeline<M: SpriteMaterial> {
    pub view_layout: BindGroupLayout,
    pub texture_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    pub dummy_white_gpu_image: GpuImage,
    marker: PhantomData<M>,
}

impl<M: SpriteMaterial> FromWorld for SpriteMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<AssetServer>,
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (asset_server, render_device, default_sampler, render_queue) =
            system_state.get_mut(world);

        let view_layout = render_device.create_bind_group_layout(
            "sprite_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        let texture_layout = render_device.create_bind_group_layout(
            "sprite_texture_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let material_layout = M::bind_group_layout(&render_device);

        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.size_f32(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpriteMaterialPipeline {
            view_layout,
            texture_layout,
            material_layout,
            dummy_white_gpu_image,
            vertex_shader: match M::vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: match M::fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            marker: PhantomData,
        }
    }
}

impl<M: SpriteMaterial> SpecializedRenderPipeline for SpriteMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = SpriteMaterialKey<M>;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key
            .pipeline_key
            .contains(SpritePipelineKey::TONEMAP_IN_SHADER)
        {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key
                .pipeline_key
                .intersection(SpritePipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == SpritePipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
            {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.pipeline_key.contains(SpritePipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let format = match key.pipeline_key.contains(SpritePipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let instance_rate_vertex_buffer_layout = VertexBufferLayout {
            array_stride: 64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // @location(0) i_model_transpose_col0: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1) i_model_transpose_col1: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                // @location(2) i_model_transpose_col2: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
                // @location(3) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 3,
                },
            ],
        };
        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_MATERIAL_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_MATERIAL_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.pipeline_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("sprite_material_pipeline".into()),
            push_constant_ranges: Vec::new(),
        };

        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout = vec![
            self.view_layout.clone(),
            self.texture_layout.clone(),
            self.material_layout.clone(),
        ];

        M::specialize(&mut descriptor, key);

        descriptor
    }
}

pub struct ExtractedSpriteWithMaterial<M: SpriteMaterial> {
    pub transform: GlobalTransform,
    pub material: AssetId<M>,
    /// Select an area of the texture
    pub rect: Option<Rect>,
    /// Change the on-screen size of the sprite
    pub custom_size: Option<Vec2>,
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSprite` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub anchor: Vec2,
    /// For cases where additional ExtractedSprites are created during extraction, this stores the
    /// entity that caused that creation for use in determining visibility.
    pub original_entity: Option<Entity>,
}

#[derive(Resource)]
pub struct ExtractedSpriteWithMaterials<M: SpriteMaterial> {
    pub sprites: EntityHashMap<Entity, ExtractedSpriteWithMaterial<M>>,
}

impl<M: SpriteMaterial> Default for ExtractedSpriteWithMaterials<M> {
    fn default() -> Self {
        Self {
            sprites: Default::default(),
        }
    }
}

pub fn extract_sprite_with_materials<M: SpriteMaterial>(
    mut extracted_sprite_with_materials: ResMut<ExtractedSpriteWithMaterials<M>>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    sprite_with_material_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &SpriteWithMaterial<M>,
            &GlobalTransform,
            &Handle<Image>,
        )>,
    >,
    atlas_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &TextureAtlasSpriteWithMaterial<M>,
            &GlobalTransform,
            &Handle<TextureAtlas>,
        )>,
    >,
) {
    extracted_sprite_with_materials.sprites.clear();

    for (entity, view_visibility, sprite, transform, handle) in sprite_with_material_query.iter() {
        if !view_visibility.get() {
            continue;
        }
        // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
        extracted_sprite_with_materials.sprites.insert(
            entity,
            ExtractedSpriteWithMaterial {
                material: sprite.material.id(),
                transform: *transform,
                rect: sprite.rect,
                // Pass the custom size
                custom_size: sprite.custom_size,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: handle.id(),
                anchor: sprite.anchor.as_vec(),
                original_entity: None,
            },
        );
    }
    for (entity, view_visibility, atlas_sprite, transform, texture_atlas_handle) in
        atlas_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }
        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            let rect = Some(
                *texture_atlas
                    .textures
                    .get(atlas_sprite.index)
                    .unwrap_or_else(|| {
                        panic!(
                            "Sprite index {:?} does not exist for texture atlas handle {:?}.",
                            atlas_sprite.index,
                            texture_atlas_handle.id(),
                        )
                    }),
            );
            extracted_sprite_with_materials.sprites.insert(
                entity,
                ExtractedSpriteWithMaterial {
                    material: atlas_sprite.material.id(),
                    transform: *transform,
                    // Select the area in the texture atlas
                    rect,
                    // Pass the custom size
                    custom_size: atlas_sprite.custom_size,
                    flip_x: atlas_sprite.flip_x,
                    flip_y: atlas_sprite.flip_y,
                    image_handle_id: texture_atlas.texture.id(),
                    anchor: atlas_sprite.anchor.as_vec(),
                    original_entity: None,
                },
            );
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteInstance {
    // Affine 4x3 transposed to 3x4
    pub i_model_transpose: [Vec4; 3],
    pub i_uv_offset_scale: [f32; 4],
}

impl SpriteInstance {
    #[inline]
    fn from(transform: &Affine3A, uv_offset_scale: &Vec4) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();
        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            i_uv_offset_scale: uv_offset_scale.to_array(),
        }
    }
}

#[derive(Resource)]
pub struct SpriteMeta<M: SpriteMaterial> {
    view_bind_group: Option<BindGroup>,
    sprite_index_buffer: BufferVec<u32>,
    sprite_instance_buffer: BufferVec<SpriteInstance>,
    marker: PhantomData<M>,
}

impl<M: SpriteMaterial> Default for SpriteMeta<M> {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            sprite_index_buffer: BufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: BufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
            marker: PhantomData,
        }
    }
}

#[derive(Component, PartialEq, Eq, Clone)]
pub struct SpriteBatch<M: SpriteMaterial> {
    image_handle_id: AssetId<Image>,
    material_handle_id: AssetId<M>,
    range: Range<u32>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_sprites<M: SpriteMaterial>(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    sprite_pipeline: Res<SpriteMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpriteMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    extracted_sprites: Res<ExtractedSpriteWithMaterials<M>>,
    mut views: Query<(
        &mut RenderPhase<Transparent2d>,
        &VisibleEntities,
        &ExtractedView,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
    render_materials: Res<RenderMaterials<M>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples());

    let draw_sprite_function = draw_functions.read().id::<DrawSprite<M>>();

    for (mut transparent_phase, visible_entities, view, tonemapping, dither) in &mut views {
        let mut view_key = SpritePipelineKey::from_hdr(view.hdr) | msaa_key;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= SpritePipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => SpritePipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => SpritePipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => SpritePipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= SpritePipelineKey::DEBAND_DITHER;
            }
        }

        view_entities.clear();
        view_entities.extend(visible_entities.entities.iter().map(|e| e.index() as usize));

        transparent_phase
            .items
            .reserve(extracted_sprites.sprites.len());

        for (entity, extracted_sprite) in extracted_sprites.sprites.iter() {
            let Some(material) = render_materials.get(&extracted_sprite.material) else {
                continue;
            };

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &sprite_pipeline,
                SpriteMaterialKey {
                    pipeline_key: view_key,
                    bind_group_data: material.key.clone(),
                },
            );

            let index = extracted_sprite.original_entity.unwrap_or(*entity).index();

            if !view_entities.contains(index as usize) {
                continue;
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            // Add the item to the render phase
            transparent_phase.add(Transparent2d {
                draw_function: draw_sprite_function,
                pipeline,
                entity: *entity,
                sort_key,
                // batch_range and dynamic_offset will be calculated in prepare_sprites
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_sprites<M: SpriteMaterial>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta<M>>,
    view_uniforms: Res<ViewUniforms>,
    sprite_material_pipeline: Res<SpriteMaterialPipeline<M>>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    extracted_sprite_with_materials: Res<ExtractedSpriteWithMaterials<M>>,
    mut phases: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<SpriteAssetEvents>,
    render_materials: Res<RenderMaterials<M>>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added {..} |
            // images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, SpriteBatch<M>)> = Vec::with_capacity(*previous_len);

        // Clear the sprite instances
        sprite_meta.sprite_instance_buffer.clear();

        sprite_meta.view_bind_group = Some(render_device.create_bind_group(
            "sprite_view_bind_group",
            &sprite_material_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Index buffer indices
        let mut index = 0;

        let image_bind_groups = &mut *image_bind_groups;

        for mut transparent_phase in &mut phases {
            let mut batch_item_index = 0;
            let mut batch_image_size = Vec2::ZERO;
            let mut batch_image_handle = AssetId::invalid();
            let mut batch_material_handle = AssetId::invalid();
            // Iterate through the phase items and detect when successive sprites that can be batched.
            // Spawn an entity with a `SpriteBatch` component for each possible batch.
            // Compatible items share the same entity.
            for item_index in 0..transparent_phase.items.len() {
                let item = &transparent_phase.items[item_index];
                let Some(extracted_sprite) =
                    extracted_sprite_with_materials.sprites.get(&item.entity)
                else {
                    // If there is a phase item that is not a sprite, then we must start a new
                    // batch to draw the other phase item(s) and to respect draw order. This can be
                    // done by invalidating the batch_image_handle
                    batch_image_handle = AssetId::invalid();
                    batch_material_handle = AssetId::invalid();
                    continue;
                };

                let batch_image_changed = batch_image_handle != extracted_sprite.image_handle_id;
                if batch_image_changed {
                    let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                        continue;
                    };

                    batch_image_size = Vec2::new(gpu_image.size.x, gpu_image.size.y);
                    batch_image_handle = extracted_sprite.image_handle_id;
                    image_bind_groups
                        .values
                        .entry(batch_image_handle)
                        .or_insert_with(|| {
                            render_device.create_bind_group(
                                "sprite_texture_bind_group",
                                &sprite_material_pipeline.texture_layout,
                                &BindGroupEntries::sequential((
                                    &gpu_image.texture_view,
                                    &gpu_image.sampler,
                                )),
                            )
                        });
                }

                let batch_material_changed = batch_material_handle != extracted_sprite.material;
                if batch_material_changed {
                    if render_materials.get(&extracted_sprite.material).is_none() {
                        continue;
                    };

                    batch_material_handle = extracted_sprite.material;
                }

                // By default, the size of the quad is the size of the texture
                let mut quad_size = batch_image_size;

                // Calculate vertex data for this item
                let mut uv_offset_scale: Vec4;

                // If a rect is specified, adjust UVs and the size of the quad
                if let Some(rect) = extracted_sprite.rect {
                    let rect_size = rect.size();
                    uv_offset_scale = Vec4::new(
                        rect.min.x / batch_image_size.x,
                        rect.max.y / batch_image_size.y,
                        rect_size.x / batch_image_size.x,
                        -rect_size.y / batch_image_size.y,
                    );
                    quad_size = rect_size;
                } else {
                    uv_offset_scale = Vec4::new(0.0, 1.0, 1.0, -1.0);
                }

                if extracted_sprite.flip_x {
                    uv_offset_scale.x += uv_offset_scale.z;
                    uv_offset_scale.z *= -1.0;
                }
                if extracted_sprite.flip_y {
                    uv_offset_scale.y += uv_offset_scale.w;
                    uv_offset_scale.w *= -1.0;
                }

                // Override the size if a custom one is specified
                if let Some(custom_size) = extracted_sprite.custom_size {
                    quad_size = custom_size;
                }
                let transform = extracted_sprite.transform.affine()
                    * Affine3A::from_scale_rotation_translation(
                        quad_size.extend(1.0),
                        Quat::IDENTITY,
                        (quad_size * (-extracted_sprite.anchor - Vec2::splat(0.5))).extend(0.0),
                    );

                // Store the vertex data and add the item to the render phase
                sprite_meta
                    .sprite_instance_buffer
                    .push(SpriteInstance::from(&transform, &uv_offset_scale));

                if batch_image_changed || batch_material_changed {
                    batch_item_index = item_index;

                    batches.push((
                        item.entity,
                        SpriteBatch {
                            image_handle_id: batch_image_handle,
                            material_handle_id: batch_material_handle,
                            range: index..index,
                        },
                    ));
                }

                transparent_phase.items[batch_item_index]
                    .batch_range_mut()
                    .end += 1;
                batches.last_mut().unwrap().1.range.end += 1;
                index += 1;
            }
        }
        sprite_meta
            .sprite_instance_buffer
            .write_buffer(&render_device, &render_queue);

        if sprite_meta.sprite_index_buffer.len() != 6 {
            sprite_meta.sprite_index_buffer.clear();

            // NOTE: This code is creating 6 indices pointing to 4 vertices.
            // The vertices form the corners of a quad based on their two least significant bits.
            // 10   11
            //
            // 00   01
            // The sprite shader can then use the two least significant bits as the vertex index.
            // The rest of the properties to transform the vertex positions and UVs (which are
            // implicit) are baked into the instance transform, and UV offset and scale.
            // See bevy_sprite/src/render/sprite.wgsl for the details.
            sprite_meta.sprite_index_buffer.push(2);
            sprite_meta.sprite_index_buffer.push(0);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(3);
            sprite_meta.sprite_index_buffer.push(2);

            sprite_meta
                .sprite_index_buffer
                .write_buffer(&render_device, &render_queue);
        }

        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
}

pub type DrawSprite<M> = (
    SetItemPipeline,
    SetSpriteViewBindGroup<M, 0>,
    SetSpriteTextureBindGroup<M, 1>,
    SetSpriteMaterialBindGroup<M, 2>,
    DrawSpriteBatch<M>,
);

pub struct SetSpriteViewBindGroup<M: SpriteMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: SpriteMaterial, const I: usize> RenderCommand<P>
    for SetSpriteViewBindGroup<M, I>
{
    type Param = SRes<SpriteMeta<M>>;
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'_ ViewUniformOffset,
        _entity: (),
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            sprite_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}
pub struct SetSpriteTextureBindGroup<M: SpriteMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: SpriteMaterial, const I: usize> RenderCommand<P>
    for SetSpriteTextureBindGroup<M, I>
{
    type Param = SRes<ImageBindGroups>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch<M>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch<M>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(
            I,
            image_bind_groups
                .values
                .get(&batch.image_handle_id)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub struct SetSpriteMaterialBindGroup<M: SpriteMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: SpriteMaterial, const I: usize> RenderCommand<P>
    for SetSpriteMaterialBindGroup<M, I>
{
    type Param = SRes<RenderMaterials<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch<M>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: ROQueryItem<'_, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(material) = materials
            .into_inner()
            .get(&material_handle.material_handle_id)
        else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawSpriteBatch<M: SpriteMaterial>(PhantomData<M>);
impl<P: PhaseItem, M: SpriteMaterial> RenderCommand<P> for DrawSpriteBatch<M> {
    type Param = SRes<SpriteMeta<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch<M>>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch<M>,
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(
            0,
            sprite_meta
                .sprite_instance_buffer
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterials<T: SpriteMaterial>(HashMap<AssetId<T>, PreparedSpriteMaterial<T>>);

impl<T: SpriteMaterial> Default for RenderMaterials<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct PreparedSpriteMaterial<T: SpriteMaterial> {
    pub bindings: Vec<(u32, OwnedBindingResource)>,
    pub bind_group: BindGroup,
    pub key: T::Data,
}

#[derive(Resource)]
pub struct ExtractedSpriteMaterials<M: SpriteMaterial> {
    extracted: Vec<(AssetId<M>, M)>,
    removed: Vec<AssetId<M>>,
}

impl<M: SpriteMaterial> Default for ExtractedSpriteMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

pub fn extract_sprite_materials<M: SpriteMaterial>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.push(*id);
            }
            AssetEvent::LoadedWithDependencies { .. } => {
                // not implemented
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            extracted_assets.push((id, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedSpriteMaterials {
        extracted: extracted_assets,
        removed,
    });
}

pub struct PrepareNextFrameMaterials<M: SpriteMaterial> {
    assets: Vec<(AssetId<M>, M)>,
}

impl<M: SpriteMaterial> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

pub fn prepare_sprite_materials<M: SpriteMaterial>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedSpriteMaterials<M>>,
    mut render_materials: ResMut<RenderMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<SpriteMaterialPipeline<M>>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (id, material) in queued_assets {
        match prepare_sprite_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(id, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((id, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.remove(&removed);
    }

    for (handle, material) in std::mem::take(&mut extracted_assets.extracted) {
        match prepare_sprite_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }
}

fn prepare_sprite_material<M: SpriteMaterial>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &Res<FallbackImage>,
    pipeline: &SpriteMaterialPipeline<M>,
) -> Result<PreparedSpriteMaterial<M>, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &pipeline.material_layout,
        render_device,
        images,
        fallback_image,
    )?;
    Ok(PreparedSpriteMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        key: prepared.data,
    })
}
