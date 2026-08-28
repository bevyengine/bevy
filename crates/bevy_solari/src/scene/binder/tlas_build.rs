//! Building the binder's TLAS through `wgpu_hal` directly, bypassing `wgpu-core` and using
//! an indirect acceleration structure build.
//!
//! Vulkan and DX12 only, as their instance descriptors are byte-identical. Metal cannot work this
//! way: `wgpu-core` makes a TLAS's BLASes resident from its dependency list, and
//! `mark_acceleration_structures_built` clears that list, so traversal would read memory Metal is
//! free to have evicted. Every other backend falls back to the `wgpu-core` build in [`super::tlas`].
//!
//! Every operation has to be instantiated per backend, as `as_hal` is generic over the hal type and
//! wgpu exposes no dynamic equivalent. [`resolve`] does that once, by trying each compiled-in
//! backend and relying on `as_hal` returning `None` for a device that isn't its own, which can't
//! disagree with the adapter actually in use.

#![expect(
    unsafe_code,
    reason = "Building the TLAS without wgpu-core's per-instance cost requires wgpu_hal."
)]
// With neither backend compiled in every arm is cfg'd away, so nothing here is instantiated. It
// still has to exist, because Solari compiles everywhere and only declines to load at runtime.
#![cfg_attr(
    not(any(
        windows,
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd"
    )),
    expect(
        dead_code,
        unused_variables,
        reason = "no backend with a raw TLAS build path is compiled in for this target"
    )
)]

use bevy_render::{
    render_resource::{Blas, Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Tlas},
    renderer::RenderDevice,
};
use core::marker::PhantomData;
use wgpu::{hal, BufferUses};

/// Size of one TLAS instance descriptor, on both Vulkan and DXR.
pub const INSTANCE_DESCRIPTOR_SIZE: u64 = 64;

/// Flags the TLAS is created with, which its build has to be given again.
const TLAS_BUILD_FLAGS: wgpu::AccelerationStructureFlags =
    wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE;

/// The `wgpu_hal` operations a raw TLAS build needs, bound to one backend by [`resolve`].
pub trait RawTlasBackend: Send + Sync {
    /// Scratch space a TLAS build over `instance_count` instances of `instances` needs.
    fn scratch_size(
        &self,
        render_device: &RenderDevice,
        instances: &Buffer,
        instance_count: u32,
    ) -> Option<u64>;

    /// Allocates a buffer usable as acceleration structure build scratch.
    ///
    /// There is no [`BufferUsages`] for scratch and `STORAGE` won't do, as Vulkan only adds
    /// `SHADER_DEVICE_ADDRESS` for acceleration structure usages. So it is created through hal and
    /// handed to `wgpu-core` to own. `wgpu-core` only defers freeing what a submission's tracker
    /// references, so the caller transitioning it each frame is what makes replacing it safe.
    fn create_scratch_buffer(&self, render_device: &RenderDevice, size: u64) -> Option<Buffer>;

    /// Records a TLAS build reading instance descriptors straight out of `instances`.
    ///
    /// The caller must already have transitioned `instances` to
    /// `BufferUses::TOP_LEVEL_ACCELERATION_STRUCTURE_INPUT` and `scratch` to
    /// `BufferUses::ACCELERATION_STRUCTURE_SCRATCH`, on an earlier command buffer in the same
    /// submission. The scratch transition is not optional even when its state is unchanged: scratch
    /// is an exclusive usage, so the redundant-looking transition is what keeps consecutive frames'
    /// builds from overlapping in it.
    ///
    /// `encoder` must have no wgpu command recorded into it, including timestamp writes, as
    /// `wgpu-core` panics if an encoder mixes the wgpu and raw encoding APIs.
    ///
    /// Marking the build clears the TLAS's dependency list, which is what otherwise keeps the
    /// BLASes it points at alive. The caller has to retain them past every submission that might
    /// still trace it.
    fn build_tlas(
        &self,
        encoder: &mut CommandEncoder,
        tlas: &mut Tlas,
        instances: &Buffer,
        instance_count: u32,
        scratch: &Buffer,
    ) -> bool;
}

/// The backend to build through, or `None` where this device has no raw path.
pub fn resolve(render_device: &RenderDevice) -> Option<&'static dyn RawTlasBackend> {
    let device = render_device.wgpu_device();

    #[cfg(any(
        windows,
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd"
    ))]
    // SAFETY: the handle is only read, and is dropped with the guard
    if unsafe { device.as_hal::<hal::api::Vulkan>() }.is_some() {
        return Some(&Hal::<hal::api::Vulkan>::NEW);
    }

    #[cfg(windows)]
    // SAFETY: the handle is only read, and is dropped with the guard
    if unsafe { device.as_hal::<hal::api::Dx12>() }.is_some() {
        return Some(&Hal::<hal::api::Dx12>::NEW);
    }

    None
}

/// [`RawTlasBackend`] for one hal backend.
///
/// `fn() -> A` rather than `A`, so the marker is [`Send`] and [`Sync`] whatever `A` is.
struct Hal<A>(PhantomData<fn() -> A>);

impl<A: hal::Api> Hal<A> {
    const NEW: Self = Self(PhantomData);

