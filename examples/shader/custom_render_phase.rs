//! This example demonstrates how to write a custom phase
//!
//! Render phases in bevy are used whenever you need to draw a groud of neshes in a specific way.
//! For example, bevy's main pass has an opaque phase, a transparent phase for both 2d and 3d.
//! Sometimes, you may want to only draw a subset of meshes before or after the builtin phase. In
//! those situations you need to write your own phase.

use std::ops::Range;

use bevy::{
    ecs::{
        entity::EntityHashSet,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::FloatOrd,
    pbr::{DrawMesh, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    mesh::{MeshVertexBufferLayoutRef, RenderMesh},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::{
        sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
        SetItemPipeline, SortedPhaseItem, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout, BindingResources,
        CachedRenderPipelineId, PipelineCache, RenderPipelineDescriptor, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    renderer::RenderDevice,
    sync_world::{MainEntity, RenderEntity},
    view::{check_visibility, ExtractedView, RenderVisibleEntities, VisibilitySystems},
    Extract, Render, RenderApp, RenderSet,
};

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
        Transform::from_xyz(0.0, 0.5, 0.0),
        CustomDrawDataHandle(custom_draw.add(CustomDrawData {
            // Set it to red
            color: Vec4::new(1.0, 0.0, 0.0, 1.0),
        })),
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
    ));
}

#[derive(Component, ExtractComponent, Clone, Copy)]
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
                    queue_custom_meshes.in_set(RenderSet::QueueMeshes),
                ),
            );
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
}
impl FromWorld for CustomDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        Self {
            layout: CustomDrawData::bind_group_layout(world.resource::<RenderDevice>()),
        }
    }
}
impl SpecializedMeshPipeline for CustomDrawPipeline {
    type Key = u32;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        todo!()
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
        let Some(material) = handle.and_then(|handle| assets.into_inner().get(&handle.0)) else {
            return RenderCommandResult::Failure("invalid item query");
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

// TODO ViewNode

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
    for (view_entity, view, visible_entities, _msaa) in &mut views {
        let Some(custom_phase) = custom_render_phases.get_mut(&view_entity) else {
            continue;
        };
        let draw_custom = custom_draw_functions.read().id::<DrawCustom>();

        let rangefinder = view.rangefinder3d();
        for (render_entity, visible_entity) in visible_entities.iter::<WithCustomDraw>() {
            println!("queue entities");
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &custom_draw_pipeline,
                0, // TODO key
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
