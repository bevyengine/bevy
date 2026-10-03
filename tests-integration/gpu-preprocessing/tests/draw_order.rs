//! A static transparent scene must retain back-to-front draw order on the GPU.
#![recursion_limit = "256"]

use bevy::{
    anti_alias::fxaa::Fxaa,
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode},
    core_pipeline::{
        core_3d::Transparent3d,
        prepass::DepthPrepass,
        tonemapping::{DebandDither, Tonemapping},
        Core3d,
    },
    mesh::Indices,
    pbr::{late_prepass_build_indirect_parameters, main_build_indirect_parameters, PbrPlugin},
    prelude::*,
    render::{
        batching::gpu_preprocessing::{
            GpuPreprocessingMode, GpuPreprocessingSupport, IndirectParametersBuffers,
            IndirectParametersIndexed, IndirectParametersNonIndexed,
        },
        gpu_readback::{Readback, ReadbackComplete},
        occlusion_culling::OcclusionCulling,
        render_resource::{
            AsBindGroup, Buffer, BufferDescriptor, BufferUsages, MapMode, PrimitiveTopology,
            TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice},
        view::NoIndirectDrawing,
        ExtractSchedule, RenderApp, RenderDebugFlags, RenderPlugin, RenderStartup,
    },
    window::ExitCondition,
};
use std::{
    any::TypeId,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

const SIDE: u32 = 8;
const CELL: u32 = 16;
const PIXELS: u32 = SIDE * CELL;
const WARMUP_READS: usize = 60;
const OCCLUSION_PERIOD: usize = 12;

#[derive(Resource, Debug)]
struct Config {
    layers: u32,
    frames: usize,
    mode: String,
    antialiasing: String,
    same_material: bool,
    non_indexed: bool,
    occlusion: bool,
    output: PathBuf,
}

impl Config {
    fn from_env() -> Self {
        let config = Self {
            layers: std::env::var("BEVY_25595_LAYERS")
                .map(|s| s.parse().expect("BEVY_25595_LAYERS must be an integer"))
                .unwrap_or(2),
            frames: std::env::var("BEVY_25595_FRAMES")
                .map(|s| s.parse().expect("BEVY_25595_FRAMES must be an integer"))
                .unwrap_or(600),
            mode: std::env::var("BEVY_25595_MODE").unwrap_or_else(|_| "gpu".into()),
            antialiasing: std::env::var("BEVY_25595_AA").unwrap_or_else(|_| "none".into()),
            same_material: std::env::var("BEVY_25595_SAME_MATERIAL").as_deref() == Ok("1"),
            non_indexed: std::env::var("BEVY_25595_NON_INDEXED").as_deref() == Ok("1"),
            occlusion: std::env::var("BEVY_25595_OCCLUSION").as_deref() == Ok("1"),
            output: std::env::var_os("BEVY_25595_OUTPUT")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::temp_dir().join("bevy-25595-transparency")),
        };
        assert!((2..=256).contains(&config.layers));
        assert!(config.frames >= 240);
        assert!(matches!(
            config.mode.as_str(),
            "gpu" | "cpu" | "direct" | "no-bindless" | "opaque"
        ));
        assert!(matches!(
            config.antialiasing.as_str(),
            "none" | "fxaa" | "msaa4"
        ));
        assert!(
            !config.occlusion || (config.mode == "gpu" && !config.same_material),
            "the occlusion scene requires GPU mode and two differently colored materials"
        );
        config
    }
}

#[derive(Resource, Default)]
struct Results {
    reads: usize,
    checked: usize,
    bad_frames: usize,
    bad_cells: usize,
}

#[derive(Component)]
struct Occluder;

#[derive(Resource, Clone, Default)]
struct LateCounts(Arc<Mutex<LateCountResults>>);

#[derive(Default)]
struct LateCountResults {
    reads: usize,
    checked: usize,
    late_frames: usize,
    zero_late_frames: usize,
    max_late_instances: u32,
}

#[derive(Resource, Default)]
struct PendingLateReadback(Option<(Buffer, usize)>);

