use crate::{
    AmbientLight, DirectionalLight, DirectionalLightShadowMap, ExtractedMeshes, MeshMeta,
    PbrShaders, PointLight, PointLightShadowMap,
};
use bevy_core_pipeline::Transparent3dPhase;
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_math::{const_vec3, Mat4, Vec3, Vec4};
use bevy_render2::{
    camera::CameraProjection,
    color::Color,
    mesh::Mesh,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{Draw, DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    shader::Shader,
    texture::*,
    view::{ExtractedView, ViewUniformOffset},
};
use bevy_transform::components::GlobalTransform;
use crevice::std140::AsStd140;
use std::num::NonZeroU32;

pub struct ExtractedAmbientLight {
    color: Color,
    brightness: f32,
}

pub struct ExtractedPointLight {
    color: Color,
    /// luminous intensity in lumens per steradian
    intensity: f32,
    range: f32,
    radius: f32,
    transform: GlobalTransform,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

pub type ExtractedPointLightShadowMap = PointLightShadowMap;

pub struct ExtractedDirectionalLight {
    color: Color,
    illuminance: f32,
    direction: Vec3,
    projection: Mat4,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

pub type ExtractedDirectionalLightShadowMap = DirectionalLightShadowMap;

#[repr(C)]
#[derive(Copy, Clone, AsStd140, Default, Debug)]
pub struct GpuPointLight {
    projection: Mat4,
    color: Vec4,
    position: Vec3,
    inverse_square_range: f32,
    radius: f32,
    near: f32,
    far: f32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

#[repr(C)]
#[derive(Copy, Clone, AsStd140, Default, Debug)]
pub struct GpuDirectionalLight {
    view_projection: Mat4,
    color: Vec4,
    dir_to_light: Vec3,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, AsStd140)]
pub struct GpuLights {
    // TODO: this comes first to work around a WGSL alignment issue. We need to solve this issue before releasing the renderer rework
    point_lights: [GpuPointLight; MAX_POINT_LIGHTS],
    directional_lights: [GpuDirectionalLight; MAX_DIRECTIONAL_LIGHTS],
    ambient_color: Vec4,
    n_point_lights: u32,
    n_directional_lights: u32,
}

// NOTE: this must be kept in sync with the same constants in pbr.frag
pub const MAX_POINT_LIGHTS: usize = 10;
pub const MAX_DIRECTIONAL_LIGHTS: usize = 1;
pub const POINT_SHADOW_LAYERS: u32 = (6 * MAX_POINT_LIGHTS) as u32;
pub const DIRECTIONAL_SHADOW_LAYERS: u32 = MAX_DIRECTIONAL_LIGHTS as u32;
pub const SHADOW_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub struct ShadowShaders {
    pub shader_module: ShaderModule,
    pub pipeline: RenderPipeline,
    pub view_layout: BindGroupLayout,
    pub point_light_sampler: Sampler,
    pub directional_light_sampler: Sampler,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for ShadowShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let pbr_shaders = world.get_resource::<PbrShaders>().unwrap();
        let shader = Shader::from_wgsl(include_str!("depth.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&view_layout, &pbr_shaders.mesh_layout],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: 32,
                    step_mode: InputStepMode::Vertex,
                    attributes: &[
                        // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 0,
                        },
                        // Normal
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        },
                        // Uv
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                    ],
                }],
                module: &shader_module,
                entry_point: "vertex",
            },
            fragment: None,
            depth_stencil: Some(DepthStencilState {
                format: SHADOW_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            layout: Some(&pipeline_layout),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
        });

        ShadowShaders {
            shader_module,
            pipeline,
            view_layout,
            point_light_sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareFunction::GreaterEqual),
                ..Default::default()
            }),
            directional_light_sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareFunction::GreaterEqual),
                ..Default::default()
            }),
        }
    }
}

