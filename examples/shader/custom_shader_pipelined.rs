use bevy::{
    core_pipeline::{SetItemPipeline, Transparent3d},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    math::{Vec3, Vec4},
    pbr2::{
        DrawMesh, MeshUniform, PbrPipeline, PbrPipelineKey, SetMeshViewBindGroup,
        SetTransformBindGroup,
    },
    prelude::{AddAsset, App, AssetServer, Assets, GlobalTransform, Handle, Plugin, Transform},
    reflect::TypeUuid,
    render2::{
        camera::PerspectiveCameraBundle,
        color::Color,
        mesh::{shape, Mesh},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase, TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::ExtractedView,
        RenderApp, RenderStage,
    },
    PipelinedDefaultPlugins,
};
use crevice::std140::{AsStd140, Std140};

fn main() {
    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(CustomMaterialPlugin)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // cube
    commands.spawn().insert_bundle((
        meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        GlobalTransform::default(),
        materials.add(CustomMaterial {
            color: Color::GREEN,
        }),
    ));

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4ee9c363-1124-4113-890e-199d81b00281"]
pub struct CustomMaterial {
    color: Color,
}

#[derive(Clone)]
pub struct GpuCustomMaterial {
    _buffer: Buffer,
    bind_group: BindGroup,
}

impl RenderAsset for CustomMaterial {
    type ExtractedAsset = CustomMaterial;
    type PreparedAsset = GpuCustomMaterial;
    type Param = (SRes<RenderDevice>, SRes<CustomPipeline>);
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, custom_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let color: Vec4 = extracted_asset.color.as_rgba_linear().into();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: color.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &custom_pipeline.material_layout,
        });

        Ok(GpuCustomMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}
pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<CustomMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<CustomMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<CustomMaterial>::default());
        app.sub_app(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<CustomPipeline>()
            .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}

pub struct CustomPipeline {
    material_layout: BindGroupLayout,
    pipeline: CachedPipelineId,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("shaders/custom.wgsl");

        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(Vec4::std140_size_static() as u64),
                },
                count: None,
            }],
            label: None,
        });

        let pbr_pipeline = world.get_resource::<PbrPipeline>().unwrap();
        let mut descriptor = pbr_pipeline.specialize(PbrPipelineKey::empty());
        descriptor.vertex.shader = shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = shader;
        descriptor.layout = Some(vec![
            pbr_pipeline.view_layout.clone(),
            material_layout.clone(),
            pbr_pipeline.mesh_layout.clone(),
        ]);

        let mut pipeline_cache = world.get_resource_mut::<RenderPipelineCache>().unwrap();
        CustomPipeline {
            pipeline: pipeline_cache.queue(descriptor),
            material_layout,
        }
    }
}

pub fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    materials: Res<RenderAssets<CustomMaterial>>,
    custom_pipeline: Res<CustomPipeline>,
    material_meshes: Query<(Entity, &Handle<CustomMaterial>, &MeshUniform), With<Handle<Mesh>>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();
    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, material_handle, mesh_uniform) in material_meshes.iter() {
            if materials.contains_key(material_handle) {
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline: custom_pipeline.pipeline,
                    draw_function: draw_custom,
                    distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetCustomMaterialBindGroup,
    SetTransformBindGroup<2>,
    DrawMesh,
);

struct SetCustomMaterialBindGroup;
impl RenderCommand<Transparent3d> for SetCustomMaterialBindGroup {
    type Param = (
        SRes<RenderAssets<CustomMaterial>>,
        SQuery<Read<Handle<CustomMaterial>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: &Transparent3d,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let material_handle = query.get(item.entity).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(1, &material.bind_group, &[]);
    }
}