#[test]
#[ignore = "requires a GPU with bindless material support"]
fn transparent_instances_preserve_draw_order() {
    let config = Config::from_env();
    println!("CONFIG {config:?}");
    let frames = config.frames;
    let cpu = config.mode == "cpu";
    let no_bindless = config.mode == "no-bindless";
    let occlusion = config.occlusion;
    let render_debug_flags = if occlusion {
        RenderDebugFlags::ALLOW_COPIES_FROM_INDIRECT_PARAMETERS
    } else {
        RenderDebugFlags::empty()
    };
    let settings = bevy::render::settings::WgpuSettings {
        disabled_features: no_bindless
            .then_some(bevy::render::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY),
        ..default()
    };
    let mut app = App::new();
    app.insert_resource(config)
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<Results>()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(PbrPlugin {
                    use_gpu_instance_buffer_builder: !cpu,
                    debug_flags: render_debug_flags,
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: settings.into(),
                    synchronous_pipeline_compilation: true,
                    debug_flags: render_debug_flags,
                    ..default()
                }),
        )
        .add_systems(Startup, setup);
    if occlusion {
        let late_counts = LateCounts::default();
        app.insert_resource(late_counts.clone())
            .add_systems(Update, move_occluders);
        app.sub_app_mut(RenderApp)
            .insert_resource(late_counts)
            .init_resource::<PendingLateReadback>()
            .add_systems(ExtractSchedule, read_late_indirect_parameters)
            .add_systems(
                Core3d,
                copy_late_indirect_parameters
                    .after(late_prepass_build_indirect_parameters)
                    .before(main_build_indirect_parameters),
            );
    }
    if cpu {
        // Both mesh building and render-phase classification must use CPU mode.
        // Initialize the override after RenderStartup creates capabilities.
        app.sub_app_mut(RenderApp).add_systems(
            RenderStartup,
            (|mut support: ResMut<GpuPreprocessingSupport>| {
                support.max_supported_mode = GpuPreprocessingMode::None;
            })
            .after(bevy::render::init_gpu_resource::<GpuPreprocessingSupport>),
        );
    }
    app.finish();
    let device = app.sub_app(RenderApp).world().resource::<RenderDevice>();
    assert_eq!(
        StandardMaterial::bindless_supported(device),
        !no_bindless,
        "this test needs bindless material support unless testing its disabled control"
    );
    if occlusion {
        assert!(app
            .sub_app(RenderApp)
            .world()
            .resource::<GpuPreprocessingSupport>()
            .is_culling_supported());
    }
    app.cleanup();
    for _ in 0..frames {
        app.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    let results = app.world().resource::<Results>();
    println!(
        "RESULT reads={} checked={} bad_frames={} bad_cells={}",
        results.reads, results.checked, results.bad_frames, results.bad_cells
    );
    assert!(
        results.checked >= 100,
        "insufficient completed GPU readbacks"
    );
    assert_eq!(
        results.bad_frames, 0,
        "GPU draws violated transparent depth order"
    );
    if occlusion {
        let counts = app.world().resource::<LateCounts>().0.lock().unwrap();
        println!(
            "OCCLUSION reads={} checked={} late_frames={} zero_late_frames={} max_late_instances={}",
            counts.reads,
            counts.checked,
            counts.late_frames,
            counts.zero_late_frames,
            counts.max_late_instances,
        );
        assert!(
            counts.checked >= 100,
            "insufficient late-parameter readbacks"
        );
        assert!(counts.late_frames > 0, "no transparent late draws observed");
        assert!(
            counts.zero_late_frames > 0,
            "transparent late draws never returned to zero"
        );
    }
}

