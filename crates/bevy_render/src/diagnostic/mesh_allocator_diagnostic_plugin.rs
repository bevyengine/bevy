use bevy_app::{Plugin, PreUpdate};
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_ecs::{resource::Resource, system::Res};
use bevy_platform::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::{mesh::allocator::MeshAllocator, Extract, ExtractSchedule, RenderApp};

/// Number of meshes allocated by the allocator
const MESH_ALLOCATOR_SLABS: DiagnosticPath = DiagnosticPath::const_new("mesh_allocator_slabs");

/// Number of meshes allocated by the allocator
const MESH_ALLOCATOR_SLABS_SIZE: DiagnosticPath =
    DiagnosticPath::const_new("mesh_allocator_slabs_size");

pub struct MeshAllocatorDiagnosticPlugin;

impl Plugin for MeshAllocatorDiagnosticPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_diagnostic(Diagnostic::new(MESH_ALLOCATOR_SLABS).with_suffix(" slabs"))
            .register_diagnostic(Diagnostic::new(MESH_ALLOCATOR_SLABS_SIZE).with_suffix(" bytes"))
            .init_resource::<MeshAllocatorMeasurements>()
            .add_systems(PreUpdate, add_mesh_allocator_measurement);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, measure_allocator);
        }
    }
}

#[derive(Debug, Default, Resource)]
struct MeshAllocatorMeasurements {
    slabs: AtomicUsize,
    slabs_size: AtomicU64,
}

fn add_mesh_allocator_measurement(
    mut diagnostics: Diagnostics,
    measurements: Res<MeshAllocatorMeasurements>,
) {
    diagnostics.add_measurement(&MESH_ALLOCATOR_SLABS, || {
        measurements.slabs.load(Ordering::Relaxed) as f64
    });
    diagnostics.add_measurement(&MESH_ALLOCATOR_SLABS_SIZE, || {
        measurements.slabs_size.load(Ordering::Relaxed) as f64
    });
}

fn measure_allocator(
    measurements: Extract<Res<MeshAllocatorMeasurements>>,
    allocator: Res<MeshAllocator>,
) {
    measurements
        .slabs
        .store(allocator.slab_count(), Ordering::Relaxed);
    measurements
        .slabs_size
        .store(allocator.slabs_size(), Ordering::Relaxed);
}
