use core::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
};

use bevy_app::{Plugin, PreUpdate};
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_ecs::{resource::Resource, system::Res};
use bevy_platform::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use bevy_render::{Extract, ExtractSchedule, RenderApp};

use crate::{Material, MaterialBindGroupAllocators};

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
