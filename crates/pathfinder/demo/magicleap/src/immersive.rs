#![allow(unused_imports)]
#![allow(dead_code)]

use crate::display::Display;
use crate::display::DisplayCamera;

use log::debug;

use pathfinder_demo::Options;
use pathfinder_demo::BuildOptions;
use pathfinder_demo::SceneThreadProxy;
use pathfinder_demo::MainToSceneMsg;
use pathfinder_demo::SceneToMainMsg;
use pathfinder_demo::Camera;
use pathfinder_demo::CameraTransform3D;
use pathfinder_gl::GLDevice;
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_geometry::point::Point2DI32;
use pathfinder_geometry::point::Point2DF32;
use pathfinder_geometry::point::Point3DF32;
use pathfinder_geometry::rect::RectI32;
use pathfinder_geometry::transform2d::Transform2F32;
use pathfinder_geometry::transform3d::Transform4F32;
use pathfinder_geometry::transform3d::Perspective;
use pathfinder_gpu::Device;
use pathfinder_simd::default::F32x4;
use pathfinder_svg::BuiltSVG;
use pathfinder_renderer::scene::Scene;
use pathfinder_renderer::builder::RenderTransform;

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use usvg;

pub struct ImmersiveDemo<D> {
    display: D,
    renderer: Renderer<GLDevice>,
    scene_thread_proxy: SceneThreadProxy,
    svg_size: Point2DF32,
    svg_to_world: Option<Transform4F32>,
}

static DEFAULT_SVG_VIRTUAL_PATH: &'static str = "svg/Ghostscript_Tiger.svg";

// SVG dimensions in metres
const MAX_SVG_HEIGHT: f32 = 1.0;
const MAX_SVG_WIDTH: f32 = 1.0;
const DEFAULT_SVG_DISTANCE: f32 = 1.5;

impl<D: Display> ImmersiveDemo<D> {
    pub fn new(mut display: D) -> Result<Self, D::Error> {
        display.make_current()?;
        let resources = display.resource_loader();
        let options = Options::get();
        let svg_data = resources.slurp(DEFAULT_SVG_VIRTUAL_PATH)?;
        let tree = usvg::Tree::from_data(&svg_data[..], &usvg::Options::default())?;
        let svg = BuiltSVG::from_tree(tree);
	let svg_size = svg.scene.view_box.size();
        let scene_thread_proxy = SceneThreadProxy::new(svg.scene, options);
        let _ = scene_thread_proxy.sender.send(MainToSceneMsg::SetDrawableSize(display.size()));
        let device = GLDevice::new(display.gl_version());
	let viewport = RectI32::new(Point2DI32::new(0, 0), display.size());
        let renderer = Renderer::new(device, resources, viewport, display.size());
        Ok(ImmersiveDemo {
            display,
            renderer,
            scene_thread_proxy,
	    svg_size,
            svg_to_world: None,
        })
    }

    pub fn running(&self) -> bool {
        self.display.running()
    }

    pub fn render_scene(&mut self) -> Result<(), D::Error> {
        self.display.make_current()?;
	let cameras = self.display.begin_frame()?;

        debug!("PF rendering a frame");
        let start = Instant::now();

        let svg_size = self.svg_size;
        let svg_to_world = self.svg_to_world.get_or_insert_with(|| {
	    let view: Transform4F32 = cameras[0].view();
            let svg_to_world_scale = f32::max(MAX_SVG_WIDTH / svg_size.x(), MAX_SVG_HEIGHT / svg_size.y());
	    let svg_width = svg_size.x() * svg_to_world_scale;
	    let svg_height = svg_size.y() * svg_to_world_scale;
            Transform4F32::from_uniform_scale(svg_to_world_scale)
                .pre_mul(&Transform4F32::from_translation(-svg_width / 2.0, -svg_height / 2.0, -DEFAULT_SVG_DISTANCE))
                .pre_mul(&Transform4F32::from_scale(1.0, -1.0, 1.0))
                .pre_mul(&view.inverse())
	});

        let render_transforms = cameras.iter()
	    .map(|camera| RenderTransform::Perspective(
	        camera.perspective()
	        .post_mul(&camera.view())
	        .post_mul(&svg_to_world)
	    )).collect();
        let msg = MainToSceneMsg::Build(BuildOptions {
            render_transforms: render_transforms,
            stem_darkening_font_size: None,
        });
        let _ = self.scene_thread_proxy.sender.send(msg);

        if let Ok(reply) = self.scene_thread_proxy.receiver.recv() {
            for (camera, scene) in cameras.iter_mut().zip(reply.render_scenes) {
                debug!("PF rendering eye after {}ms", (Instant::now() - start).as_millis());
                camera.make_current()?;
  	        let bounds = camera.bounds();
                let background = F32x4::new(0.0, 0.0, 0.0, 1.0);
                self.renderer.device.clear(Some(background), Some(1.0), Some(0));
                self.renderer.enable_depth();
                self.renderer.set_viewport(bounds);
                self.renderer.render_scene(&scene.built_scene);
            }
	}

	debug!("PF rendered frame after {}ms", (Instant::now() - start).as_millis());
        self.display.end_frame()?;
        Ok(())
    }
}
