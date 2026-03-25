use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::BufWriter,
    mem::size_of,
    path::PathBuf,
};

use bevy::{
    app::AppExit,
    diagnostic::{
        DiagnosticsStore, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
    },
    ecs::{
        archetype::ArchetypeEntity,
        change_detection::Tick,
        resource::Resource,
        storage::{TableId, TableRow},
    },
    log::{error, info},
    pbr::{diagnostic::MaterialAllocatorDiagnosticPlugin, StandardMaterial},
    prelude::*,
    render::diagnostic::{
        MeshAllocatorDiagnosticPlugin, RenderBenchmarkMeasurements, RenderBenchmarkPhaseCount,
        RenderBenchmarkSnapshot, RenderBenchmarkViewCount,
    },
};
use serde::Serialize;

#[derive(Clone, Default, Resource)]
pub struct BenchmarkMetadata(pub BTreeMap<String, String>);

#[derive(Clone, Resource)]
struct BenchmarkCaptureConfig {
    scene: String,
    output_path: Option<PathBuf>,
    warmup_frames: u32,
    sample_frames: u32,
}

#[derive(Default, Resource)]
struct BenchmarkCaptureState {
    frame_index: u32,
    samples: Vec<BenchmarkFrameSample>,
}

#[derive(Serialize)]
struct BenchmarkArtifact {
    scene: String,
    metadata: BTreeMap<String, String>,
    warmup_frames: u32,
    sample_frames: u32,
    captured_frames: usize,
    summary: BenchmarkSummary,
    last_render_snapshot: Option<SerializableRenderBenchmarkSnapshot>,
    samples: Vec<BenchmarkFrameSample>,
}

#[derive(Serialize)]
struct BenchmarkSummary {
    averages: BenchmarkAggregate,
    maxima: BenchmarkAggregate,
}

#[derive(Default, Serialize)]
struct BenchmarkAggregate {
    frame_time_ms: Option<f64>,
    extract_time_ms: Option<f64>,
    entity_sync_time_ms: Option<f64>,
    entity_sync_record_count: Option<f64>,
    entity_sync_added_count: Option<f64>,
    entity_sync_removed_count: Option<f64>,
    entity_sync_component_removed_count: Option<f64>,
    visibility_time_ms: Option<f64>,
    mesh_extraction_time_ms: Option<f64>,
    material_extraction_time_ms: Option<f64>,
    queue_time_ms: Option<f64>,
    prepare_time_ms: Option<f64>,
    render_world_entity_count: Option<f64>,
    visible_item_count: Option<f64>,
    opaque_phase_item_count: Option<f64>,
    alpha_mask_phase_item_count: Option<f64>,
    transparent_phase_item_count: Option<f64>,
    shadow_phase_item_count: Option<f64>,
    renderer_cpu_memory_bytes: Option<f64>,
    mass_instance_chunk_count: Option<f64>,
    mass_instance_indexed_entity_count: Option<f64>,
    mass_instance_dirty_chunk_count: Option<f64>,
    scene_world_memory_bytes_estimate: Option<f64>,
    scene_process_memory_bytes: Option<f64>,
    main_world_entity_count: Option<f64>,
}

#[derive(Clone, Serialize)]
struct BenchmarkFrameSample {
    frame_index: u32,
    frame_time_ms: f64,
    extract_time_ms: Option<f64>,
    entity_sync_time_ms: Option<f64>,
    entity_sync_record_count: Option<usize>,
    entity_sync_added_count: Option<usize>,
    entity_sync_removed_count: Option<usize>,
    entity_sync_component_removed_count: Option<usize>,
    visibility_time_ms: Option<f64>,
    mesh_extraction_time_ms: Option<f64>,
    material_extraction_time_ms: Option<f64>,
    queue_time_ms: Option<f64>,
    prepare_time_ms: Option<f64>,
    render_world_entity_count: Option<usize>,
    visible_item_count: Option<usize>,
    opaque_phase_item_count: Option<usize>,
    alpha_mask_phase_item_count: Option<usize>,
    transparent_phase_item_count: Option<usize>,
    shadow_phase_item_count: Option<usize>,
    renderer_cpu_memory_bytes: Option<u64>,
    mass_instance_chunk_count: Option<usize>,
    mass_instance_indexed_entity_count: Option<usize>,
    mass_instance_dirty_chunk_count: Option<usize>,
    scene_world_memory_bytes_estimate: u64,
    scene_process_memory_bytes: Option<u64>,
    main_world_entity_count: usize,
}

