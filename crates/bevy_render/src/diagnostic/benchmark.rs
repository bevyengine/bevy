use alloc::{string::String, sync::Arc, vec::Vec};

use bevy_app::{App, Plugin, PostUpdate, PreUpdate};
use bevy_camera::visibility::VisibilitySystems;
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_ecs::{
    prelude::World,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_platform::{
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
    time::Instant,
};

use crate::{
    view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity},
    Render, RenderApp, RenderSystems,
};

const ENTITY_SYNC_MS: DiagnosticPath = DiagnosticPath::const_new("render_benchmark/entity_sync_ms");
const ENTITY_SYNC_RECORDS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/entity_sync_records");
const ENTITY_SYNC_ADDED: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/entity_sync_added");
const ENTITY_SYNC_REMOVED: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/entity_sync_removed");
const ENTITY_SYNC_COMPONENT_REMOVED: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/entity_sync_component_removed");
const EXTRACT_MS: DiagnosticPath = DiagnosticPath::const_new("render_benchmark/extract_ms");
const VISIBILITY_MS: DiagnosticPath = DiagnosticPath::const_new("render_benchmark/visibility_ms");
const MESH_EXTRACTION_MS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/mesh_extraction_ms");
const MATERIAL_EXTRACTION_MS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/material_extraction_ms");
const QUEUE_MESHES_MS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/queue_meshes_ms");
const PREPARE_MS: DiagnosticPath = DiagnosticPath::const_new("render_benchmark/prepare_ms");
const RENDER_WORLD_ENTITIES: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/render_world_entities");
const VISIBLE_ITEMS: DiagnosticPath = DiagnosticPath::const_new("render_benchmark/visible_items");
const OPAQUE_3D_PHASE_ITEMS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/phase/opaque3d_items");
const ALPHA_MASK_3D_PHASE_ITEMS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/phase/alpha_mask3d_items");
const TRANSPARENT_3D_PHASE_ITEMS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/phase/transparent3d_items");
const SHADOW_PHASE_ITEMS: DiagnosticPath =
    DiagnosticPath::const_new("render_benchmark/phase/shadow_items");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderBenchmarkViewId {
    pub main_entity_bits: u64,
    pub auxiliary_entity_bits: u64,
    pub subview_index: u32,
}

