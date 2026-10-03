//! Headless GPU regression for meshes whose bindless material is relocated after
//! waiting for asynchronous textures. Run explicitly on a bindless-capable GPU.
#![recursion_limit = "256"]
use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    pbr::{MeshInputUniform, MeshUniform, PbrPlugin, PreparedMaterial, RenderMeshInstances},
    prelude::*,
    render::{
        batching::gpu_preprocessing::{
            BatchedInstanceBuffers, GpuPreprocessingMode, GpuPreprocessingSupport,
        },
        erased_render_asset::ErasedRenderAssets,
        gpu_readback::{Readback, ReadbackComplete},
        material_bind_groups::RenderMaterialBindings,
        render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderDevice,
        RenderApp, RenderPlugin,
    },
    window::ExitCondition,
};
const SIDE: u32 = 10;
const PX: u32 = 320;
const CELL: u32 = PX / SIDE;
const COLORS: [[u8; 3]; 4] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]];
#[derive(Resource, Default)]
struct Pixels(Vec<u8>);
fn tick(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn white() -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
fn snapshot(app: &App, e: Entity, m: &Handle<StandardMaterial>) -> (u32, u32, u32, bool) {
    let world = app.sub_app(RenderApp).world();
    let binding = world
        .resource::<RenderMaterialBindings>()
        .get(&m.id().untyped())
        .unwrap();
    let instances = world.resource::<RenderMeshInstances>();
    let mesh = instances.render_mesh_queue_data(e.into()).unwrap();
    let slot = match world.get_resource::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>() {
        Some(buffers) => {
            buffers
                .current_input_buffer
                .get_unchecked(mesh.current_uniform_index.0)
                .material_and_lightmap_bind_group_slot
                & 0xffff
        }
        None => mesh.material_bindings_index().slot.0,
    };
    (
        binding.group.0,
        binding.slot.0,
        slot,
        world
            .resource::<ErasedRenderAssets<PreparedMaterial>>()
            .get(m.id())
            .is_some(),
    )
}
#[test]
#[ignore = "requires a GPU with bindless material support"]
fn delayed_material_preparation_updates_mesh_binding() {
    let output = std::env::var_os("BEVY_25595_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("bevy-25595-material"));
    let cpu = std::env::var("BEVY_25595_MODE").as_deref() == Ok("cpu");
    let no_bindless = std::env::var("BEVY_25595_MODE").as_deref() == Ok("no-bindless");
    let settings = bevy::render::settings::WgpuSettings {
        disabled_features: no_bindless
            .then_some(bevy::render::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY),
        ..default()
    };
    let mut app = App::new();
    app.init_resource::<Pixels>()
        .insert_resource(ClearColor(Color::BLACK));
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(PbrPlugin {
                use_gpu_instance_buffer_builder: !cpu,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: settings.into(),
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    );
    app.finish();
    app.cleanup();
    if cpu {
        // RenderStartup initializes capabilities on the first update. Keep
        // phase classification consistent with the CPU-only mesh builder.
        tick(&mut app, 1);
        app.sub_app_mut(RenderApp)
            .world_mut()
            .resource_mut::<GpuPreprocessingSupport>()
            .max_supported_mode = GpuPreprocessingMode::None;
    }
    let device = app.sub_app(RenderApp).world().resource::<RenderDevice>();
    assert!(
        no_bindless || StandardMaterial::bindless_supported(device),
        "this regression needs a GPU with bindless StandardMaterial support"
    );
    println!(
        "bindless={} cap={}",
        StandardMaterial::bindless_supported(device),
        StandardMaterial::bindless_slot_count().unwrap().resolve()
    );
    let mut target = Image::new_target_texture(PX, PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(target);
    app.world_mut().spawn((
        Camera3d::default(),
        RenderTarget::Image(target.clone().into()),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: SIDE as f32,
                height: SIDE as f32,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0., 0., 10.),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
    ));
    app.world_mut().spawn(Readback::texture(target)).observe(
        |event: On<ReadbackComplete>, mut p: ResMut<Pixels>| {
            p.0 = event.data.clone();
        },
    );
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(0.8, 0.8));
    let mut records = Vec::new();
    for i in 0..SIDE * SIDE {
        let texture = app.world_mut().resource_mut::<Assets<Image>>().add(white());
        let c = COLORS[i as usize % 4];
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb_u8(c[0], c[1], c[2]),
                base_color_texture: Some(texture),
                unlit: true,
                ..default()
            });
        let e = app
            .world_mut()
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(
                    (i % SIDE) as f32 + 0.5 - SIDE as f32 / 2.,
                    (i / SIDE) as f32 + 0.5 - SIDE as f32 / 2.,
                    0.,
                ),
            ))
            .id();
        records.push((e, material));
    }
    tick(&mut app, 150);
    // Unique textures fill the first binding array. Its last material must move
    // to another array when it grows from one texture to five textures.
    let (target_index, _) = records
        .iter()
        .enumerate()
        .filter_map(|(i, (e, m))| {
            let s = snapshot(&app, *e, m);
            (s.0 == 0).then_some((i, s.1))
        })
        .max_by_key(|x| x.1)
        .unwrap();
    let (entity, material) = records[target_index].clone();
    let before = snapshot(&app, entity, &material);
    println!("TARGET index={target_index} before={before:?}");
    assert!(before.3, "initial material was not prepared");
    assert_eq!(before.1, before.2, "initial mesh binding was incorrect");
    let before_pixels = app.world().resource::<Pixels>().0.clone();
    assert_eq!(before_pixels.len(), (PX * PX * 4) as usize);
    image::save_buffer(
        format!("{}-before.png", output.display()),
        &before_pixels,
        PX,
        PX,
        image::ColorType::Rgba8,
    )
    .unwrap();
    // Retain valid handles but withhold their image data for several frames.
    // This exercises PrepareAssetError::RetryNextUpdate after an already-ready
    // material is modified, rather than only initial material loading.
    let pending: Vec<_> = (0..5)
        .map(|_| app.world().resource::<Assets<Image>>().reserve_handle())
        .collect();
    {
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        let mut m = materials.get_mut(&material).unwrap();
        m.base_color_texture = Some(pending[0].clone());
        m.normal_map_texture = Some(pending[1].clone());
        m.metallic_roughness_texture = Some(pending[2].clone());
        m.occlusion_texture = Some(pending[3].clone());
        m.emissive_texture = Some(pending[4].clone());
    }
    tick(&mut app, 30);
    let pending_state = snapshot(&app, entity, &material);
    println!("PENDING {pending_state:?}");
    assert!(
        !pending_state.3,
        "test did not exercise material preparation retry"
    );
    let removal = std::env::var("BEVY_25595_REMOVE").unwrap_or_default();
    match removal.as_str() {
        "despawn" => {
            app.world_mut().despawn(entity);
        }
        "mesh" | "reinsert" => {
            app.world_mut().entity_mut(entity).remove::<Mesh3d>();
            if removal == "reinsert" {
                app.world_mut().entity_mut(entity).insert(Mesh3d(mesh));
            }
        }
        "" => {}
        _ => panic!("BEVY_25595_REMOVE must be despawn, mesh, or reinsert"),
    }
    if matches!(removal.as_str(), "despawn" | "mesh") {
        tick(&mut app, 3);
        let retained = app
            .sub_app(RenderApp)
            .world()
            .resource::<RenderMeshInstances>()
            .render_mesh_queue_data(entity.into())
            .is_some();
        println!("REMOVED mode={removal} retained={retained}");
        assert!(!retained, "removed pending mesh retained its GPU instance");
        return;
    }
    for h in pending {
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(h.id(), white())
            .unwrap();
    }
    tick(&mut app, 150);
    let after = snapshot(&app, entity, &material);
    let px = &app.world().resource::<Pixels>().0;
    let x = target_index as u32 % SIDE;
    let y = target_index as u32 / SIDE;
    let offset = (((SIDE - 1 - y) * CELL + CELL / 2) * PX + x * CELL + CELL / 2) as usize * 4;
    assert_eq!(
        &before_pixels[offset..offset + 3],
        &COLORS[target_index % 4],
        "initial target material color was incorrect"
    );
    let path = format!("{}-after.png", output.display());
    image::save_buffer(path, px, PX, PX, image::ColorType::Rgba8).unwrap();
    println!(
        "AFTER cpu={cpu} no_bindless={no_bindless} {after:?} expected_color={:?} actual_color={:?}",
        COLORS[target_index % 4],
        &px[offset..offset + 3]
    );
    assert!(after.3, "material never prepared");
    if !no_bindless {
        assert_ne!(
            before.0, after.0,
            "test did not force a bindless group change"
        );
        assert_ne!(
            before.1, after.1,
            "test did not force a bindless slot change"
        );
    }
    assert_eq!(
        after.1, after.2,
        "material reallocation after async texture retry left a stale mesh input slot"
    );
    assert_eq!(
        &px[offset..offset + 3],
        &COLORS[target_index % 4],
        "rendered material color changed after loading white textures"
    );
    assert!(
        px == &before_pixels,
        "loading white textures changed the rendered grid"
    );
}