fn setup(
    mut commands: Commands,
    config: Res<Config>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut target = Image::new_target_texture(PIXELS, PIXELS, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = images.add(target);
    let mut camera = commands.spawn((
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
        if config.antialiasing == "msaa4" {
            Msaa::Sample4
        } else {
            Msaa::Off
        },
        Tonemapping::None,
        DebandDither::Disabled,
    ));
    if config.mode == "direct" {
        camera.insert(NoIndirectDrawing);
    }
    if config.antialiasing == "fxaa" {
        camera.insert(Fxaa::default());
    }
    if config.occlusion {
        camera.insert((DepthPrepass, OcclusionCulling));
    }
    let mut mesh = if config.same_material {
        // Distinguish instances with vertex colors and transforms even when
        // they share exactly the same material and bind group.
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-0.4, -0.4, 0.],
                [0., -0.4, 0.],
                [0., 0.4, 0.],
                [-0.4, 0.4, 0.],
                [0., -0.4, 0.],
                [0.4, -0.4, 0.],
                [0.4, 0.4, 0.],
                [0., 0.4, 0.],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 8])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 8])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![
                [1., 0., 0., 1.],
                [1., 0., 0., 1.],
                [1., 0., 0., 1.],
                [1., 0., 0., 1.],
                [0., 1., 0., 1.],
                [0., 1., 0., 1.],
                [0., 1., 0., 1.],
                [0., 1., 0., 1.],
            ],
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]))
    } else {
        Mesh::from(Rectangle::new(0.8, 0.8))
    };
    if config.non_indexed {
        mesh.duplicate_vertices();
    }
    let mesh = meshes.add(mesh);
    let alpha_mode = if config.mode == "opaque" {
        AlphaMode::Opaque
    } else {
        AlphaMode::Blend
    };
    // Alpha one makes the expected pixel exact while still exercising the
    // sorted transparent phase: the nearest red layer must hide the green ones.
    let front = materials.add(StandardMaterial {
        base_color: if config.same_material {
            Color::WHITE
        } else {
            Color::srgb(1., 0., 0.)
        },
        alpha_mode,
        unlit: true,
        ..default()
    });
    let back = if config.same_material {
        front.clone()
    } else {
        materials.add(StandardMaterial {
            base_color: Color::srgb(0., 1., 0.),
            alpha_mode,
            unlit: true,
            ..default()
        })
    };
    for layer in 0..config.layers {
        for y in 0..SIDE {
            for x in 0..SIDE {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(if layer == config.layers - 1 {
                        front.clone()
                    } else {
                        back.clone()
                    }),
                    Transform::from_xyz(
                        x as f32 + 0.5 - SIDE as f32 / 2.,
                        y as f32 + 0.5 - SIDE as f32 / 2.,
                        layer as f32 * 0.01,
                    )
                    .with_rotation(
                        if config.same_material && layer != config.layers - 1 {
                            Quat::from_rotation_z(std::f32::consts::PI)
                        } else {
                            Quat::IDENTITY
                        },
                    ),
                ));
            }
        }
    }
    if config.occlusion {
        let mut occluder_mesh = Mesh::from(Rectangle::new(1., 1.));
        if config.non_indexed {
            occluder_mesh.duplicate_vertices();
        }
        let occluder_mesh = meshes.add(occluder_mesh);
        let occluder_material = materials.add(StandardMaterial {
            base_color: Color::srgb(1., 0., 0.),
            alpha_mode: AlphaMode::Opaque,
            unlit: true,
            ..default()
        });
        for y in 0..SIDE {
            for x in 0..SIDE {
                commands.spawn((
                    Occluder,
                    Mesh3d(occluder_mesh.clone()),
                    MeshMaterial3d(occluder_material.clone()),
                    Transform::from_xyz(
                        x as f32 + 0.5 - SIDE as f32 / 2.,
                        y as f32 + 0.5 - SIDE as f32 / 2.,
                        config.layers as f32 * 0.01 + 1.,
                    ),
                ));
            }
        }
    }
    commands
        .spawn(Readback::texture(target))
        .observe(check_frame);
}

fn move_occluders(
    mut occluders: Query<&mut Transform, With<Occluder>>,
    config: Res<Config>,
    mut frame: Local<usize>,
) {
    if frame.is_multiple_of(OCCLUSION_PERIOD) {
        // The transparent instances remain static. Moving only these opaque
        // rectangles exercises both occlusion and disocclusion every 12 frames.
        let z = if (*frame / OCCLUSION_PERIOD).is_multiple_of(2) {
            config.layers as f32 * 0.01 + 1.
        } else {
            // Hide the occluders behind the camera (z=10), so missing
            // transparent layers reveal black instead of an opaque red backdrop.
            11.
        };
        for mut transform in &mut occluders {
            transform.translation.z = z;
        }
    }
    *frame += 1;
}