impl From<RetainedViewEntity> for RenderBenchmarkViewId {
    fn from(view: RetainedViewEntity) -> Self {
        Self {
            main_entity_bits: view.main_entity.id().to_bits(),
            auxiliary_entity_bits: view.auxiliary_entity.id().to_bits(),
            subview_index: view.subview_index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderBenchmarkViewCount {
    pub view: RenderBenchmarkViewId,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderBenchmarkPhaseCount {
    pub phase: String,
    pub view: RenderBenchmarkViewId,
    pub item_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderBenchmarkSnapshot {
    pub render_world_entity_count: usize,
    pub total_visible_items: usize,
    pub visible_items_per_view: Vec<RenderBenchmarkViewCount>,
    pub phase_item_counts: Vec<RenderBenchmarkPhaseCount>,
}

#[derive(Default)]
struct RenderBenchmarkMeasurementsInner {
    entity_sync_ns: AtomicU64,
    entity_sync_records: AtomicUsize,
    entity_sync_added: AtomicUsize,
    entity_sync_removed: AtomicUsize,
    entity_sync_component_removed: AtomicUsize,
    extract_ns: AtomicU64,
    visibility_ns: AtomicU64,
    mesh_extraction_ns: AtomicU64,
    material_extraction_ns: AtomicU64,
    queue_meshes_ns: AtomicU64,
    prepare_ns: AtomicU64,
    render_world_entities: AtomicUsize,
    visible_items: AtomicUsize,
    opaque_3d_phase_items: AtomicUsize,
    alpha_mask_3d_phase_items: AtomicUsize,
    transparent_3d_phase_items: AtomicUsize,
    shadow_phase_items: AtomicUsize,
    snapshot: Mutex<RenderBenchmarkSnapshot>,
}

#[derive(Clone, Default, Resource)]
pub struct RenderBenchmarkMeasurements(Arc<RenderBenchmarkMeasurementsInner>);

impl RenderBenchmarkMeasurements {
    pub fn record_entity_sync(
        &self,
        elapsed: core::time::Duration,
        records: usize,
        added: usize,
        removed: usize,
        component_removed: usize,
    ) {
        self.0
            .entity_sync_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
        self.0.entity_sync_records.store(records, Ordering::Relaxed);
        self.0.entity_sync_added.store(added, Ordering::Relaxed);
        self.0.entity_sync_removed.store(removed, Ordering::Relaxed);
        self.0
            .entity_sync_component_removed
            .store(component_removed, Ordering::Relaxed);
    }

    pub fn record_extract_schedule(&self, elapsed: core::time::Duration) {
        self.0
            .extract_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_visibility(&self, elapsed: core::time::Duration) {
        self.0
            .visibility_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_mesh_extraction(&self, elapsed: core::time::Duration) {
        self.0
            .mesh_extraction_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_material_extraction(&self, elapsed: core::time::Duration) {
        self.0
            .material_extraction_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_queue_meshes(&self, elapsed: core::time::Duration) {
        self.0
            .queue_meshes_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_prepare(&self, elapsed: core::time::Duration) {
        self.0
            .prepare_ns
            .store(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn update_render_visibility_snapshot(
        &self,
        render_world_entity_count: usize,
        visible_items_per_view: Vec<RenderBenchmarkViewCount>,
    ) {
        let total_visible_items = visible_items_per_view
            .iter()
            .map(|count| count.item_count)
            .sum::<usize>();

        self.0
            .render_world_entities
            .store(render_world_entity_count, Ordering::Relaxed);
        self.0
            .visible_items
            .store(total_visible_items, Ordering::Relaxed);

        let mut snapshot = self.0.snapshot.lock().unwrap();
        snapshot.render_world_entity_count = render_world_entity_count;
        snapshot.total_visible_items = total_visible_items;
        snapshot.visible_items_per_view = visible_items_per_view;
    }

    pub fn update_phase_snapshot(
        &self,
        opaque_3d_items: usize,
        alpha_mask_3d_items: usize,
        transparent_3d_items: usize,
        shadow_items: usize,
        phase_item_counts: Vec<RenderBenchmarkPhaseCount>,
    ) {
        self.0
            .opaque_3d_phase_items
            .store(opaque_3d_items, Ordering::Relaxed);
        self.0
            .alpha_mask_3d_phase_items
            .store(alpha_mask_3d_items, Ordering::Relaxed);
        self.0
            .transparent_3d_phase_items
            .store(transparent_3d_items, Ordering::Relaxed);
        self.0
            .shadow_phase_items
            .store(shadow_items, Ordering::Relaxed);

        self.0.snapshot.lock().unwrap().phase_item_counts = phase_item_counts;
    }

    pub fn snapshot(&self) -> RenderBenchmarkSnapshot {
        self.0.snapshot.lock().unwrap().clone()
    }

    pub fn entity_sync_ms(&self) -> f64 {
        ns_to_ms(self.0.entity_sync_ns.load(Ordering::Relaxed))
    }

    pub fn entity_sync_records(&self) -> usize {
        self.0.entity_sync_records.load(Ordering::Relaxed)
    }

    pub fn entity_sync_added(&self) -> usize {
        self.0.entity_sync_added.load(Ordering::Relaxed)
    }

    pub fn entity_sync_removed(&self) -> usize {
        self.0.entity_sync_removed.load(Ordering::Relaxed)
    }

    pub fn entity_sync_component_removed(&self) -> usize {
        self.0.entity_sync_component_removed.load(Ordering::Relaxed)
    }

    pub fn extract_ms(&self) -> f64 {
        ns_to_ms(self.0.extract_ns.load(Ordering::Relaxed))
    }

    pub fn visibility_ms(&self) -> f64 {
        ns_to_ms(self.0.visibility_ns.load(Ordering::Relaxed))
    }

    pub fn mesh_extraction_ms(&self) -> f64 {
        ns_to_ms(self.0.mesh_extraction_ns.load(Ordering::Relaxed))
    }

    pub fn material_extraction_ms(&self) -> f64 {
        ns_to_ms(self.0.material_extraction_ns.load(Ordering::Relaxed))
    }

    pub fn queue_meshes_ms(&self) -> f64 {
        ns_to_ms(self.0.queue_meshes_ns.load(Ordering::Relaxed))
    }

    pub fn prepare_ms(&self) -> f64 {
        ns_to_ms(self.0.prepare_ns.load(Ordering::Relaxed))
    }

    pub fn render_world_entity_count(&self) -> usize {
        self.0.render_world_entities.load(Ordering::Relaxed)
    }

    pub fn visible_item_count(&self) -> usize {
        self.0.visible_items.load(Ordering::Relaxed)
    }

    pub fn opaque_3d_phase_item_count(&self) -> usize {
        self.0.opaque_3d_phase_items.load(Ordering::Relaxed)
    }

    pub fn alpha_mask_3d_phase_item_count(&self) -> usize {
        self.0.alpha_mask_3d_phase_items.load(Ordering::Relaxed)
    }

    pub fn transparent_3d_phase_item_count(&self) -> usize {
        self.0.transparent_3d_phase_items.load(Ordering::Relaxed)
    }

    pub fn shadow_phase_item_count(&self) -> usize {
        self.0.shadow_phase_items.load(Ordering::Relaxed)
    }
}

#[derive(Default, Resource)]
struct RenderBenchmarkTimingState {
    visibility_start: Option<Instant>,
    queue_start: Option<Instant>,
    prepare_start: Option<Instant>,
}

pub fn ensure_render_benchmark_measurements(app: &mut App) -> RenderBenchmarkMeasurements {
    let measurements = app
        .world()
        .get_resource::<RenderBenchmarkMeasurements>()
        .cloned()
        .unwrap_or_default();

    if app
        .world()
        .get_resource::<RenderBenchmarkMeasurements>()
        .is_none()
    {
        app.insert_resource(measurements.clone());
    }

    if let Some(render_app) = app.get_sub_app_mut(RenderApp)
        && render_app
            .world()
            .get_resource::<RenderBenchmarkMeasurements>()
            .is_none()
    {
        render_app.insert_resource(measurements.clone());
    }

    measurements
}

#[derive(Default)]
pub struct RenderBenchmarkDiagnosticsPlugin;

impl Plugin for RenderBenchmarkDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        ensure_render_benchmark_measurements(app);

        app.register_diagnostic(Diagnostic::new(ENTITY_SYNC_MS).with_suffix(" ms"))
            .register_diagnostic(Diagnostic::new(ENTITY_SYNC_RECORDS).with_suffix(" records"))
            .register_diagnostic(Diagnostic::new(ENTITY_SYNC_ADDED).with_suffix(" entities"))
            .register_diagnostic(Diagnostic::new(ENTITY_SYNC_REMOVED).with_suffix(" entities"))
            .register_diagnostic(
                Diagnostic::new(ENTITY_SYNC_COMPONENT_REMOVED).with_suffix(" removals"),
            )
            .register_diagnostic(Diagnostic::new(EXTRACT_MS).with_suffix(" ms"))
            .register_diagnostic(Diagnostic::new(VISIBILITY_MS).with_suffix(" ms"))
            .register_diagnostic(Diagnostic::new(MESH_EXTRACTION_MS).with_suffix(" ms"))
            .register_diagnostic(Diagnostic::new(MATERIAL_EXTRACTION_MS).with_suffix(" ms"))
            .register_diagnostic(Diagnostic::new(QUEUE_MESHES_MS).with_suffix(" ms"))
            .register_diagnostic(Diagnostic::new(PREPARE_MS).with_suffix(" ms"))
            .register_diagnostic(
                Diagnostic::new(RENDER_WORLD_ENTITIES).with_suffix(" render entities"),
            )
            .register_diagnostic(Diagnostic::new(VISIBLE_ITEMS).with_suffix(" items"))
            .register_diagnostic(Diagnostic::new(OPAQUE_3D_PHASE_ITEMS).with_suffix(" items"))
            .register_diagnostic(Diagnostic::new(ALPHA_MASK_3D_PHASE_ITEMS).with_suffix(" items"))
            .register_diagnostic(Diagnostic::new(TRANSPARENT_3D_PHASE_ITEMS).with_suffix(" items"))
            .register_diagnostic(Diagnostic::new(SHADOW_PHASE_ITEMS).with_suffix(" items"))
            .init_resource::<RenderBenchmarkTimingState>()
            .add_systems(PreUpdate, add_render_benchmark_measurements)
            .add_systems(
                PostUpdate,
                (
                    start_visibility_timer.before(VisibilitySystems::CheckVisibility),
                    finish_visibility_timer.after(VisibilitySystems::CheckVisibility),
                ),
            );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderBenchmarkTimingState>()
                .add_systems(
                    Render,
                    (
                        start_queue_timer.before(RenderSystems::QueueMeshes),
                        finish_queue_timer
                            .after(RenderSystems::QueueMeshes)
                            .before(RenderSystems::PhaseSort),
                        start_prepare_timer.before(RenderSystems::Prepare),
                        finish_prepare_timer
                            .after(RenderSystems::Prepare)
                            .before(RenderSystems::Render),
                        sample_render_world_visibility.after(RenderSystems::PhaseSort),
                    ),
                );
        }
    }
}

fn add_render_benchmark_measurements(
    mut diagnostics: Diagnostics,
    measurements: Res<RenderBenchmarkMeasurements>,
) {
    diagnostics.add_measurement(&ENTITY_SYNC_MS, || measurements.entity_sync_ms());
    diagnostics.add_measurement(&ENTITY_SYNC_RECORDS, || {
        measurements.entity_sync_records() as f64
    });
    diagnostics.add_measurement(&ENTITY_SYNC_ADDED, || {
        measurements.entity_sync_added() as f64
    });
    diagnostics.add_measurement(&ENTITY_SYNC_REMOVED, || {
        measurements.entity_sync_removed() as f64
    });
    diagnostics.add_measurement(&ENTITY_SYNC_COMPONENT_REMOVED, || {
        measurements.entity_sync_component_removed() as f64
    });
    diagnostics.add_measurement(&EXTRACT_MS, || measurements.extract_ms());
    diagnostics.add_measurement(&VISIBILITY_MS, || measurements.visibility_ms());
    diagnostics.add_measurement(&MESH_EXTRACTION_MS, || measurements.mesh_extraction_ms());
    diagnostics.add_measurement(&MATERIAL_EXTRACTION_MS, || {
        measurements.material_extraction_ms()
    });
    diagnostics.add_measurement(&QUEUE_MESHES_MS, || measurements.queue_meshes_ms());
    diagnostics.add_measurement(&PREPARE_MS, || measurements.prepare_ms());
    diagnostics.add_measurement(&RENDER_WORLD_ENTITIES, || {
        measurements.render_world_entity_count() as f64
    });
    diagnostics.add_measurement(&VISIBLE_ITEMS, || measurements.visible_item_count() as f64);
    diagnostics.add_measurement(&OPAQUE_3D_PHASE_ITEMS, || {
        measurements.opaque_3d_phase_item_count() as f64
    });
    diagnostics.add_measurement(&ALPHA_MASK_3D_PHASE_ITEMS, || {
        measurements.alpha_mask_3d_phase_item_count() as f64
    });
    diagnostics.add_measurement(&TRANSPARENT_3D_PHASE_ITEMS, || {
        measurements.transparent_3d_phase_item_count() as f64
    });
    diagnostics.add_measurement(&SHADOW_PHASE_ITEMS, || {
        measurements.shadow_phase_item_count() as f64
    });
}

fn start_visibility_timer(mut state: ResMut<RenderBenchmarkTimingState>) {
    state.visibility_start = Some(Instant::now());
}

fn finish_visibility_timer(
    mut state: ResMut<RenderBenchmarkTimingState>,
    measurements: Res<RenderBenchmarkMeasurements>,
) {
    let Some(start) = state.visibility_start.take() else {
        return;
    };
    measurements.record_visibility(start.elapsed());
}

fn start_queue_timer(mut state: ResMut<RenderBenchmarkTimingState>) {
    state.queue_start = Some(Instant::now());
}

fn finish_queue_timer(
    mut state: ResMut<RenderBenchmarkTimingState>,
    measurements: Res<RenderBenchmarkMeasurements>,
) {
    let Some(start) = state.queue_start.take() else {
        return;
    };
    measurements.record_queue_meshes(start.elapsed());
}

fn start_prepare_timer(mut state: ResMut<RenderBenchmarkTimingState>) {
    state.prepare_start = Some(Instant::now());
}

fn finish_prepare_timer(
    mut state: ResMut<RenderBenchmarkTimingState>,
    measurements: Res<RenderBenchmarkMeasurements>,
) {
    let Some(start) = state.prepare_start.take() else {
        return;
    };
    measurements.record_prepare(start.elapsed());
}

fn sample_render_world_visibility(world: &mut World) {
    let Some(measurements) = world.get_resource::<RenderBenchmarkMeasurements>().cloned() else {
        return;
    };

    let mut visible_items_per_view = Vec::new();
    let mut view_query = world.query::<(&ExtractedView, &RenderVisibleEntities)>();
    for (view, visible_entities) in view_query.iter(world) {
        visible_items_per_view.push(RenderBenchmarkViewCount {
            view: view.retained_view_entity.into(),
            item_count: visible_entities
                .entities
                .values()
                .map(|entities| entities.entities.len())
                .sum(),
        });
    }

    measurements.update_render_visibility_snapshot(
        world.entities().count_spawned() as usize,
        visible_items_per_view,
    );
}

#[inline]
fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}