#[derive(Clone, Serialize)]
struct SerializableRenderBenchmarkSnapshot {
    render_world_entity_count: usize,
    total_visible_items: usize,
    visible_items_per_view: Vec<SerializableRenderBenchmarkViewCount>,
    phase_item_counts: Vec<SerializableRenderBenchmarkPhaseCount>,
}

#[derive(Clone, Serialize)]
struct SerializableRenderBenchmarkViewCount {
    main_entity_bits: u64,
    auxiliary_entity_bits: u64,
    subview_index: u32,
    item_count: usize,
}

#[derive(Clone, Serialize)]
struct SerializableRenderBenchmarkPhaseCount {
    phase: String,
    main_entity_bits: u64,
    auxiliary_entity_bits: u64,
    subview_index: u32,
    item_count: usize,
}

pub struct BenchmarkOutputPlugin {
    scene: String,
}

impl BenchmarkOutputPlugin {
    pub fn new(scene: impl Into<String>) -> Self {
        Self {
            scene: scene.into(),
        }
    }
}

impl Plugin for BenchmarkOutputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BenchmarkCaptureConfig {
            scene: self.scene.clone(),
            output_path: std::env::var_os("BEVY_BENCHMARK_OUTPUT_PATH").map(PathBuf::from),
            warmup_frames: parse_env_u32("BEVY_BENCHMARK_WARMUP_FRAMES", 120),
            sample_frames: parse_env_u32("BEVY_BENCHMARK_SAMPLE_FRAMES", 120),
        })
        .init_resource::<BenchmarkCaptureState>()
        .add_systems(Last, capture_benchmark_sample);
    }
}

fn capture_benchmark_sample(world: &mut World) {
    let Some(config) = world.get_resource::<BenchmarkCaptureConfig>().cloned() else {
        return;
    };
    let Some(output_path) = config.output_path.clone() else {
        return;
    };

    let frame_index = {
        let mut state = world.resource_mut::<BenchmarkCaptureState>();
        state.frame_index += 1;
        if state.frame_index <= config.warmup_frames {
            return;
        }
        state.frame_index
    };

    let sample = collect_sample(world, frame_index);

    {
        let mut state = world.resource_mut::<BenchmarkCaptureState>();
        state.samples.push(sample);
        if state.samples.len() < config.sample_frames as usize {
            return;
        }
    }

    let metadata = world
        .get_resource::<BenchmarkMetadata>()
        .map(|metadata| metadata.0.clone())
        .unwrap_or_default();
    let samples = world.resource::<BenchmarkCaptureState>().samples.clone();
    let summary = summarize_samples(&samples);
    let last_render_snapshot = world
        .get_resource::<RenderBenchmarkMeasurements>()
        .map(|measurements| serialize_snapshot(measurements.snapshot()));

    let artifact = BenchmarkArtifact {
        scene: config.scene,
        metadata,
        warmup_frames: config.warmup_frames,
        sample_frames: config.sample_frames,
        captured_frames: samples.len(),
        summary,
        last_render_snapshot,
        samples,
    };

    if let Some(parent) = output_path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        error!("failed to create benchmark output directory: {error}");
        world.write_message(AppExit::error());
        return;
    }

    match File::create(&output_path) {
        Ok(file) => {
            let writer = BufWriter::new(file);
            if let Err(error) = serde_json::to_writer_pretty(writer, &artifact) {
                error!("failed to write benchmark artifact: {error}");
                world.write_message(AppExit::error());
                return;
            }
        }
        Err(error) => {
            error!("failed to create benchmark artifact file: {error}");
            world.write_message(AppExit::error());
            return;
        }
    }

    info!("wrote benchmark artifact to {}", output_path.display());
    world.write_message(AppExit::Success);
}