fn copy_late_indirect_parameters(
    mut context: RenderContext,
    parameters: Res<IndirectParametersBuffers>,
    device: Res<RenderDevice>,
    mut pending: ResMut<PendingLateReadback>,
) {
    let Some(phase) = parameters.get(&TypeId::of::<Transparent3d>()) else {
        return;
    };
    let (source, count, stride) = if phase.indexed.batch_count() != 0 {
        assert_eq!(phase.non_indexed.batch_count(), 0);
        (
            phase.indexed.data_buffer(),
            phase.indexed.batch_count(),
            size_of::<IndirectParametersIndexed>(),
        )
    } else {
        (
            phase.non_indexed.data_buffer(),
            phase.non_indexed.batch_count(),
            size_of::<IndirectParametersNonIndexed>(),
        )
    };
    let Some(source) = source.filter(|_| count != 0) else {
        return;
    };
    // This scene has one view and one transparent mesh class. Sorted draws
    // have fixed command slots, so batch_count is the valid contiguous range;
    // buffer capacity could include stale commands and must not be counted.
    let size = (count * stride) as u64;
    let staging = device.create_buffer(&BufferDescriptor {
        label: Some("transparent late indirect parameters"),
        size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    context
        .command_encoder()
        .copy_buffer_to_buffer(source, 0, &staging, 0, size);
    assert!(pending.0.is_none());
    pending.0 = Some((staging, stride));
}

fn read_late_indirect_parameters(
    mut pending: ResMut<PendingLateReadback>,
    counts: Res<LateCounts>,
) {
    let Some((buffer, stride)) = pending.0.take() else {
        return;
    };
    let counts = counts.0.clone();
    // ExtractSchedule runs after the preceding frame was submitted. Mapping
    // here keeps pipelined rendering enabled and avoids mapping before copy.
    buffer
        .clone()
        .slice(..)
        .map_async(MapMode::Read, move |result| {
            result.expect("late indirect parameter readback failed");
            let data = buffer.slice(..).get_mapped_range().unwrap();
            // Both wgpu indirect command layouts have instance_count at byte 4.
            // The snapshot is after the late builder and before the main builder,
            // so these counts are late-only, not early + late.
            let late_sum: u32 = data
                .chunks_exact(stride)
                .map(|command| u32::from_ne_bytes(command[4..8].try_into().unwrap()))
                .sum();
            drop(data);
            buffer.unmap();
            let mut counts = counts.lock().unwrap();
            counts.reads += 1;
            if counts.reads > WARMUP_READS {
                counts.checked += 1;
                counts.late_frames += usize::from(late_sum > 0);
                counts.zero_late_frames += usize::from(late_sum == 0);
                counts.max_late_instances = counts.max_late_instances.max(late_sum);
            }
        });
}

fn check_frame(event: On<ReadbackComplete>, config: Res<Config>, mut results: ResMut<Results>) {
    results.reads += 1;
    if results.reads <= WARMUP_READS {
        return;
    }
    assert_eq!(event.data.len(), (PIXELS * PIXELS * 4) as usize);
    results.checked += 1;
    let mut bad = 0;
    for y in 0..SIDE {
        for x in 0..SIDE {
            let samples: &[(u32, [u8; 3])] = if config.same_material {
                &[(CELL * 3 / 8, [255, 0, 0]), (CELL * 5 / 8, [0, 255, 0])]
            } else {
                &[(CELL / 2, [255, 0, 0])]
            };
            // Expected colored pixels reject an empty or entirely black frame.
            if samples.iter().any(|(column, expected)| {
                let offset =
                    (((SIDE - 1 - y) * CELL + CELL / 2) * PIXELS + x * CELL + column) as usize * 4;
                event.data[offset..offset + 3]
                    .iter()
                    .zip(expected)
                    .any(|(&actual, &expected)| actual.abs_diff(expected) > 4)
            }) {
                bad += 1;
            }
        }
    }
    if bad != 0 {
        results.bad_frames += 1;
        results.bad_cells += bad;
        if results.bad_frames <= 3 {
            println!("CORRUPT read={} bad_cells={bad}", results.reads);
        }
    }
    if results.checked == 1 || (bad != 0 && results.bad_frames == 1) {
        let suffix = if bad == 0 { "good" } else { "bad" };
        let path = PathBuf::from(format!("{}-{suffix}.png", config.output.display()));
        image::save_buffer(path, &event.data, PIXELS, PIXELS, image::ColorType::Rgba8).unwrap();
    }
}
