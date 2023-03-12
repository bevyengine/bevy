//! A shader and a material that uses it.

use bevy::{
    prelude::*,
    reflect::TypeUuid,
    sprite::{ AddonMeta, MaterialMesh2dBundle, Material2d, Material2dPlugin, MaterialAddon },
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin,},
        render_resource::{AsBindGroup, BindGroupLayoutEntry, BufferDescriptor, BufferSize,  ShaderRef, BufferUsages, },
        renderer::RenderQueue,
        RenderApp,
    }
};
use bevy_internal::render::render_resource::{BindingType, BufferBindingType, ShaderStages};
use bevy_internal::render::RenderSet;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<CustomMaterial>::default())
        .add_startup_system(setup)
        .run();
}

/// set up a simple 2D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
        transform: Transform::default().with_scale(Vec3::splat(128.)),
        material: materials.add(CustomMaterial {
            color: Color::BLUE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Blend,
        }),
        ..default()
    });
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material_with_addon.wgsl".into()
    }

    type Addons = (CursorAddon, ());
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "c595e853-ddbc-446e-a0a6-54d62671a5fe"]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Color,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

#[derive(Clone)]
pub struct CursorAddon;

#[derive(Default, Resource)]
struct ExtractedCursor {
    x: f32,
    y: f32,
}

const MEMORY_SIZE: u64 = std::mem::size_of::<ExtractedCursor>() as u64;

impl MaterialAddon for CursorAddon {
    fn create_bind_group_layout_entry(_asset_server: &AssetServer, binding: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(MEMORY_SIZE),
            },
            count: None,
        }
    }

    fn build_addon<G: Send + Sync + 'static, const BINDING: usize>(app: &mut App) {

        println!("CursorAddon.build {BINDING}");

        if !app.is_plugin_added::<ExtractResourcePlugin::<ExtractedCursor>>() {
            app.add_plugin(ExtractResourcePlugin::<ExtractedCursor>::default());
        }
        app.sub_app_mut(RenderApp)
            .add_system(prepare::<G, BINDING>.in_set(RenderSet::Prepare));
    }

    fn create_buffer_descriptor<'a>() -> BufferDescriptor<'a> {
        BufferDescriptor {
            label: Some("cursor uniform buffer"),
            size: MEMORY_SIZE,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }
    }
}

impl ExtractResource for ExtractedCursor {
    type Source = Input<MouseButton>;
    fn extract_resource(src: &Self::Source) -> Self {
        ExtractedCursor {
            x: if src.pressed(MouseButton::Left) { 1.0 } else { 0.0 },
            y: if src.pressed(MouseButton::Right) { 1.0 } else { 0.0 },
        }
    }
}

fn prepare<G: Send + Sync + 'static, const BINDING: usize>(
    extracted: Res<ExtractedCursor>,
    meta: Res<AddonMeta<G>>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &meta.buffers[BINDING],
        0,
        bevy::core::cast_slice(&[extracted.x, extracted.y])
    );
}