    /// Records the build itself, leaving [`RawTlasBackend::build_tlas`] to mark it.
    fn record_build(
        &self,
        encoder: &mut CommandEncoder,
        tlas: &mut Tlas,
        instances: &Buffer,
        instance_count: u32,
        scratch: &Buffer,
    ) -> Option<()> {
        use hal::CommandEncoder as _;

        // SAFETY: the handles are only read, and none of them is destroyed here
        let hal_instances: *const A::Buffer = &*unsafe { instances.as_hal::<A>() }?;
        // SAFETY: as above
        let hal_scratch: *const A::Buffer = &*unsafe { scratch.as_hal::<A>() }?;
        // SAFETY: as above
        let hal_tlas = unsafe { tlas.as_hal::<A>() }?;

        // SAFETY: both buffers outlive this call through the caller's borrows, and neither is
        // destroyed between reading its address and the build recording
        let (hal_instances, hal_scratch) = unsafe { (&*hal_instances, &*hal_scratch) };

        let entries =
            hal::AccelerationStructureEntries::Instances(hal::AccelerationStructureInstances {
                buffer: Some(hal_instances),
                offset: 0,
                count: instance_count,
            });

        let descriptor = hal::BuildAccelerationStructureDescriptor {
            entries: &entries,
            mode: hal::AccelerationStructureBuildMode::Build,
            flags: TLAS_BUILD_FLAGS,
            source_acceleration_structure: None,
            destination_acceleration_structure: &*hal_tlas,
            scratch_buffer: hal_scratch,
            scratch_buffer_offset: 0,
        };

        // The caller transitioned both buffers, so the only barriers left are the acceleration
        // structure's own, which have no public spelling.
        //
        // SAFETY: every resource in `descriptor` belongs to backend `A` and to this device, the
        // scratch buffer is sized by `scratch_size` and used by no other build in this submission,
        // and the encoder is neither ended nor its raw handle destroyed here
        unsafe {
            encoder.as_hal_mut::<A, _, _>(|encoder| {
                let encoder = encoder?;

                // Write-after-read against submissions still tracing this TLAS as their previous
                // frame, and against a BLAS compaction copy submitted ahead of this build. A
                // barrier's first scope covers everything already submitted, which covers both.
                encoder.place_acceleration_structure_barrier(hal::AccelerationStructureBarrier {
                    usage: hal::StateTransition {
                        from: hal::AccelerationStructureUses::SHADER_INPUT
                            | hal::AccelerationStructureUses::COPY_DST,
                        to: hal::AccelerationStructureUses::BUILD_OUTPUT
                            | hal::AccelerationStructureUses::BUILD_INPUT,
                    },
                });

                encoder.build_acceleration_structures(1, [descriptor]);

                // And read-after-write, for the passes later this frame that trace the result
                encoder.place_acceleration_structure_barrier(hal::AccelerationStructureBarrier {
                    usage: hal::StateTransition {
                        from: hal::AccelerationStructureUses::BUILD_OUTPUT,
                        to: hal::AccelerationStructureUses::SHADER_INPUT,
                    },
                });

                Some(())
            })
        }
    }
}

impl<A: hal::Api> RawTlasBackend for Hal<A> {
    fn scratch_size(
        &self,
        render_device: &RenderDevice,
        instances: &Buffer,
        instance_count: u32,
    ) -> Option<u64> {
        use hal::Device as _;

        // SAFETY: the handle is only read, and is dropped with the guard
        let hal_device = unsafe { render_device.wgpu_device().as_hal::<A>() }?;
        // SAFETY: the handle is only read, and is dropped with the guard
        let hal_instances = unsafe { instances.as_hal::<A>() }?;

        // Sizing ignores the buffer itself, but `wgpu_hal`'s Metal path unwraps it unconditionally,
        // so a real one has to be handed over
        let entries =
            hal::AccelerationStructureEntries::Instances(hal::AccelerationStructureInstances {
                buffer: Some(&*hal_instances),
                offset: 0,
                count: instance_count,
            });

        // SAFETY: `entries` describes a buffer belonging to backend `A`, and the query records
        // nothing and reads no memory through it
        let sizes = unsafe {
            hal_device.get_acceleration_structure_build_sizes(
                &hal::GetAccelerationStructureBuildSizesDescriptor {
                    entries: &entries,
                    flags: TLAS_BUILD_FLAGS,
                },
            )
        };

        Some(sizes.build_scratch_size)
    }

    fn create_scratch_buffer(&self, render_device: &RenderDevice, size: u64) -> Option<Buffer> {
        use hal::Device as _;

        let device = render_device.wgpu_device();
        let hal_buffer = {
            // SAFETY: the handle is only read, and is dropped with the guard
            let hal_device = unsafe { device.as_hal::<A>() }?;
            // SAFETY: the descriptor is valid and the buffer is handed to `wgpu-core` below, which
            // takes over destroying it
            unsafe {
                hal_device.create_buffer(&hal::BufferDescriptor {
                    label: Some("solari_tlas_scratch"),
                    size,
                    usage: BufferUses::ACCELERATION_STRUCTURE_SCRATCH,
                    memory_flags: hal::MemoryFlags::empty(),
                })
            }
            .ok()?
        };

        // `usage` is only consulted for wgpu-level validation, which this buffer never sees
        let descriptor = BufferDescriptor {
            label: Some("solari_tlas_scratch"),
            size,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        // SAFETY: `hal_buffer` was created from this device, matches `descriptor`, and has nonzero
        // size
        Some(unsafe { device.create_buffer_from_hal::<A>(hal_buffer, &descriptor) }.into())
    }

    fn build_tlas(
        &self,
        encoder: &mut CommandEncoder,
        tlas: &mut Tlas,
        instances: &Buffer,
        instance_count: u32,
        scratch: &Buffer,
    ) -> bool {
        if self
            .record_build(encoder, tlas, instances, instance_count, scratch)
            .is_none()
        {
            return false;
        }

        // Without this the build is invisible to `wgpu-core` and it rejects the TLAS when bound
        //
        // SAFETY: the TLAS was just built into this encoder, and every BLAS its instances point at
        // was built and submitted earlier in the frame by `prepare_raytracing_blas`
        unsafe {
            encoder.mark_acceleration_structures_built(core::iter::empty::<&Blas>(), [&*tlas]);
        }

        true
    }
}
