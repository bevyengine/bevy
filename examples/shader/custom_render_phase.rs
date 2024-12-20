//! This example demonstrates how to write a custom phase
//!
//! Render phases in bevy are used whenever you need to draw a groud of neshes in a specific way.
//! For example, bevy's main pass has an opaque phase, a transparent phase for both 2d and 3d.
//! Sometimes, you may want to only draw a subset of meshes before or after the builtin phase. In
//! those situations you need to write your own phase.

use std::ops::Range;

use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::{
        entity::EntityHashSet,
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::FloatOrd,
    pbr::{
        material_bind_groups::MaterialBindGroupSlot, DrawMesh, MeshInputUniform, MeshPipeline,
        MeshPipelineKey, MeshPipelineViewLayoutKey, MeshUniform, RenderMeshInstances,
        SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
};
use bevy_render::{
    batching::{
        gpu_preprocessing::{batch_and_prepare_sorted_render_phase, IndirectParametersBuffer},
        GetBatchData, GetFullBatchData, NoAutomaticBatching,
    },
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    mesh::{allocator::MeshAllocator, MeshVertexBufferLayoutRef, RenderMesh},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_graph::{
        NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
    },
    render_phase::{
        sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
        SetItemPipeline, SortedPhaseItem, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout, BindingResources,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
        CompareFunction, DepthStencilState, Face, FragmentState, FrontFace, MultisampleState,
        PipelineCache, PolygonMode, PrimitiveState, RenderPassDescriptor, RenderPipelineDescriptor,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
        TextureFormat, VertexState,
    },
    renderer::{RenderContext, RenderDevice},
    sync_world::{MainEntity, RenderEntity},
    view::{
        check_visibility, ExtractedView, NoIndirectDrawing, RenderVisibleEntities, ViewTarget,
        VisibilitySystems,
    },
    Extract, Render, RenderApp, RenderSet,
};
use nonmax::NonMaxU32;

const SHADER_ASSET_PATH: &str = "shaders/custom_draw_phase.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CustomPhasPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut custom_draw: ResMut<Assets<CustomDrawData>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 2.5, 0.0),
        CustomDrawDataHandle(custom_draw.add(CustomDrawData {
            // Set it to red
            color: Vec4::new(1.0, 0.0, 0.0, 1.0),
        })),
        // TODO temp
        NoAutomaticBatching,
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        // disable msaa for simplicity
        Msaa::Off,
        NoIndirectDrawing,
    ));
}

#[derive(Component, ExtractComponent, Clone, Copy, Default)]
struct CustomDrawMarker;

/// A query filter that tells [`view::check_visibility`] about our custom
/// rendered entity.
type WithCustomDraw = With<CustomDrawMarker>;

struct CustomPhasPlugin;
impl Plugin for CustomPhasPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<CustomDrawData>().add_plugins((
            RenderAssetPlugin::<PreparedCustomDrawData>::default(),
            ExtractComponentPlugin::<CustomDrawMarker>::default(),
        ));
        // Make sure to tell Bevy to check our entity for visibility. Bevy won't
        // do this by default, for efficiency reasons.
        app.add_systems(
            PostUpdate,
            // For this example it isn't stricly necessary, we could rely on the check already done
            // for the base mesh
            check_visibility::<WithCustomDraw>.in_set(VisibilitySystems::CheckVisibility),
        );
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedMeshPipelines<CustomDrawPipeline>>()
            .init_resource::<DrawFunctions<CustomPhase>>()
            .add_render_command::<CustomPhase, DrawCustom>()
            .init_resource::<ViewSortedRenderPhases<CustomPhase>>()
            .add_systems(ExtractSchedule, extract_camera_phases)
            .add_systems(
                Render,
                (
                    sort_phase_system::<CustomPhase>.in_set(RenderSet::PhaseSort),
                    batch_and_prepare_sorted_render_phase::<CustomPhase, CustomDrawPipeline>
                        .in_set(RenderSet::PrepareResources),
                    queue_custom_meshes.in_set(RenderSet::QueueMeshes),
                ),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<CustomDrawNode>>(Core3d, CustomDrawPassLabel)
            // Tell the node to run after the main pass
            .add_render_graph_edges(Core3d, (Node3d::MainOpaquePass, CustomDrawPassLabel));
    }

    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<CustomDrawPipeline>();
    }
}

