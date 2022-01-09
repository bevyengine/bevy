use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            std140::{AsStd140, Std140},
            *,
        },
        renderer::RenderDevice,
    },
    sprite::Material2d,
    sprite::{Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle, Mesh2dHandle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<Custom2dMaterial>::default())
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Custom2dMaterial>>,
) {
    let size = Vec2::new(300.0, 300.0);

    // quad
    let mesh_handle = Mesh2dHandle(meshes.add(Mesh::from(shape::Quad::new(size))));

    let material = materials.add(Custom2dMaterial {
        color: Vec4::new(1.0, 0.0, 0.0, 1.0),
        size,
        transparencies: [0.5, 0.2, 0.3, 0.5, 0.7],
        positions: [0.5, 0.75, 0.875, 0.9375, 0.96875],
    });

    commands.spawn().insert_bundle(MaterialMesh2dBundle {
        mesh: mesh_handle.clone(),
        material,
        ..Default::default()
    });

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

#[derive(Debug, Clone, TypeUuid, AsStd140)]
#[uuid = "4ee9c363-1124-4113-890e-199d81b00281"]
pub struct Custom2dMaterial {
    pub color: Vec4,
    pub size: Vec2,
    pub transparencies: [f32; 5],
    pub positions: [f32; 5],
}

#[derive(Clone)]
pub struct GpuCustom2dMaterial {
    _buffer: Buffer,
    bind_group: BindGroup,
}

impl Material2d for Custom2dMaterial {
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        Some(asset_server.load("shaders/custom_2d_material.wgsl"))
    }

    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(Custom2dMaterial::std140_size_static() as u64),
                },
                count: None,
            }],
            label: None,
        })
    }
}

impl RenderAsset for Custom2dMaterial {
    type ExtractedAsset = Custom2dMaterial;
    type PreparedAsset = GpuCustom2dMaterial;
    type Param = (SRes<RenderDevice>, SRes<Material2dPipeline<Self>>);
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, material_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let custom_material_std140 = extracted_asset.as_std140();
        let custom_material_bytes = custom_material_std140.as_bytes();

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: custom_material_bytes,
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &material_pipeline.material2d_layout,
        });

        Ok(GpuCustom2dMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}
