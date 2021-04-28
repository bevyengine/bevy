use std::io::Write;

use bevy::{
    asset::AssetPlugin,
    core::CorePlugin,
    diagnostic::DiagnosticsPlugin,
    input::InputPlugin,
    log::LogPlugin,
    pbr::AmbientLight,
    prelude::*,
    render::{
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
            TextureAttachment,
        },
        pipeline::{
            BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, CompareFunction,
            DepthBiasState, DepthStencilState, PipelineDescriptor, StencilFaceState, StencilState,
        },
        render_graph::{
            base::{self, BaseRenderGraphConfig},
            fullscreen_pass_node, FullscreenPassNode, GlobalRenderResourcesNode, RenderGraph,
            WindowTextureNode,
        },
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
        texture::{
            Extent3d, SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsage,
        },
    },
    scene::ScenePlugin,
    window::{WindowId, WindowPlugin},
};

mod node {
    pub const POST_PASS: &str = "post_pass_node";
    pub const MAIN_COLOR_TEXTURE: &str = "main_color_texture_node";
}

#[derive(Debug, Clone, RenderResources)]
struct MyResource {
    value: f32,
}

fn main() {
    let mut app = App::build();

    app.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0 / 5.0f32,
    })
    .insert_resource(Msaa { samples: 4 })
    .insert_resource(MyResource { value: 1.0 });

    app.add_plugin(LogPlugin::default())
        .add_plugin(CorePlugin::default())
        .add_plugin(TransformPlugin::default())
        .add_plugin(DiagnosticsPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin::default())
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin::default());

    // cannot currently override config for a plugin as part of DefaultPlugins
    app.add_plugin(bevy::render::RenderPlugin {
        base_render_graph_config: Some(BaseRenderGraphConfig {
            add_2d_camera: true,
            add_3d_camera: true,
            add_main_depth_texture: true,
            add_main_pass: true,
            connect_main_pass_to_swapchain: false,
            connect_main_pass_to_main_depth_texture: true,
        }),
    });

    app.add_plugin(bevy::pbr::PbrPlugin::default());

    app.add_plugin(bevy::gltf::GltfPlugin::default());

    app.add_plugin(bevy::winit::WinitPlugin::default());

    app.add_plugin(bevy::wgpu::WgpuPlugin::default());

    app.add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut render_graph: ResMut<RenderGraph>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    msaa: Res<Msaa>,
) {
    setup_render_graph(&mut *render_graph, &mut *pipelines, &mut *shaders, &*msaa);

    render_graph.add_system_node(
        "my_resource_node",
        GlobalRenderResourcesNode::<MyResource>::new(),
    );
    render_graph
        .add_node_edge("my_resource_node", node::POST_PASS)
        .unwrap();

    commands.spawn_scene(asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"));
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..Default::default()
    });
    commands
        .spawn_bundle(PointLightBundle {
            transform: Transform::from_xyz(3.0, 5.0, 3.0),
            ..Default::default()
        })
        .insert(Rotates);
}

fn setup_render_graph(
    render_graph: &mut RenderGraph,
    pipelines: &mut Assets<PipelineDescriptor>,
    shaders: &mut Assets<Shader>,
    msaa: &Msaa,
) {
    // Rendergraph additions
    render_graph.add_node(
        node::MAIN_COLOR_TEXTURE,
        WindowTextureNode::new(
            WindowId::primary(),
            TextureDescriptor {
                size: Extent3d::new(1, 1, 1),
                mip_level_count: 1,
                sample_count: 1,
                dimension: bevy::render::texture::TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
            },
            Some(SamplerDescriptor::default()),
            None,
        ),
    );

    render_graph
        .add_slot_edge(
            node::MAIN_COLOR_TEXTURE,
            WindowTextureNode::OUT_TEXTURE,
            base::node::MAIN_PASS,
            if msaa.samples > 1 {
                "color_resolve_target"
            } else {
                "color_attachment"
            },
        )
        .unwrap();

    // Set up post processing pipeline
    let pipeline_descriptor = PipelineDescriptor {
        depth_stencil: None,
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendState {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendState {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                fullscreen_pass_node::shaders::VERTEX_SHADER,
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                "#version 450

                layout(location=0) in vec2 v_Uv;

                layout(set = 0, binding = 0) uniform texture2D color_texture;
                layout(set = 0, binding = 1) uniform sampler color_texture_sampler;

                layout(std140, set = 1, binding = 0) uniform MyResource_value {
                    float value;
                };

                layout(location=0) out vec4 o_Target;

                void main() {
                    o_Target = texture(sampler2D(color_texture, color_texture_sampler), v_Uv) * value;
                }
                ",
            ))),
        })
    };

    let pipeline_handle = pipelines.add(pipeline_descriptor);

    // Setup post processing pass
    let pass_descriptor = PassDescriptor {
        color_attachments: vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgb(0.1, 0.2, 0.3)),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
        sample_count: 1,
    };

    // Create the pass node
    let post_pass_node = FullscreenPassNode::new(
        pass_descriptor,
        pipeline_handle,
        vec!["color_texture".into()],
    );
    render_graph.add_node(node::POST_PASS, post_pass_node);

    // Run after main pass
    render_graph
        .add_node_edge(base::node::MAIN_PASS, node::POST_PASS)
        .unwrap();

    // Connect color_attachment
    render_graph
        .add_slot_edge(
            base::node::PRIMARY_SWAP_CHAIN,
            WindowTextureNode::OUT_TEXTURE,
            node::POST_PASS,
            "color_attachment",
        )
        .unwrap();

    // Connect extra texture and sampler input
    render_graph
        .add_slot_edge(
            node::MAIN_COLOR_TEXTURE,
            WindowTextureNode::OUT_TEXTURE,
            node::POST_PASS,
            "color_texture",
        )
        .unwrap();

    render_graph
        .add_slot_edge(
            node::MAIN_COLOR_TEXTURE,
            WindowTextureNode::OUT_SAMPLER,
            node::POST_PASS,
            "color_texture_sampler",
        )
        .unwrap();
}

/// this component indicates what entities should rotate
struct Rotates;

fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in query.iter_mut() {
        *transform = Transform::from_rotation(Quat::from_rotation_y(
            (4.0 * std::f32::consts::PI / 20.0) * time.delta_seconds(),
        )) * *transform;
    }
}