fn collect_sample(world: &mut World, frame_index: u32) -> BenchmarkFrameSample {
    let diagnostics = world.resource::<DiagnosticsStore>();
    let measurements = world.get_resource::<RenderBenchmarkMeasurements>();

    let mesh_allocator_bytes = diagnostic_value(
        diagnostics,
        MeshAllocatorDiagnosticPlugin::slabs_size_diagnostic_path(),
    )
    .map(|value| value.max(0.0) as u64);
    let material_allocator_path =
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_size_diagnostic_path();
    let material_allocator_bytes =
        diagnostic_value(diagnostics, &material_allocator_path).map(|value| value.max(0.0) as u64);

    let renderer_cpu_memory_bytes = match (mesh_allocator_bytes, material_allocator_bytes) {
        (None, None) => None,
        (Some(mesh_bytes), None) => Some(mesh_bytes),
        (None, Some(material_bytes)) => Some(material_bytes),
        (Some(mesh_bytes), Some(material_bytes)) => Some(mesh_bytes + material_bytes),
    };

    let process_memory_bytes = diagnostic_value(
        diagnostics,
        &SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    )
    .map(gib_to_bytes);
    let mass_instance_chunk_index =
        world.get_resource::<bevy::pbr::experimental::MassInstanceChunkIndex>();
    let scene_world_memory_bytes_estimate = estimate_scene_world_memory_bytes(world);

    BenchmarkFrameSample {
        frame_index,
        frame_time_ms: diagnostic_value(diagnostics, &FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .unwrap_or_default(),
        extract_time_ms: measurements.map(RenderBenchmarkMeasurements::extract_ms),
        entity_sync_time_ms: measurements.map(RenderBenchmarkMeasurements::entity_sync_ms),
        entity_sync_record_count: measurements
            .map(RenderBenchmarkMeasurements::entity_sync_records),
        entity_sync_added_count: measurements.map(RenderBenchmarkMeasurements::entity_sync_added),
        entity_sync_removed_count: measurements
            .map(RenderBenchmarkMeasurements::entity_sync_removed),
        entity_sync_component_removed_count: measurements
            .map(RenderBenchmarkMeasurements::entity_sync_component_removed),
        visibility_time_ms: measurements.map(RenderBenchmarkMeasurements::visibility_ms),
        mesh_extraction_time_ms: measurements.map(RenderBenchmarkMeasurements::mesh_extraction_ms),
        material_extraction_time_ms: measurements
            .map(RenderBenchmarkMeasurements::material_extraction_ms),
        queue_time_ms: measurements.map(RenderBenchmarkMeasurements::queue_meshes_ms),
        prepare_time_ms: measurements.map(RenderBenchmarkMeasurements::prepare_ms),
        render_world_entity_count: measurements
            .map(RenderBenchmarkMeasurements::render_world_entity_count),
        visible_item_count: measurements.map(RenderBenchmarkMeasurements::visible_item_count),
        opaque_phase_item_count: measurements
            .map(RenderBenchmarkMeasurements::opaque_3d_phase_item_count),
        alpha_mask_phase_item_count: measurements
            .map(RenderBenchmarkMeasurements::alpha_mask_3d_phase_item_count),
        transparent_phase_item_count: measurements
            .map(RenderBenchmarkMeasurements::transparent_3d_phase_item_count),
        shadow_phase_item_count: measurements
            .map(RenderBenchmarkMeasurements::shadow_phase_item_count),
        renderer_cpu_memory_bytes,
        mass_instance_chunk_count: mass_instance_chunk_index
            .map(bevy::pbr::experimental::MassInstanceChunkIndex::chunk_count),
        mass_instance_indexed_entity_count: mass_instance_chunk_index
            .map(bevy::pbr::experimental::MassInstanceChunkIndex::entity_count),
        mass_instance_dirty_chunk_count: mass_instance_chunk_index
            .map(bevy::pbr::experimental::MassInstanceChunkIndex::dirty_chunk_count),
        scene_world_memory_bytes_estimate,
        scene_process_memory_bytes: process_memory_bytes,
        main_world_entity_count: world.entities().count_spawned() as usize,
    }
}

fn summarize_samples(samples: &[BenchmarkFrameSample]) -> BenchmarkSummary {
    BenchmarkSummary {
        averages: BenchmarkAggregate {
            frame_time_ms: average_required(samples.iter().map(|sample| sample.frame_time_ms)),
            extract_time_ms: average_optional(samples.iter().map(|sample| sample.extract_time_ms)),
            entity_sync_time_ms: average_optional(
                samples.iter().map(|sample| sample.entity_sync_time_ms),
            ),
            entity_sync_record_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.entity_sync_record_count.map(|value| value as f64)),
            ),
            entity_sync_added_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.entity_sync_added_count.map(|value| value as f64)),
            ),
            entity_sync_removed_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.entity_sync_removed_count.map(|value| value as f64)),
            ),
            entity_sync_component_removed_count: average_optional(samples.iter().map(|sample| {
                sample
                    .entity_sync_component_removed_count
                    .map(|value| value as f64)
            })),
            visibility_time_ms: average_optional(
                samples.iter().map(|sample| sample.visibility_time_ms),
            ),
            mesh_extraction_time_ms: average_optional(
                samples.iter().map(|sample| sample.mesh_extraction_time_ms),
            ),
            material_extraction_time_ms: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.material_extraction_time_ms),
            ),
            queue_time_ms: average_optional(samples.iter().map(|sample| sample.queue_time_ms)),
            prepare_time_ms: average_optional(samples.iter().map(|sample| sample.prepare_time_ms)),
            render_world_entity_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.render_world_entity_count.map(|value| value as f64)),
            ),
            visible_item_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.visible_item_count.map(|value| value as f64)),
            ),
            opaque_phase_item_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.opaque_phase_item_count.map(|value| value as f64)),
            ),
            alpha_mask_phase_item_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.alpha_mask_phase_item_count.map(|value| value as f64)),
            ),
            transparent_phase_item_count: average_optional(samples.iter().map(|sample| {
                sample
                    .transparent_phase_item_count
                    .map(|value| value as f64)
            })),
            shadow_phase_item_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.shadow_phase_item_count.map(|value| value as f64)),
            ),
            renderer_cpu_memory_bytes: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.renderer_cpu_memory_bytes.map(|value| value as f64)),
            ),
            mass_instance_chunk_count: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.mass_instance_chunk_count.map(|value| value as f64)),
            ),
            mass_instance_indexed_entity_count: average_optional(samples.iter().map(|sample| {
                sample
                    .mass_instance_indexed_entity_count
                    .map(|value| value as f64)
            })),
            mass_instance_dirty_chunk_count: average_optional(samples.iter().map(|sample| {
                sample
                    .mass_instance_dirty_chunk_count
                    .map(|value| value as f64)
            })),
            scene_world_memory_bytes_estimate: average_optional(
                samples
                    .iter()
                    .map(|sample| Some(sample.scene_world_memory_bytes_estimate as f64)),
            ),
            scene_process_memory_bytes: average_optional(
                samples
                    .iter()
                    .map(|sample| sample.scene_process_memory_bytes.map(|value| value as f64)),
            ),
            main_world_entity_count: average_optional(
                samples
                    .iter()
                    .map(|sample| Some(sample.main_world_entity_count as f64)),
            ),
        },
        maxima: BenchmarkAggregate {
            frame_time_ms: max_required(samples.iter().map(|sample| sample.frame_time_ms)),
            extract_time_ms: max_optional(samples.iter().map(|sample| sample.extract_time_ms)),
            entity_sync_time_ms: max_optional(
                samples.iter().map(|sample| sample.entity_sync_time_ms),
            ),
            entity_sync_record_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.entity_sync_record_count.map(|value| value as f64)),
            ),
            entity_sync_added_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.entity_sync_added_count.map(|value| value as f64)),
            ),
            entity_sync_removed_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.entity_sync_removed_count.map(|value| value as f64)),
            ),
            entity_sync_component_removed_count: max_optional(samples.iter().map(|sample| {
                sample
                    .entity_sync_component_removed_count
                    .map(|value| value as f64)
            })),
            visibility_time_ms: max_optional(
                samples.iter().map(|sample| sample.visibility_time_ms),
            ),
            mesh_extraction_time_ms: max_optional(
                samples.iter().map(|sample| sample.mesh_extraction_time_ms),
            ),
            material_extraction_time_ms: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.material_extraction_time_ms),
            ),
            queue_time_ms: max_optional(samples.iter().map(|sample| sample.queue_time_ms)),
            prepare_time_ms: max_optional(samples.iter().map(|sample| sample.prepare_time_ms)),
            render_world_entity_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.render_world_entity_count.map(|value| value as f64)),
            ),
            visible_item_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.visible_item_count.map(|value| value as f64)),
            ),
            opaque_phase_item_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.opaque_phase_item_count.map(|value| value as f64)),
            ),
            alpha_mask_phase_item_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.alpha_mask_phase_item_count.map(|value| value as f64)),
            ),
            transparent_phase_item_count: max_optional(samples.iter().map(|sample| {
                sample
                    .transparent_phase_item_count
                    .map(|value| value as f64)
            })),
            shadow_phase_item_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.shadow_phase_item_count.map(|value| value as f64)),
            ),
            renderer_cpu_memory_bytes: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.renderer_cpu_memory_bytes.map(|value| value as f64)),
            ),
            mass_instance_chunk_count: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.mass_instance_chunk_count.map(|value| value as f64)),
            ),
            mass_instance_indexed_entity_count: max_optional(samples.iter().map(|sample| {
                sample
                    .mass_instance_indexed_entity_count
                    .map(|value| value as f64)
            })),
            mass_instance_dirty_chunk_count: max_optional(samples.iter().map(|sample| {
                sample
                    .mass_instance_dirty_chunk_count
                    .map(|value| value as f64)
            })),
            scene_world_memory_bytes_estimate: max_optional(
                samples
                    .iter()
                    .map(|sample| Some(sample.scene_world_memory_bytes_estimate as f64)),
            ),
            scene_process_memory_bytes: max_optional(
                samples
                    .iter()
                    .map(|sample| sample.scene_process_memory_bytes.map(|value| value as f64)),
            ),
            main_world_entity_count: max_optional(
                samples
                    .iter()
                    .map(|sample| Some(sample.main_world_entity_count as f64)),
            ),
        },
    }
}