#[derive(AsBindGroup, Asset, Clone, TypePath)]
struct CustomDrawData {
    #[uniform(0)]
    color: Vec4,
}

#[derive(Resource)]
struct CustomDrawPipeline {
    layout: BindGroupLayout,
    /// The base mesh pipeline defined by bevy
    ///
    /// This isn't required, but if you want to use a bevy `Mesh` it's easier when you
    /// have access to the base `MeshPipeline` that bevy already defines
    mesh_pipeline: MeshPipeline,
    /// Stores the shader used for this pipeline directly on the pipeline.
    /// This isn't required, it's only done like this for simplicity.
    shader_handle: Handle<Shader>,
}
impl FromWorld for CustomDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        // Load the shader
        let shader_handle: Handle<Shader> = world.resource::<AssetServer>().load(SHADER_ASSET_PATH);
        Self {
            layout: CustomDrawData::bind_group_layout(world.resource::<RenderDevice>()),
            mesh_pipeline: MeshPipeline::from_world(world),
            shader_handle,
        }
    }
}
impl SpecializedMeshPipeline for CustomDrawPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        // Define the vertex attributes based on a standard bevy [`Mesh`]
        let mut vertex_attributes = Vec::new();
        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            // Make sure this matches the shader location
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }
        // This will automatically generate the correct `VertexBufferLayout` based on the vertex attributes
        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        Ok(RenderPipelineDescriptor {
            label: Some("Specialized Mesh Pipeline".into()),
            layout: vec![
                // Bind group 0 is the view uniform
                self.mesh_pipeline
                    .get_view_layout(MeshPipelineViewLayoutKey::from(key))
                    .clone(),
                // Bind group 1 is the mesh uniform
                self.mesh_pipeline.mesh_layouts.model_only.clone(),
                // Bind group 2 is our custom data
                // TODO
                //self.layout.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                // Customize how to store the meshes' vertex attributes in the vertex buffer
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: self.shader_handle.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                ..default()
            },
            depth_stencil: None,
            // It's generally recommended to specialize your pipeline for MSAA,
            // but it's not always possible
            multisample: MultisampleState::default(),
            zero_initialize_workgroup_memory: false,
        })
    }
}

/// Data prepared for a custom draw
struct PreparedCustomDrawData {
    bindings: BindingResources,
    bind_group: BindGroup,
}
impl RenderAsset for PreparedCustomDrawData {
    type SourceAsset = CustomDrawData;

    type Param = (
        SRes<RenderDevice>,
        SRes<CustomDrawPipeline>,
        <CustomDrawData as AsBindGroup>::Param,
    );

    fn prepare_asset(
        material: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_device, pipeline, data_param): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        match material.as_bind_group(&pipeline.layout, render_device, data_param) {
            Ok(prepared) => Ok(PreparedCustomDrawData {
                bindings: prepared.bindings,
                bind_group: prepared.bind_group,
            }),
            Err(AsBindGroupError::RetryNextUpdate) => {
                Err(PrepareAssetError::RetryNextUpdate(material))
            }
            Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
        }
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetCustomBindGroup<2>,
    DrawMesh,
);

#[derive(Component)]
#[require(CustomDrawMarker)]
struct CustomDrawDataHandle(Handle<CustomDrawData>);

