use crate::{
    DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup,
};
use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::Opaque3d;
use bevy_ecs::{
    prelude::*,
    query::QueryItem,
    reflect::ReflectComponent,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_math::Vec4;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    color::Color,
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_component::{ExtractComponent, ExtractComponentPlugin},
    render_phase::{
        AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
        SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
        BufferSize, DynamicUniformVec,
    },
    render_resource::{
        PipelineCache, PolygonMode, RenderPipelineDescriptor, Shader, ShaderStages,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    renderer::{RenderDevice, RenderQueue},
    view::{ExtractedView, Msaa, VisibleEntities},
    RenderApp, RenderStage,
};
use bevy_utils::tracing::error;

pub const WIREFRAME_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 192598014480025766);

/// A [`Plugin`] that draws wireframes.
#[derive(Debug, Default)]
pub struct WireframePlugin;

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            WIREFRAME_SHADER_HANDLE,
            "render/wireframe.wgsl",
            Shader::from_wgsl
        );

        app.init_resource::<WireframeConfig>()
            .add_plugin(ExtractComponentPlugin::<Wireframe>::extract_visible())
            .add_plugin(ExtractComponentPlugin::<WireframeColor>::extract_visible());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Opaque3d, DrawWireframes>()
                .init_resource::<GlobalWireframeMeta>()
                .init_resource::<WireframePipeline>()
                .init_resource::<SpecializedMeshPipelines<WireframePipeline>>()
                .add_system_to_stage(RenderStage::Extract, extract_wireframe_config)
                .add_system_to_stage(RenderStage::Prepare, prepare_wireframes)
                .add_system_to_stage(RenderStage::Queue, queue_wireframes_bind_group)
                .add_system_to_stage(RenderStage::Queue, queue_wireframes);
        }
    }
}

fn extract_wireframe_config(mut commands: Commands, wireframe_config: Res<WireframeConfig>) {
    if wireframe_config.is_added() || wireframe_config.is_changed() {
        commands.insert_resource(wireframe_config.into_inner().clone());
    }
}

#[allow(clippy::type_complexity)]
fn prepare_wireframes(
    mut commands: Commands,
    config: Res<WireframeConfig>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut wireframe_meta: ResMut<GlobalWireframeMeta>,
    global_query: Query<(Entity, Option<&WireframeColor>), (With<Handle<Mesh>>, With<MeshUniform>)>,
    wireframe_query: Query<
        (Entity, Option<&WireframeColor>),
        (With<Handle<Mesh>>, With<MeshUniform>, With<Wireframe>),
    >,
) {
    wireframe_meta.uniforms.clear();
    wireframe_meta.uniforms.push(WireframeUniform {
        color: config.default_color.as_linear_rgba_f32().into(),
    });

    let add_wireframe_uniform = |(entity, wireframe_color): (Entity, Option<&WireframeColor>)| {
        let override_color = wireframe_color.map(|wireframe_color| wireframe_color.0);
        let uniform_offset = WireframeUniformOffset(if let Some(override_color) = override_color {
            wireframe_meta.uniforms.push(WireframeUniform {
                color: override_color.as_linear_rgba_f32().into(),
            })
        } else {
            0
        });
        commands.entity(entity).insert(uniform_offset);
    };

    if config.on_all_meshes {
        global_query.for_each(add_wireframe_uniform);
    } else {
        wireframe_query.for_each(add_wireframe_uniform);
    }

    wireframe_meta
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

/// Stores the [`BindGroup`] of wireframe data that is used on the GPU side.
///
/// Internal [`WireframePlugin`] resource.
struct GlobalWireframeBindGroup {
    bind_group: BindGroup,
}

fn queue_wireframes_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    meta: Res<GlobalWireframeMeta>,
    bind_group: Option<Res<GlobalWireframeBindGroup>>,
) {
    if bind_group.is_none() {
        commands.insert_resource(GlobalWireframeBindGroup {
            bind_group: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: meta.uniforms.binding().unwrap(),
                }],
                label: Some("wireframe_bind_group"),
                layout: &meta.bind_group_layout,
            }),
        });
    }
}

/// Toggles wireframe rendering for any entity it is attached to.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Default, Copy, Clone, Reflect)]
#[reflect(Component)]
pub struct Wireframe;

impl ExtractComponent for Wireframe {
    type Query = &'static Wireframe;

    type Filter = ();

    #[inline]
    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        *item
    }
}

/// Sets the color of the [`Wireframe`] of the entity it is attached to.
///
/// This overrides the [`WireframeConfig::default_color`].
#[derive(Component, Debug, Default, Copy, Clone, Reflect)]
#[reflect(Component)]
pub struct WireframeColor(pub Color);

impl ExtractComponent for WireframeColor {
    type Query = &'static WireframeColor;

    type Filter = ();

    #[inline]
    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        *item
    }
}

