use core::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
};

use bevy_app::{Plugin, PreUpdate};
use bevy_core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_ecs::{prelude::World, resource::Resource, schedule::IntoScheduleConfigs, system::Res};
use bevy_platform::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::Instant,
};
use bevy_render::{
    diagnostic::{
        ensure_render_benchmark_measurements, RenderBenchmarkMeasurements,
        RenderBenchmarkPhaseCount, RenderBenchmarkViewId,
    },
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhase, RenderBin, SortedPhaseItem, ViewBinnedRenderPhases,
        ViewSortedRenderPhases,
    },
    view::RetainedViewEntity,
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};

use crate::{
    material::MaterialExtractionSystems, render::mesh::MeshExtractionSystems, Material,
    MaterialBindGroupAllocators, Shadow,
};

pub struct MaterialAllocatorDiagnosticPlugin<M: Material> {
    suffix: &'static str,
    _phantom: PhantomData<M>,
}

impl<M: Material> MaterialAllocatorDiagnosticPlugin<M> {
    pub fn new(suffix: &'static str) -> Self {
        Self {
            suffix,
            _phantom: PhantomData,
        }
    }
}

impl<M: Material> Default for MaterialAllocatorDiagnosticPlugin<M> {
    fn default() -> Self {
        Self {
            suffix: " materials",
            _phantom: PhantomData,
        }
    }
}

impl<M: Material> MaterialAllocatorDiagnosticPlugin<M> {
    /// Get the [`DiagnosticPath`] for slab count
    pub fn slabs_diagnostic_path() -> DiagnosticPath {
        DiagnosticPath::from_components(["material_allocator_slabs", type_name::<M>()])
    }
    /// Get the [`DiagnosticPath`] for total slabs size
    pub fn slabs_size_diagnostic_path() -> DiagnosticPath {
        DiagnosticPath::from_components(["material_allocator_slabs_size", type_name::<M>()])
    }
    /// Get the [`DiagnosticPath`] for material allocations
    pub fn allocations_diagnostic_path() -> DiagnosticPath {
        DiagnosticPath::from_components(["material_allocator_allocations", type_name::<M>()])
    }
}

impl<M: Material> Plugin for MaterialAllocatorDiagnosticPlugin<M> {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_diagnostic(
            Diagnostic::new(Self::slabs_diagnostic_path()).with_suffix(" slabs"),
        )
        .register_diagnostic(
            Diagnostic::new(Self::slabs_size_diagnostic_path()).with_suffix(" bytes"),
        )
        .register_diagnostic(
            Diagnostic::new(Self::allocations_diagnostic_path()).with_suffix(self.suffix),
        )
        .init_resource::<MaterialAllocatorMeasurements<M>>()
        .add_systems(PreUpdate, add_material_allocator_measurement::<M>);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, measure_allocator::<M>);
        }
    }
}

#[derive(Debug, Resource)]
struct MaterialAllocatorMeasurements<M: Material> {
    slabs: AtomicUsize,
    slabs_size: AtomicUsize,
    allocations: AtomicU64,
    _phantom: PhantomData<M>,
}

