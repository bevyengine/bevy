//! A shader that overrides a core pbr function for a material

use bevy::{
    pbr::{MaterialPipelineKey, StandardMaterialKey},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_resource::{
            AsBindGroup, AsBindGroupError, BindGroupLayout, PreparedBindGroup, ShaderRef,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_startup_system(setup)
        .add_system(move_light)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(
            Mesh::try_from(shape::Icosphere {
                radius: 0.5,
                subdivisions: 5,
            })
            .unwrap(),
        ),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(CustomMaterial {
            inner: StandardMaterial {
                base_color: Color::LIME_GREEN,
                reflectance: 0.2,
                perceptual_roughness: 0.5,
                ..Default::default()
            },
        }),
        ..default()
    });

    // light
    commands
        .spawn(PointLightBundle {
            point_light: PointLight {
                color: Color::WHITE,
                intensity: 300.0,
                ..Default::default()
            },
            transform: Transform::from_xyz(-1.0, 0.25, 1.0),
            ..Default::default()
        })
        .insert(Move);

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-1.0, 1.25, 2.5).looking_at(Vec3::Y * 0.25, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct Move;

fn move_light(mut q: Query<&mut Transform, With<Move>>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        t.translation = Vec3::new(
            time.elapsed_seconds().sin() * 1.75,
            1.5,
            time.elapsed_seconds().cos() * 1.75,
        );
    }
}

#[derive(TypeUuid, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e4a3f056e0"]
pub struct CustomMaterial {
    inner: StandardMaterial,
}

// todo this should be done with a configurable standard material extension once implemented
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material_override.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        StandardMaterial::vertex_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.inner.alpha_mode()
    }

    fn specialize(
        pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayout,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // build a MaterialPipeline<StandardMaterial> out of the MaterialPipeline<Self> we've been given
        let bevy::pbr::MaterialPipeline::<Self> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            ..
        } = pipeline.clone();

        let standard_pipeline = bevy::pbr::MaterialPipeline::<StandardMaterial> {
            mesh_pipeline,
            material_layout,
            vertex_shader,
            fragment_shader,
            marker: Default::default(),
        };

        let key = MaterialPipelineKey {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data,
        };

        // defer to StandardMaterial's specialize function
        StandardMaterial::specialize(&standard_pipeline, descriptor, layout, key)
    }

    fn depth_bias(&self) -> f32 {
        self.inner.depth_bias()
    }
}

impl AsBindGroup for CustomMaterial {
    type Data = <StandardMaterial as AsBindGroup>::Data;
    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &bevy::render::render_asset::RenderAssets<bevy::prelude::Image>,
        fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<StandardMaterialKey>, AsBindGroupError> {
        let inner_prepared =
            self.inner
                .as_bind_group(layout, render_device, images, fallback_image)?;
        Ok(PreparedBindGroup {
            bindings: inner_prepared.bindings,
            bind_group: inner_prepared.bind_group,
            data: inner_prepared.data,
        })
    }
    fn bind_group_layout(a: &RenderDevice) -> bevy::render::render_resource::BindGroupLayout {
        StandardMaterial::bind_group_layout(a)
    }
}