fn serialize_snapshot(snapshot: RenderBenchmarkSnapshot) -> SerializableRenderBenchmarkSnapshot {
    SerializableRenderBenchmarkSnapshot {
        render_world_entity_count: snapshot.render_world_entity_count,
        total_visible_items: snapshot.total_visible_items,
        visible_items_per_view: snapshot
            .visible_items_per_view
            .into_iter()
            .map(serialize_view_count)
            .collect(),
        phase_item_counts: snapshot
            .phase_item_counts
            .into_iter()
            .map(serialize_phase_count)
            .collect(),
    }
}

fn serialize_view_count(
    view_count: RenderBenchmarkViewCount,
) -> SerializableRenderBenchmarkViewCount {
    SerializableRenderBenchmarkViewCount {
        main_entity_bits: view_count.view.main_entity_bits,
        auxiliary_entity_bits: view_count.view.auxiliary_entity_bits,
        subview_index: view_count.view.subview_index,
        item_count: view_count.item_count,
    }
}

fn serialize_phase_count(
    phase_count: RenderBenchmarkPhaseCount,
) -> SerializableRenderBenchmarkPhaseCount {
    SerializableRenderBenchmarkPhaseCount {
        phase: phase_count.phase,
        main_entity_bits: phase_count.view.main_entity_bits,
        auxiliary_entity_bits: phase_count.view.auxiliary_entity_bits,
        subview_index: phase_count.view.subview_index,
        item_count: phase_count.item_count,
    }
}