/// Configuration resource for [`WireframePlugin`].
#[derive(Debug, Clone)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes. If `false`, only meshes with a [`Wireframe`] component will be rendered.
    pub on_all_meshes: bool,
    /// The default color for wireframes.
    ///
    /// If [`Self::on_all_meshes`] is set, any [`Entity`] that does not have a [`Wireframe`] component attached to it will have
    /// wireframes in this color. Otherwise, this will be the fallback color for any entity that has a [`Wireframe`],
    /// but no [`WireframeColor`].
    pub default_color: Color,
}

impl Default for WireframeConfig {
    fn default() -> Self {
        Self {
            on_all_meshes: false,
            default_color: Color::WHITE,
        }
    }
}

/// Holds the offset of a [`WireframeUniform`] in the [`GlobalWireframeMeta::uniforms`].
///
/// Internal [`WireframePlugin`] component.
#[derive(Component, Copy, Clone, Debug, Default)]
#[repr(transparent)]
struct WireframeUniformOffset(u32);

/// [`WireframeUniform`] is the GPU representation of a [`Wireframe`].
///
/// Internal [`WireframePlugin`] state.
#[derive(Debug, AsStd140)]
struct WireframeUniform {
    color: Vec4,
}

/// The data required for rendering [`Wireframe`]s.
///
/// Internal [`WireframePlugin`] resource.
#[derive(Component)]
struct GlobalWireframeMeta {
    uniforms: DynamicUniformVec<WireframeUniform>,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for GlobalWireframeMeta {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(
                            WireframeUniform::std140_size_static() as u64
                        ),
                    },
                    count: None,
                }],
                label: Some("wireframe_bind_group_layout"),
            });

        Self {
            uniforms: Default::default(),
            bind_group_layout,
        }
    }
}

/// [`WireframePipeline`] is the specialized rendering pipeline for wireframes.
///
/// Internal [`WireframePlugin`] resource.
struct WireframePipeline {
    mesh_pipeline: MeshPipeline,
    wireframe_bind_group_layout: BindGroupLayout,
    shader: Handle<Shader>,
}
impl FromWorld for WireframePipeline {
    fn from_world(render_world: &mut World) -> Self {
        WireframePipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            wireframe_bind_group_layout: render_world
                .get_resource::<GlobalWireframeMeta>()
                .unwrap()
                .bind_group_layout
                .clone(),
            shader: WIREFRAME_SHADER_HANDLE.typed(),
        }
    }
}

impl SpecializedMeshPipeline for WireframePipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone_weak();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone_weak();
        descriptor
            .layout
            .as_mut()
            .unwrap()
            .push(self.wireframe_bind_group_layout.clone());
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn queue_wireframes(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    wireframe_config: Res<WireframeConfig>,
    wireframe_pipeline: Res<WireframePipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WireframePipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    mut material_meshes: ParamSet<(
        Query<(Entity, &Handle<Mesh>, &MeshUniform)>,
        Query<(Entity, &Handle<Mesh>, &MeshUniform), With<Wireframe>>,
    )>,
    mut views: Query<(&ExtractedView, &VisibleEntities, &mut RenderPhase<Opaque3d>)>,
) {
    let draw_custom = opaque_3d_draw_functions
        .read()
        .get_id::<DrawWireframes>()
        .unwrap();
    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);
    for (view, visible_entities, mut opaque_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);

        let add_render_phase =
            |(entity, mesh_handle, mesh_uniform): (Entity, &Handle<Mesh>, &MeshUniform)| {
                if let Some(mesh) = render_meshes.get(mesh_handle) {
                    let key = msaa_key
                        | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                    let pipeline_id = pipelines.specialize(
                        &mut pipeline_cache,
                        &wireframe_pipeline,
                        key,
                        &mesh.layout,
                    );
                    let pipeline_id = match pipeline_id {
                        Ok(id) => id,
                        Err(err) => {
                            error!("{}", err);
                            return;
                        }
                    };
                    opaque_phase.add(Opaque3d {
                        entity,
                        pipeline: pipeline_id,
                        draw_function: draw_custom,
                        distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                    });
                }
            };

        if wireframe_config.on_all_meshes {
            material_meshes.p0().iter().for_each(add_render_phase);
        } else {
            let query = material_meshes.p1();
            visible_entities
                .entities
                .iter()
                .filter_map(|visible_entity| query.get(*visible_entity).ok())
                .for_each(add_render_phase);
        }
    }
}

/// [`SetWireframeBindGroup`]`<bindgroup index>` binds the [`GlobalWireframeBindGroup`] there.
///
/// Internal [`WireframePlugin`] render command.
struct SetWireframeBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetWireframeBindGroup<I> {
    type Param = (
        SRes<GlobalWireframeBindGroup>,
        SQuery<Read<WireframeUniformOffset>, With<Handle<Mesh>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (global_wireframe_bind_group, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let wireframe_uniform_offset = view_query.get(item).unwrap();
        pass.set_bind_group(
            I,
            &global_wireframe_bind_group.into_inner().bind_group,
            &[wireframe_uniform_offset.0],
        );
        RenderCommandResult::Success
    }
}

type DrawWireframes = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetWireframeBindGroup<2>,
    DrawMesh,
);