// TODO: ultimately these could be filtered down to lights relevant to actual views
pub fn extract_lights(
    mut commands: Commands,
    ambient_light: Res<AmbientLight>,
    point_light_shadow_map: Res<PointLightShadowMap>,
    directional_light_shadow_map: Res<DirectionalLightShadowMap>,
    point_lights: Query<(Entity, &PointLight, &GlobalTransform)>,
    directional_lights: Query<(Entity, &DirectionalLight, &GlobalTransform)>,
) {
    commands.insert_resource(ExtractedAmbientLight {
        color: ambient_light.color,
        brightness: ambient_light.brightness,
    });
    commands.insert_resource::<ExtractedPointLightShadowMap>(point_light_shadow_map.clone());
    commands.insert_resource::<ExtractedDirectionalLightShadowMap>(
        directional_light_shadow_map.clone(),
    );
    // This is the point light shadow map texel size for one face of the cube as a distance of 1.0
    // world unit from the light.
    // point_light_texel_size = 2.0 * 1.0 * tan(PI / 4.0) / cube face width in texels
    // PI / 4.0 is half the cube face fov, tan(PI / 4.0) = 1.0, so this simplifies to:
    // point_light_texel_size = 2.0 / cube face width in texels
    // NOTE: When using various PCF kernel sizes, this will need to be adjusted, according to:
    // https://catlikecoding.com/unity/tutorials/custom-srp/point-and-spot-shadows/
    let point_light_texel_size = 2.0 / point_light_shadow_map.size as f32;
    for (entity, point_light, transform) in point_lights.iter() {
        commands.get_or_spawn(entity).insert(ExtractedPointLight {
            color: point_light.color,
            // NOTE: Map from luminous power in lumens to luminous intensity in lumens per steradian
            // for a point light. See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower
            // for details.
            intensity: point_light.intensity / (4.0 * std::f32::consts::PI),
            range: point_light.range,
            radius: point_light.radius,
            transform: *transform,
            shadow_depth_bias: point_light.shadow_depth_bias,
            // The factor of SQRT_2 is for the worst-case diagonal offset
            shadow_normal_bias: point_light.shadow_normal_bias
                * point_light_texel_size
                * std::f32::consts::SQRT_2,
        });
    }
    for (entity, directional_light, transform) in directional_lights.iter() {
        // Calulate the directional light shadow map texel size using the largest x,y dimension of
        // the orthographic projection divided by the shadow map resolution
        // NOTE: When using various PCF kernel sizes, this will need to be adjusted, according to:
        // https://catlikecoding.com/unity/tutorials/custom-srp/directional-shadows/
        let largest_dimension = (directional_light.shadow_projection.right
            - directional_light.shadow_projection.left)
            .max(
                directional_light.shadow_projection.top
                    - directional_light.shadow_projection.bottom,
            );
        let directional_light_texel_size =
            largest_dimension / directional_light_shadow_map.size as f32;
        commands
            .get_or_spawn(entity)
            .insert(ExtractedDirectionalLight {
                color: directional_light.color,
                illuminance: directional_light.illuminance,
                direction: transform.forward(),
                projection: directional_light.shadow_projection.get_projection_matrix(),
                shadow_depth_bias: directional_light.shadow_depth_bias,
                // The factor of SQRT_2 is for the worst-case diagonal offset
                shadow_normal_bias: directional_light.shadow_normal_bias
                    * directional_light_texel_size
                    * std::f32::consts::SQRT_2,
            });
    }
}

// Can't do `Vec3::Y * -1.0` because mul isn't const
const NEGATIVE_X: Vec3 = const_vec3!([-1.0, 0.0, 0.0]);
const NEGATIVE_Y: Vec3 = const_vec3!([0.0, -1.0, 0.0]);
const NEGATIVE_Z: Vec3 = const_vec3!([0.0, 0.0, -1.0]);

struct CubeMapFace {
    target: Vec3,
    up: Vec3,
}