fn diagnostic_value(
    diagnostics: &DiagnosticsStore,
    path: &bevy::diagnostic::DiagnosticPath,
) -> Option<f64> {
    diagnostics
        .get(path)
        .and_then(|diagnostic| diagnostic.value())
}

fn estimate_scene_world_memory_bytes(world: &World) -> u64 {
    let components = world.components();
    let storages = world.storages();

    let mut table_components = vec![None; storages.tables.len()];
    let mut total_bytes = 0u64;

    for archetype in world.archetypes().iter() {
        total_bytes += archetype.entities().len() as u64 * size_of::<ArchetypeEntity>() as u64;

        let table_index = archetype.table_id().as_usize();
        if table_components[table_index].is_none() {
            table_components[table_index] = Some(archetype.table_components().collect::<Vec<_>>());
        }
    }

    for (table_index, component_ids) in table_components.into_iter().enumerate() {
        let Some(component_ids) = component_ids else {
            continue;
        };
        let Some(table) = storages.tables.get(TableId::from_usize(table_index)) else {
            continue;
        };

        let capacity = table.entity_capacity() as u64;
        total_bytes += capacity * size_of::<Entity>() as u64;

        for component_id in component_ids {
            let Some(component_info) = components.get_info(component_id) else {
                continue;
            };
            total_bytes += capacity * table_component_slot_bytes(component_info.layout()) as u64;
        }
    }

    for (component_id, sparse_set) in storages.sparse_sets.iter() {
        let Some(component_info) = components.get_info(component_id) else {
            continue;
        };
        total_bytes +=
            sparse_set.len() as u64 * sparse_component_slot_bytes(component_info.layout()) as u64;
    }

    total_bytes
}

fn table_component_slot_bytes(layout: std::alloc::Layout) -> usize {
    layout.pad_to_align().size() + size_of::<Tick>() * 2 + maybe_location_slot_bytes()
}

fn sparse_component_slot_bytes(layout: std::alloc::Layout) -> usize {
    table_component_slot_bytes(layout) + size_of::<Entity>() + size_of::<TableRow>()
}

fn maybe_location_slot_bytes() -> usize {
    if cfg!(feature = "track_location") {
        size_of::<&'static std::panic::Location<'static>>()
    } else {
        0
    }
}

fn parse_env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn average_required(values: impl Iterator<Item = f64>) -> Option<f64> {
    average_optional(values.map(Some))
}

fn average_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0usize), |(sum, count), value| match value {
        Some(value) => (sum + value, count + 1),
        None => (sum, count),
    });
    (count > 0).then_some(sum / count as f64)
}

fn max_required(values: impl Iterator<Item = f64>) -> Option<f64> {
    max_optional(values.map(Some))
}

fn max_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    values.flatten().reduce(f64::max)
}

fn gib_to_bytes(gib: f64) -> u64 {
    (gib * 1024.0 * 1024.0 * 1024.0).round() as u64
}
