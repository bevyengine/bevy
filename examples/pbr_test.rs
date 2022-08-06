use bevy::{prelude::*, render::{render_resource::PipelineCache, RenderApp, render_phase::{RenderPhase, CachedRenderPipelinePhaseItem}, RenderStage}, core_pipeline::core_3d::{Opaque3d, AlphaMask3d}, pbr::queue_material_meshes};
use naga::{valid::{ValidationFlags, Capabilities}, back::wgsl::WriterFlags};
use naga_oil::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    app.add_startup_system(setup);

    let render_app = &mut app.sub_app_mut(RenderApp);
    render_app.add_system_to_stage(RenderStage::Queue, get_shader::<AlphaMask3d>.after(queue_material_meshes::<StandardMaterial>));

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(PbrBundle{
        mesh: meshes.add(shape::Quad::default().into()),
        material: mats.add(StandardMaterial { alpha_mode: AlphaMode::Mask(0.5), base_color_texture: Some(asset_server.load("img.png")), ..Default::default() }),
        ..Default::default()
    });

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn get_shader<T: CachedRenderPipelinePhaseItem>(
    views: Query<&RenderPhase<T>>,
    pipeline_cache: Res<PipelineCache>,
    mut done: Local<bool>,
) {
    if !*done {
        println!("checking");
        for phase in views.iter() {
            for phase_item in phase.items.iter() {
                let id = phase_item.cached_pipeline();
                if let (Ok(vertex_shader), Ok(Some(fragment_shader))) = (pipeline_cache.get_processed_vertex_shader(id), pipeline_cache.get_processed_fragment_shader(id)) {
                    *done = true;
                    println!("gotem");
                    let vertex_module = naga::front::wgsl::parse_str(vertex_shader.get_wgsl_source().unwrap()).unwrap();
                    let fragment_module = naga::front::wgsl::parse_str(fragment_shader.get_wgsl_source().unwrap()).unwrap();
                    println!("{:#?}", fragment_module);

                    let frag_entrypoint = fragment_module.entry_points.iter().find(|ep| ep.name.as_str() == "fragment").unwrap();
                    let vertex_entrypoint = vertex_module.entry_points.iter().find(|ep| ep.name.as_str() == "vertex").unwrap();

                    let mut frag_req = ModuleRequires::default();
                    let frag_inputs = frag_req.add_entrypoint(&fragment_module, frag_entrypoint, Default::default(), None);
                    println!("{:#?}", frag_inputs);

                    let rewritten_shader = frag_req.rewrite(&fragment_module);
                    println!("{:#?}", rewritten_shader);

                    let info = naga::valid::Validator::new(ValidationFlags::all(), Capabilities::default()).validate(&rewritten_shader).unwrap();
                    let text = naga::back::wgsl::write_string(&rewritten_shader, &info, WriterFlags::EXPLICIT_TYPES).unwrap();
                    println!("rewritten frag wgsl: {}", text);
            
                } else {
                    println!("nope");
                }
            }
        }
    }
}