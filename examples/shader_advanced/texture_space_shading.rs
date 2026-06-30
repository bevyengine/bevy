//! This example shows how render a texture to be used on a mesh in a technique called "texture space shading."
//! One reason this could be used is for expensive computations that can be reused (either spatially or temporally) with different views.
//!
//! This is a fairly low level example and assumes some familiarity with rendering concepts and wgpu.

use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{schedule::Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_asset::RenderAssets,
        render_resource::{binding_types::uniform_buffer, *},
        renderer::RenderContext,
        texture::GpuImage,
        RenderApp, RenderStartup,
    },
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/texture_space_shading.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TextureSpaceShadingPlugin))
        .insert_resource(TextureUpdateTimer(Timer::from_seconds(
            0.1,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup)
        .add_systems(Update, (tick, rotate, update_settings))
        .run();
}

struct TextureSpaceShadingPlugin;

impl Plugin for TextureSpaceShadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // Extract TextureSpaceShadingSettings and GenerateTexture to the render world
            ExtractComponentPlugin::<TextureSpaceShadingSettings>::default(),
            ExtractComponentPlugin::<GenerateTexture>::default(),
            // Also make TextureSpaceShadingSettings available to the shader
            UniformComponentPlugin::<TextureSpaceShadingSettings>::default(),
        ));

        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_texture_space_shading_pipeline);
        render_app.add_systems(
            Core3d,
            texture_space_shading_system.in_set(Core3dSystems::Prepass),
        );
    }
}

fn texture_space_shading_system(
    mesh: Query<
        (
            &GenerateTexture,
            &DynamicUniformIndex<TextureSpaceShadingSettings>,
        ),
        Changed<TextureSpaceShadingSettings>,
    >,
    texture_space_shading_pipeline: Option<Res<TextureSpaceShadingPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    settings_uniforms: Res<ComponentUniforms<TextureSpaceShadingSettings>>,
    images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
) {
    let Some(texture_space_shading_pipeline) = texture_space_shading_pipeline else {
        return;
    };

    let Some(pipeline) =
        pipeline_cache.get_render_pipeline(texture_space_shading_pipeline.pipeline_id)
    else {
        return;
    };

    let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
        return;
    };

    let bind_group = ctx.render_device().create_bind_group(
        "texture_space_shading_bind_group",
        &pipeline_cache.get_bind_group_layout(&texture_space_shading_pipeline.layout),
        // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
        &BindGroupEntries::sequential((
            // Set the settings binding
            settings_binding.clone(),
        )),
    );

    // Run a pass per entity
    for (generate_texture, settings_index) in mesh {
        let Some(image) = images.get(&generate_texture.texture) else {
            continue;
        };

        let mut render_pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("texture_space_shading_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    // Write to the image specified in the GenerateTexture component
                    view: &image.texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

        render_pass.set_pipeline(pipeline);
        // The settings_index selects the TextureSpaceShadingSettings uniform for this entity.
        render_pass.set_bind_group(0, &*bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);
    }
}

// This contains global data used by the render pipeline. This will be created once on startup.
#[derive(Resource)]
struct TextureSpaceShadingPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline_id: CachedRenderPipelineId,
}

fn init_texture_space_shading_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    // We need to define the bind group layout used for our pipeline
    let layout = BindGroupLayoutDescriptor::new(
        "texture_space_shading_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            // The layout entries will only be visible in the fragment stage
            ShaderStages::FRAGMENT,
            (
                // The settings uniform that will control the effect
                uniform_buffer::<TextureSpaceShadingSettings>(true),
            ),
        ),
    );

    // Get the shader handle
    let shader = asset_server.load(SHADER_ASSET_PATH);
    // This will setup a fullscreen triangle for the vertex state.
    let vertex_state = fullscreen_shader.to_vertex_state();
    let pipeline_id = pipeline_cache
        // This will add the pipeline to the cache and queue its creation
        .queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("texture_space_shading_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: vertex_state,
            fragment: Some(FragmentState {
                shader,
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
    commands.insert_resource(TextureSpaceShadingPipeline {
        layout,
        pipeline_id,
    });
}

// This is the component that will get passed to the shader
#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
struct TextureSpaceShadingSettings {
    t: f32,
    // WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl2")]
    _webgl2_padding: Vec3,
}

// This stores the texture that will be rendered
#[derive(Component, Default, Clone, ExtractComponent)]
struct GenerateTexture {
    texture: Handle<Image>,
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::default(), Vec3::Y),
        Camera {
            clear_color: Color::srgb(0.05, 0.05, 0.05).into(),
            ..default()
        },
    ));

    // cube
    let texture_size = Extent3d {
        width: 1024,
        height: 1024,
        depth_or_array_layers: 1,
    };

    // a blank image
    let mut image = Image::new_fill(
        texture_size,
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Need to set this flag to allow rendering to this texture
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    let texture = images.add(image);

    // A simple atlas for a cube
    // 0      1/3    2/3       1
    //     .-------+------+--------.
    //     | front | back | right  |
    // 1/3 +-------+------+--------+
    //     | left  | top  | bottom |
    // 2/3 '-------+------+--------+
    let atlas = |n: usize| {
        let col: usize = n / 3;
        let row: usize = n % 3;
        Vec2::new(row as f32 / 3., col as f32 / 3.)
    };
    let mut cube_mesh = Mesh::from(Cuboid::default());
    // Rewrite UVs so they don't overlap
    let uvs: Vec<_> = match cube_mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) => uvs
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .flat_map(|(i, f)| {
                let offset = atlas(i);
                f.iter()
                    .map(move |[u, v]| [*u / 3. + offset.x, *v / 3. + offset.y])
            })
            .collect(),
        _ => panic!(),
    };
    cube_mesh = cube_mesh.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    commands.spawn((
        Mesh3d(meshes.add(cube_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(texture.clone()),
            ..Default::default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Rotates,
        // Add the setting to the entity that controls the animation
        TextureSpaceShadingSettings {
            t: 0.5,
            ..default()
        },
        // Pass the texture to be rendered
        GenerateTexture { texture },
    ));
    // light
    commands.spawn(DirectionalLight {
        illuminance: 1_000.,
        ..default()
    });
}

#[derive(Resource, Deref, DerefMut)]
struct TextureUpdateTimer(Timer);

fn tick(time: Res<Time>, mut timer: ResMut<TextureUpdateTimer>) {
    timer.tick(time.delta());
}

#[derive(Component)]
struct Rotates;

/// Rotates any entity around the x and y axis
fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in &mut query {
        transform.rotate_x(0.55 * time.delta_secs());
        transform.rotate_z(0.15 * time.delta_secs());
    }
}

/// Change the setting over time to show that the effect is controlled from the main world
fn update_settings(
    mut settings: Query<&mut TextureSpaceShadingSettings>,
    time: Res<Time>,
    timer: Res<TextureUpdateTimer>,
) {
    // Update at a limited rate to show that the texture doesn't need to be updated every frame.
    if timer.just_finished() {
        for mut setting in &mut settings {
            let mut t = ops::sin(time.elapsed_secs());
            // Make it loop periodically
            t = ops::sin(t);
            // Remap it to 0..1 because the intensity can't be negative
            t = t * 0.5 + 0.5;

            // Set the intensity.
            // This will then be extracted to the render world and uploaded to the GPU automatically by the [`UniformComponentPlugin`]
            setting.t = t;
        }
    }
}