// see https://www.khronos.org/opengl/wiki/Cubemap_Texture
const CUBE_MAP_FACES: [CubeMapFace; 6] = [
    // 0 	GL_TEXTURE_CUBE_MAP_POSITIVE_X
    CubeMapFace {
        target: NEGATIVE_X,
        up: NEGATIVE_Y,
    },
    // 1 	GL_TEXTURE_CUBE_MAP_NEGATIVE_X
    CubeMapFace {
        target: Vec3::X,
        up: NEGATIVE_Y,
    },
    // 2 	GL_TEXTURE_CUBE_MAP_POSITIVE_Y
    CubeMapFace {
        target: NEGATIVE_Y,
        up: Vec3::Z,
    },
    // 3 	GL_TEXTURE_CUBE_MAP_NEGATIVE_Y
    CubeMapFace {
        target: Vec3::Y,
        up: NEGATIVE_Z,
    },
    // 4 	GL_TEXTURE_CUBE_MAP_POSITIVE_Z
    CubeMapFace {
        target: NEGATIVE_Z,
        up: NEGATIVE_Y,
    },
    // 5 	GL_TEXTURE_CUBE_MAP_NEGATIVE_Z
    CubeMapFace {
        target: Vec3::Z,
        up: NEGATIVE_Y,
    },
];

fn face_index_to_name(face_index: usize) -> &'static str {
    match face_index {
        0 => "+x",
        1 => "-x",
        2 => "+y",
        3 => "-y",
        4 => "+z",
        5 => "-z",
        _ => "invalid",
    }
}

pub struct ViewLight {
    pub depth_texture_view: TextureView,
    pub pass_name: String,
}

pub struct ViewLights {
    pub point_light_depth_texture: Texture,
    pub point_light_depth_texture_view: TextureView,
    pub directional_light_depth_texture: Texture,
    pub directional_light_depth_texture_view: TextureView,
    pub lights: Vec<Entity>,
    pub gpu_light_binding_index: u32,
}

#[derive(Default)]
pub struct LightMeta {
    pub view_gpu_lights: DynamicUniformVec<GpuLights>,
    pub shadow_view_bind_group: Option<BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_lights(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut light_meta: ResMut<LightMeta>,
    views: Query<Entity, With<RenderPhase<Transparent3dPhase>>>,
    ambient_light: Res<ExtractedAmbientLight>,
    point_light_shadow_map: Res<ExtractedPointLightShadowMap>,
    directional_light_shadow_map: Res<ExtractedDirectionalLightShadowMap>,
    point_lights: Query<&ExtractedPointLight>,
    directional_lights: Query<&ExtractedDirectionalLight>,
) {
    // PERF: view.iter().count() could be views.iter().len() if we implemented ExactSizeIterator for archetype-only filters
    light_meta
        .view_gpu_lights
        .reserve_and_clear(views.iter().count(), &render_device);

    let ambient_color = ambient_light.color.as_rgba_linear() * ambient_light.brightness;
    // set up light data for each view
    for entity in views.iter() {
        let point_light_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                size: Extent3d {
                    width: point_light_shadow_map.size as u32,
                    height: point_light_shadow_map.size as u32,
                    depth_or_array_layers: POINT_SHADOW_LAYERS,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SHADOW_FORMAT,
                usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
                label: None,
            },
        );
        let directional_light_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                size: Extent3d {
                    width: directional_light_shadow_map.size as u32,
                    height: directional_light_shadow_map.size as u32,
                    depth_or_array_layers: DIRECTIONAL_SHADOW_LAYERS,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SHADOW_FORMAT,
                usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
                label: None,
            },
        );
        let mut view_lights = Vec::new();

        let mut gpu_lights = GpuLights {
            ambient_color: ambient_color.into(),
            n_point_lights: point_lights.iter().len() as u32,
            n_directional_lights: directional_lights.iter().len() as u32,
            point_lights: [GpuPointLight::default(); MAX_POINT_LIGHTS],
            directional_lights: [GpuDirectionalLight::default(); MAX_DIRECTIONAL_LIGHTS],
        };

        // TODO: this should select lights based on relevance to the view instead of the first ones that show up in a query
        for (light_index, light) in point_lights.iter().enumerate().take(MAX_POINT_LIGHTS) {
            let projection =
                Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1);

            // ignore scale because we don't want to effectively scale light radius and range
            // by applying those as a view transform to shadow map rendering of objects
            // and ignore rotation because we want the shadow map projections to align with the axes
            let view_translation = GlobalTransform::from_translation(light.transform.translation);