impl<M: Material> Default for MaterialAllocatorMeasurements<M> {
    fn default() -> Self {
        Self {
            slabs: AtomicUsize::default(),
            slabs_size: AtomicUsize::default(),
            allocations: AtomicU64::default(),
            _phantom: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct PbrBenchmarkDiagnosticsPlugin;

#[derive(Default, Resource)]
struct PbrBenchmarkTimingState {
    mesh_extraction_start: Option<Instant>,
    material_extraction_start: Option<Instant>,
}

impl Plugin for PbrBenchmarkDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        ensure_render_benchmark_measurements(app);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PbrBenchmarkTimingState>()
            .add_systems(
                ExtractSchedule,
                (
                    start_mesh_extraction_timer.before(MeshExtractionSystems),
                    finish_mesh_extraction_timer.after(MeshExtractionSystems),
                    start_material_extraction_timer.before(MaterialExtractionSystems),
                    finish_material_extraction_timer.after(MaterialExtractionSystems),
                ),
            );

        render_app.add_systems(
            Render,
            sample_pbr_phase_counts
                .after(RenderSystems::PhaseSort)
                .before(RenderSystems::Prepare),
        );
    }
}

fn add_material_allocator_measurement<M: Material>(
    mut diagnostics: Diagnostics,
    measurements: Res<MaterialAllocatorMeasurements<M>>,
) {
    diagnostics.add_measurement(
        &MaterialAllocatorDiagnosticPlugin::<M>::slabs_diagnostic_path(),
        || measurements.slabs.load(Ordering::Relaxed) as f64,
    );
    diagnostics.add_measurement(
        &MaterialAllocatorDiagnosticPlugin::<M>::slabs_size_diagnostic_path(),
        || measurements.slabs_size.load(Ordering::Relaxed) as f64,
    );
    diagnostics.add_measurement(
        &MaterialAllocatorDiagnosticPlugin::<M>::allocations_diagnostic_path(),
        || measurements.allocations.load(Ordering::Relaxed) as f64,
    );
}

fn measure_allocator<M: Material + Any>(
    measurements: Extract<Res<MaterialAllocatorMeasurements<M>>>,
    allocators: Res<MaterialBindGroupAllocators>,
) {
    if let Some(allocator) = allocators.get(&TypeId::of::<M>()) {
        measurements
            .slabs
            .store(allocator.slab_count(), Ordering::Relaxed);
        measurements
            .slabs_size
            .store(allocator.slabs_size(), Ordering::Relaxed);
        measurements
            .allocations
            .store(allocator.allocations(), Ordering::Relaxed);
    }
}

fn start_mesh_extraction_timer(mut state: bevy_ecs::system::ResMut<PbrBenchmarkTimingState>) {
    state.mesh_extraction_start = Some(Instant::now());
}

fn finish_mesh_extraction_timer(
    mut state: bevy_ecs::system::ResMut<PbrBenchmarkTimingState>,
    measurements: Res<RenderBenchmarkMeasurements>,
) {
    let Some(start) = state.mesh_extraction_start.take() else {
        return;
    };
    measurements.record_mesh_extraction(start.elapsed());
}

fn start_material_extraction_timer(mut state: bevy_ecs::system::ResMut<PbrBenchmarkTimingState>) {
    state.material_extraction_start = Some(Instant::now());
}

fn finish_material_extraction_timer(
    mut state: bevy_ecs::system::ResMut<PbrBenchmarkTimingState>,
    measurements: Res<RenderBenchmarkMeasurements>,
) {
    let Some(start) = state.material_extraction_start.take() else {
        return;
    };
    measurements.record_material_extraction(start.elapsed());
}

fn sample_pbr_phase_counts(world: &mut World) {
    let Some(measurements) = world.get_resource::<RenderBenchmarkMeasurements>().cloned() else {
        return;
    };

    let opaque_counts = collect_binned_phase_counts::<Opaque3d>(world, "opaque3d");
    let alpha_mask_counts = collect_binned_phase_counts::<AlphaMask3d>(world, "alpha_mask3d");
    let transparent_counts = collect_sorted_phase_counts::<Transparent3d>(world, "transparent3d");
    let shadow_counts = collect_binned_phase_counts::<Shadow>(world, "shadow");

    let opaque_total = opaque_counts.iter().map(|count| count.item_count).sum();
    let alpha_mask_total = alpha_mask_counts.iter().map(|count| count.item_count).sum();
    let transparent_total = transparent_counts
        .iter()
        .map(|count| count.item_count)
        .sum();
    let shadow_total = shadow_counts.iter().map(|count| count.item_count).sum();

    let mut phase_item_counts = opaque_counts;
    phase_item_counts.extend(alpha_mask_counts);
    phase_item_counts.extend(transparent_counts);
    phase_item_counts.extend(shadow_counts);

    measurements.update_phase_snapshot(
        opaque_total,
        alpha_mask_total,
        transparent_total,
        shadow_total,
        phase_item_counts,
    );
}

fn collect_binned_phase_counts<BPI>(
    world: &World,
    phase_name: &str,
) -> Vec<RenderBenchmarkPhaseCount>
where
    BPI: BinnedPhaseItem,
{
    let Some(phases) = world.get_resource::<ViewBinnedRenderPhases<BPI>>() else {
        return Vec::new();
    };

    let mut counts = Vec::with_capacity(phases.len());
    for (view, phase) in phases.iter() {
        counts.push(RenderBenchmarkPhaseCount {
            phase: phase_name.into(),
            view: view_to_benchmark_id(*view),
            item_count: binned_phase_item_count(phase),
        });
    }
    counts
}

fn collect_sorted_phase_counts<SPI>(
    world: &World,
    phase_name: &str,
) -> Vec<RenderBenchmarkPhaseCount>
where
    SPI: SortedPhaseItem,
{
    let Some(phases) = world.get_resource::<ViewSortedRenderPhases<SPI>>() else {
        return Vec::new();
    };

    let mut counts = Vec::with_capacity(phases.len());
    for (view, phase) in phases.iter() {
        counts.push(RenderBenchmarkPhaseCount {
            phase: phase_name.into(),
            view: view_to_benchmark_id(*view),
            item_count: phase.item_count(),
        });
    }
    counts
}

fn binned_phase_item_count<BPI>(phase: &BinnedRenderPhase<BPI>) -> usize
where
    BPI: BinnedPhaseItem,
{
    phase
        .multidrawable_meshes
        .values()
        .flat_map(|bins| bins.values())
        .map(RenderBin::entities)
        .map(|entities| entities.len())
        .sum::<usize>()
        + phase
            .batchable_meshes
            .values()
            .map(RenderBin::entities)
            .map(|entities| entities.len())
            .sum::<usize>()
        + phase
            .unbatchable_meshes
            .values()
            .map(|entities| entities.entities.len())
            .sum::<usize>()
        + phase
            .non_mesh_items
            .values()
            .map(|entities| entities.entities.len())
            .sum::<usize>()
}

fn view_to_benchmark_id(view: RetainedViewEntity) -> RenderBenchmarkViewId {
    view.into()
}
