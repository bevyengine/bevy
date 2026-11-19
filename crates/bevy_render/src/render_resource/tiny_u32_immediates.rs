//! A small scalar compute parameter, passed as an immediate or an immutable uniform record.

use super::{
    binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayoutDescriptor,
    BindGroupLayoutEntries, BufferBinding, BufferInitDescriptor, BufferUsages, ComputePass,
    ComputePipelineDescriptor, ShaderStages, WgpuFeatures,
};
use crate::renderer::RenderDevice;
use core::num::NonZeroU64;

/// Passes small CPU-known `u32` values to compute shaders, with a uniform fallback.
///
/// The mode is fixed by the enabled features and limits of the render device.
/// Uniform records contain their own indices and are uploaded only on growth.
/// Pipelines using this helper reserve bind group 1 for the fallback.
///
/// # Memory usage
///
/// The uniform fallback is intended for small, densely packed values, such as mip
/// counts or sequential indices. It allocates a record for every value below the
/// reserved capacity, which is rounded up to a power of two and never shrinks.
/// Each record occupies `max(4, min_uniform_buffer_offset_alignment)` bytes, so
/// memory usage scales with the largest reserved value, not the number of values
/// actually passed to [`Self::set`].
///
/// For example, reserving `1_000_001` records to pass `1_000_000` with 256-byte
/// alignment allocates a 256 MiB GPU buffer, including 252 MiB of padding. Growing
/// the table also creates a temporary CPU array of the same size. The immediate
/// path does not allocate this table.
pub struct TinyU32Immediates {
    uniforms: Option<UniformRecords>,
}

struct UniformRecords {
    stride: u32,
    capacity: u32,
    bind_group: Option<BindGroup>,
}

impl TinyU32Immediates {
    /// Selects the parameter mode for this device. No GPU resources are allocated yet.
    pub fn new(device: &RenderDevice) -> Self {
        Self {
            uniforms: (!Self::immediates_available(device)).then(|| UniformRecords {
                stride: device.limits().min_uniform_buffer_offset_alignment.max(4),
                capacity: 0,
                bind_group: None,
            }),
        }
    }

    /// Whether the device can pass a four-byte immediate.
    pub fn immediates_available(device: &RenderDevice) -> bool {
        device.features().contains(WgpuFeatures::IMMEDIATES)
            && device.limits().max_immediate_size >= 4
    }

    /// Whether this parameter uses uniform records.
    pub fn is_uniform(&self) -> bool {
        self.uniforms.is_some()
    }

    /// Configures a pipeline's scalar immediate, leaving pipelines without immediates unchanged.
    pub fn configure_pipeline(&self, descriptor: &mut ComputePipelineDescriptor) {
        if self.is_uniform() && descriptor.immediate_size != 0 {
            assert_eq!(descriptor.layout.len(), 1);
            assert_eq!(descriptor.immediate_size, 4);
            descriptor.layout.push(Self::uniform_layout());
            descriptor.shader_defs.push("UNIFORM_IMMEDIATES".into());
            descriptor.immediate_size = 0;
        }
    }

    fn uniform_layout() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "u32 immediate uniform layout",
            &BindGroupLayoutEntries::single(ShaderStages::COMPUTE, uniform_buffer::<u32>(true)),
        )
    }

    /// Makes every value below `count` available. Call before recording consumers.
    /// Reallocating replaces the bind group, while previously recorded groups stay valid.
    pub fn reserve(&mut self, device: &RenderDevice, count: u32) {
        let Some(uniforms) = &mut self.uniforms else {
            return;
        };
        if count <= uniforms.capacity {
            return;
        }
        let capacity = count
            .checked_next_power_of_two()
            .expect("uniform record count fits u32");
        let size = u64::from(capacity) * u64::from(uniforms.stride);
        let mut contents = vec![0; size as usize];
        for (value, record) in contents
            .chunks_exact_mut(uniforms.stride as usize)
            .enumerate()
        {
            record[..4].copy_from_slice(&(value as u32).to_le_bytes());
        }
        let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("u32 immediate uniform records"),
            contents: &contents,
            usage: BufferUsages::UNIFORM,
        });
        let layout = Self::uniform_layout();
        let layout = device.create_bind_group_layout(Some(layout.label.as_ref()), &layout.entries);
        uniforms.bind_group = Some(device.create_bind_group(
            "u32 immediate uniform bind group",
            &layout,
            &BindGroupEntries::single(BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: NonZeroU64::new(4),
            }),
        ));
        uniforms.capacity = capacity;
    }

    /// Sets the value for the following dispatch. It must have been reserved first.
    #[inline]
    pub fn set(&self, pass: &mut ComputePass<'_>, value: u32) {
        if let Some(uniforms) = &self.uniforms {
            assert!(value < uniforms.capacity);
            pass.set_bind_group(
                1,
                uniforms.bind_group.as_ref().unwrap(),
                &[value * uniforms.stride],
            );
        } else {
            pass.set_immediates(0, &value.to_le_bytes());
        }
    }
}