            for (face_index, CubeMapFace { target, up }) in CUBE_MAP_FACES.iter().enumerate() {
                // use the cubemap projection direction
                let view_rotation = GlobalTransform::identity().looking_at(*target, *up);

                let depth_texture_view =
                    point_light_depth_texture
                        .texture
                        .create_view(&TextureViewDescriptor {
                            label: None,
                            format: None,
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: (light_index * 6 + face_index) as u32,
                            array_layer_count: NonZeroU32::new(1),
                        });

                let view_light_entity = commands
                    .spawn()
                    .insert_bundle((
                        ViewLight {
                            depth_texture_view,
                            pass_name: format!(
                                "shadow pass point light {} {}",
                                light_index,
                                face_index_to_name(face_index)
                            ),
                        },
                        ExtractedView {
                            width: point_light_shadow_map.size as u32,
                            height: point_light_shadow_map.size as u32,
                            transform: view_translation * view_rotation,
                            projection,
                        },
                        RenderPhase::<ShadowPhase>::default(),
                    ))
                    .id();
                view_lights.push(view_light_entity);
            }

            gpu_lights.point_lights[light_index] = GpuPointLight {
                projection,
                // premultiply color by intensity
                // we don't use the alpha at all, so no reason to multiply only [0..3]
                color: (light.color.as_rgba_linear() * light.intensity).into(),
                radius: light.radius,
                position: light.transform.translation,
                inverse_square_range: 1.0 / (light.range * light.range),
                near: 0.1,
                far: light.range,
                shadow_depth_bias: light.shadow_depth_bias,
                shadow_normal_bias: light.shadow_normal_bias,
            };
        }

        for (i, light) in directional_lights
            .iter()
            .enumerate()
            .take(MAX_DIRECTIONAL_LIGHTS)
        {
            // direction is negated to be ready for N.L
            let dir_to_light = -light.direction;

            // convert from illuminance (lux) to candelas
            //
            // exposure is hard coded at the moment but should be replaced
            // by values coming from the camera
            // see: https://google.github.io/filament/Filament.html#imagingpipeline/physicallybasedcamera/exposuresettings
            const APERTURE: f32 = 4.0;
            const SHUTTER_SPEED: f32 = 1.0 / 250.0;
            const SENSITIVITY: f32 = 100.0;
            let ev100 =
                f32::log2(APERTURE * APERTURE / SHUTTER_SPEED) - f32::log2(SENSITIVITY / 100.0);
            let exposure = 1.0 / (f32::powf(2.0, ev100) * 1.2);
            let intensity = light.illuminance * exposure;

            // NOTE: A directional light seems to have to have an eye position on the line along the direction of the light
            //       through the world origin. I (Rob Swain) do not yet understand why it cannot be translated away from this.
            let view = Mat4::look_at_rh(Vec3::ZERO, light.direction, Vec3::Y);
            // NOTE: This orthographic projection defines the volume within which shadows from a directional light can be cast
            let projection = light.projection;

            gpu_lights.directional_lights[i] = GpuDirectionalLight {
                // premultiply color by intensity
                // we don't use the alpha at all, so no reason to multiply only [0..3]
                color: (light.color.as_rgba_linear() * intensity).into(),
                dir_to_light,
                // NOTE: * view is correct, it should not be view.inverse() here
                view_projection: projection * view,
                shadow_depth_bias: light.shadow_depth_bias,
                shadow_normal_bias: light.shadow_normal_bias,
            };

            let depth_texture_view =
                directional_light_depth_texture
                    .texture
                    .create_view(&TextureViewDescriptor {
                        label: None,
                        format: None,
                        dimension: Some(TextureViewDimension::D2),
                        aspect: TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: i as u32,
                        array_layer_count: NonZeroU32::new(1),
                    });

            let view_light_entity = commands
                .spawn()
                .insert_bundle((
                    ViewLight {
                        depth_texture_view,
                        pass_name: format!("shadow pass directional light {}", i),
                    },
                    ExtractedView {
                        width: directional_light_shadow_map.size as u32,
                        height: directional_light_shadow_map.size as u32,
                        transform: GlobalTransform::from_matrix(view.inverse()),
                        projection,
                    },
                    RenderPhase::<ShadowPhase>::default(),
                ))
                .id();
            view_lights.push(view_light_entity);
        }
        let point_light_depth_texture_view =
            point_light_depth_texture
                .texture
                .create_view(&TextureViewDescriptor {
                    label: None,
                    format: None,
                    dimension: Some(TextureViewDimension::CubeArray),
                    aspect: TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });
        let directional_light_depth_texture_view = directional_light_depth_texture
            .texture
            .create_view(&TextureViewDescriptor {
                label: None,
                format: None,
                dimension: Some(TextureViewDimension::D2Array),
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });

        commands.entity(entity).insert(ViewLights {
            point_light_depth_texture: point_light_depth_texture.texture,
            point_light_depth_texture_view,
            directional_light_depth_texture: directional_light_depth_texture.texture,
            directional_light_depth_texture_view,
            lights: view_lights,
            gpu_light_binding_index: light_meta.view_gpu_lights.push(gpu_lights),
        });
    }

    light_meta
        .view_gpu_lights
        .write_to_staging_buffer(&render_device);
}

