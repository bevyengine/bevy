use crate::BevyPathfinderDevice;
use bevy_asset::AssetStorage;
use bevy_render::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    render_resource::ResourceInfo,
    renderer::RenderContext,
    shader::{FieldBindType, Shader},
};
use legion::prelude::{Resources, World};
use pathfinder_canvas::{vec2f, Canvas, CanvasFontContext, ColorF, Path2D, RectF, Vector2I};
use pathfinder_renderer::{
    concurrent::{rayon::RayonExecutor, scene_proxy::SceneProxy},
    gpu::{
        options::{DestFramebuffer, RendererOptions},
        renderer::Renderer,
    },
    options::BuildOptions,
};
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use std::borrow::Cow;

#[derive(Default)]
pub struct PathfinderNode;

impl PathfinderNode {
    pub const IN_COLOR_TEXTURE: &'static str = "color";
    pub const IN_DEPTH_STENCIL_TEXTURE: &'static str = "depth_stencil";
}

impl Node for PathfinderNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        static INPUT: &[ResourceSlotInfo] = &[
            ResourceSlotInfo {
                name: Cow::Borrowed(PathfinderNode::IN_COLOR_TEXTURE),
                resource_type: FieldBindType::Texture,
            },
            ResourceSlotInfo {
                name: Cow::Borrowed(PathfinderNode::IN_DEPTH_STENCIL_TEXTURE),
                resource_type: FieldBindType::Texture,
            },
        ];
        INPUT
    }
    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let mut shaders = resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let color_texture = input.get(PathfinderNode::IN_COLOR_TEXTURE).unwrap();
        let depth_stencil_texture = input.get(PathfinderNode::IN_DEPTH_STENCIL_TEXTURE).unwrap();
        let device = BevyPathfinderDevice::new(
            render_context,
            &mut shaders,
            color_texture,
            depth_stencil_texture,
        );
        let window_size = Vector2I::new(1280 as i32, 720 as i32);
        let mut renderer = Renderer::new(
            device,
            &EmbeddedResourceLoader::new(),
            DestFramebuffer::full_window(window_size),
            RendererOptions {
                background_color: Some(ColorF::white()),
                ..Default::default()
            },
        );

        // Make a canvas. We're going to draw a house.
        let mut canvas = Canvas::new(window_size.to_f32())
            .get_context_2d(CanvasFontContext::from_system_source());

        // Set line width.
        canvas.set_line_width(10.0);

        // Draw walls.
        canvas.stroke_rect(RectF::new(vec2f(75.0, 140.0), vec2f(150.0, 110.0)));

        // Draw door.
        canvas.fill_rect(RectF::new(vec2f(130.0, 190.0), vec2f(40.0, 60.0)));

        // Draw roof.
        let mut path = Path2D::new();
        path.move_to(vec2f(50.0, 140.0));
        path.line_to(vec2f(150.0, 60.0));
        path.line_to(vec2f(250.0, 140.0));
        path.close_path();
        canvas.stroke_path(path);

        // Render the canvas to screen.
        let scene = SceneProxy::from_scene(canvas.into_canvas().into_scene(), RayonExecutor);
        scene.build_and_render(&mut renderer, BuildOptions::default());
        // TODO: submit command buffers?
    }
}