struct SetCustomBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetCustomBindGroup<I> {
    type Param = SRes<RenderAssets<PreparedCustomDrawData>>;
    type ViewQuery = ();
    type ItemQuery = Read<CustomDrawDataHandle>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        assets: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // TODO
        //let Some(material) = handle.and_then(|handle| assets.into_inner().get(&handle.0)) else {
        //    return RenderCommandResult::Failure("invalid item query");
        //};
        //pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

struct CustomPhase {
    pub sort_key: FloatOrd,
    pub entity: (Entity, MainEntity),
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for CustomPhase {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for CustomPhase {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // bevy normally uses radsort instead of the std slice::sort_by_key
        // radsort is a stable radix sort that performed better than `slice::sort_by_key` or `slice::sort_unstable_by_key`.
        // Since it is not re-exported by bevy, we just use the std sort for the purpose of the example
        items.sort_by_key(SortedPhaseItem::sort_key);
    }
}

impl CachedRenderPipelinePhaseItem for CustomPhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

impl GetBatchData for CustomDrawPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
    );
    type CompareData = AssetId<Mesh>;
    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, render_assets, mesh_allocator): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        Some((
            MeshUniform::new(
                &mesh_instance.transforms,
                first_vertex_index,
                // TODO don't hardcode this
                MaterialBindGroupSlot(0),
                None,
                None,
                None,
            ),
            None,
        ))
    }
}
impl GetFullBatchData for CustomDrawPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (_, _, _): &SystemParamItem<Self::Param>,
        (_entity, _main_entity): (Entity, MainEntity),
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        None
    }

    fn get_binned_batch_data(
        (mesh_instances, _render_assets, mesh_allocator): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index.slot,
            None,
            None,
            None,
        ))
    }

    fn get_binned_index(
        (_, _, _): &SystemParamItem<Self::Param>,
        (_entity, _main_entity): (Entity, MainEntity),
    ) -> Option<NonMaxU32> {
        // This should only be called during GPU building.
        // For this example we don't use GPU building.
        None
    }

    fn get_batch_indirect_parameters_index(
        (_, _, _): &SystemParamItem<Self::Param>,
        _indirect_parameters_buffer: &mut IndirectParametersBuffer,
        _entity: (Entity, MainEntity),
        _instance_index: u32,
    ) -> Option<NonMaxU32> {
        // We don't use gpu preprocessing for this example
        None
    }
}
fn extract_camera_phases(
    mut custom_phases: ResMut<ViewSortedRenderPhases<CustomPhase>>,
    cameras: Extract<Query<(RenderEntity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();
    for (entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }
        custom_phases.insert_or_clear(entity);
        live_entities.insert(entity);
        //println!("phase extracted");
    }
    // Clear out all dead views.
    custom_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

#[allow(clippy::too_many_arguments)]
fn queue_custom_meshes(
    custom_draw_functions: Res<DrawFunctions<CustomPhase>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomDrawPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    custom_draw_pipeline: Res<CustomDrawPipeline>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    mut custom_render_phases: ResMut<ViewSortedRenderPhases<CustomPhase>>,
    mut views: Query<(Entity, &ExtractedView, &RenderVisibleEntities, &Msaa)>,
) {
    for (view_entity, view, visible_entities, msaa) in &mut views {
        let Some(custom_phase) = custom_render_phases.get_mut(&view_entity) else {
            continue;
        };
        let draw_custom = custom_draw_functions.read().id::<DrawCustom>();

        // Create the key based on the view.
        // In this case we only care about MSAA and HDR
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        let rangefinder = view.rangefinder3d();
        for (render_entity, visible_entity) in visible_entities.iter::<WithCustomDraw>() {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            // Specialize the key for the current mesh entity
            // For this example we only specialize based on the mesh topology
            // but you could have more complex keys and that's where you'd need to create those keys
            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &custom_draw_pipeline,
                mesh_key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };
            let distance = rangefinder.distance_translation(&mesh_instance.translation);
            custom_phase.add(CustomPhase {
                // Sort the data based on the distance to the view
                sort_key: FloatOrd(distance),
                entity: (*render_entity, *visible_entity),
                pipeline: pipeline_id,
                draw_function: draw_custom,
                // Sorted phase items aren't batched
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
            });
        }
    }
}

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct CustomDrawPassLabel;

#[derive(Default)]
struct CustomDrawNode;
impl ViewNode for CustomDrawNode {
    type ViewQuery = (&'static ExtractedCamera, &'static ViewTarget);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, target): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(custom_phases) = world.get_resource::<ViewSortedRenderPhases<CustomPhase>>()
        else {
            return Ok(());
        };
        // not reguired but makes profiling easier
        let diagnostics = render_context.diagnostic_recorder();

        let color_attachments = [Some(target.get_color_attachment())];

        let view_entity = graph.view_entity();

        let Some(custom_phase) = custom_phases.get(&view_entity) else {
            return Ok(());
        };

        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _ = info_span!("custom phase pass").entered();

            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("custom pass encoder"),
                });

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("custom pass"),
                color_attachments: &color_attachments,
                // We don't bind any depth buffer for this pass
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
            let pass_span = diagnostics.pass_span(&mut render_pass, "custom_pass");

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Opaque draws
            if !custom_phase.items.is_empty() {
                #[cfg(feature = "trace")]
                let _ = info_span!("custom pass").entered();
                if let Err(err) = custom_phase.render(&mut render_pass, world, view_entity) {
                    error!("Error encountered while rendering the custom phase {err:?}");
                }
            }

            pass_span.end(&mut render_pass);
            drop(render_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}