pub struct ShadowPhase;

pub struct ShadowPassNode {
    main_view_query: QueryState<&'static ViewLights>,
    view_light_query: QueryState<(&'static ViewLight, &'static RenderPhase<ShadowPhase>)>,
}

impl ShadowPassNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            view_light_query: QueryState::new(world),
        }
    }
}

impl Node for ShadowPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(ShadowPassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.view_light_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        if let Ok(view_lights) = self.main_view_query.get_manual(world, view_entity) {
            for view_light_entity in view_lights.lights.iter().copied() {
                let (view_light, shadow_phase) = self
                    .view_light_query
                    .get_manual(world, view_light_entity)
                    .unwrap();
                let pass_descriptor = RenderPassDescriptor {
                    label: Some(&view_light.pass_name),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &view_light.depth_texture_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(0.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                };

                let draw_functions = world.get_resource::<DrawFunctions>().unwrap();

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);
                let mut draw_functions = draw_functions.write();
                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                for drawable in shadow_phase.drawn_things.iter() {
                    let draw_function = draw_functions.get_mut(drawable.draw_function).unwrap();
                    draw_function.draw(
                        world,
                        &mut tracked_pass,
                        view_light_entity,
                        drawable.draw_key,
                        drawable.sort_key,
                    );
                }
            }
        }

        Ok(())
    }
}

type DrawShadowMeshParams<'s, 'w> = (
    Res<'w, ShadowShaders>,
    Res<'w, ExtractedMeshes>,
    Res<'w, LightMeta>,
    Res<'w, MeshMeta>,
    Res<'w, RenderAssets<Mesh>>,
    Query<'w, 's, &'w ViewUniformOffset>,
);
pub struct DrawShadowMesh {
    params: SystemState<DrawShadowMeshParams<'static, 'static>>,
}

impl DrawShadowMesh {
    pub fn new(world: &mut World) -> Self {
        Self {
            params: SystemState::new(world),
        }
    }
}

impl Draw for DrawShadowMesh {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        draw_key: usize,
        _sort_key: usize,
    ) {
        let (shadow_shaders, extracted_meshes, light_meta, mesh_meta, meshes, views) =
            self.params.get(world);
        let view_uniform_offset = views.get(view).unwrap();
        let extracted_mesh = &extracted_meshes.into_inner().meshes[draw_key];
        let shadow_shaders = shadow_shaders.into_inner();
        pass.set_render_pipeline(&shadow_shaders.pipeline);
        pass.set_bind_group(
            0,
            light_meta
                .into_inner()
                .shadow_view_bind_group
                .as_ref()
                .unwrap(),
            &[view_uniform_offset.offset],
        );

        let transform_bindgroup_key = mesh_meta.mesh_transform_bind_group_key.unwrap();
        pass.set_bind_group(
            1,
            mesh_meta
                .into_inner()
                .mesh_transform_bind_group
                .get_value(transform_bindgroup_key)
                .unwrap(),
            &[extracted_mesh.transform_binding_offset],
        );

        let gpu_mesh = meshes.into_inner().get(&extracted_mesh.mesh).unwrap();
        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        if let Some(index_info) = &gpu_mesh.index_info {
            pass.set_index_buffer(index_info.buffer.slice(..), 0, IndexFormat::Uint32);
            pass.draw_indexed(0..index_info.count, 0, 0..1);
        } else {
            panic!("non-indexed drawing not supported yet")
        }
    }
}
