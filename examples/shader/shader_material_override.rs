//! A shader and a material that uses it.

use bevy::{
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
    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.5,
            subdivisions: 5,
        })),
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
        .spawn_bundle(PointLightBundle {
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
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-1.0, 1.25, 2.5).looking_at(Vec3::Y * 0.25, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct Move;

fn move_light(mut q: Query<&mut Transform, With<Move>>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        t.translation = Vec3::new(
            time.seconds_since_startup().sin() as f32 * 1.75,
            1.5,
            time.seconds_since_startup().cos() as f32 * 1.75,
        );
    }
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material_override.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.inner.alpha_mode()
    }
}

// This is the struct that will be passed to your shader
#[derive(TypeUuid, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e4a3f056e0"]
pub struct CustomMaterial {
    inner: StandardMaterial,
}

impl AsBindGroup for CustomMaterial {
    type Data = <StandardMaterial as AsBindGroup>::Data;
    fn as_bind_group(
        &self,
        a: &BindGroupLayout,
        b: &RenderDevice,
        c: &bevy::render::render_asset::RenderAssets<bevy::prelude::Image>,
        d: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
        let inner_prepared = self.inner.as_bind_group(a, b, c, d)?;
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
