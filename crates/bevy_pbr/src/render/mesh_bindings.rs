//! Bind group layout related definitions for the mesh pipeline.

use std::{
    array,
    fmt::{self, Display, Formatter},
};

use bevy_math::Mat4;
use bevy_render::{
    mesh::morph::MAX_MORPH_WEIGHTS,
    render_resource::{
        BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, Buffer, TextureView,
    },
    renderer::RenderDevice,
};
use bitflags::bitflags;
use smallvec::SmallVec;

use crate::render::skin::MAX_JOINTS;

const MORPH_WEIGHT_SIZE: usize = std::mem::size_of::<f32>();
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
pub(crate) const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub(crate) struct MeshLayoutKey: u8 {
        const SKINNED = 1;
        const MORPHED = 2;
    }
}

const MESH_LAYOUT_COUNT: usize = MeshLayoutKey::all().bits() as usize + 1;

/// Individual layout entries.
mod layout_entry {
    use super::{JOINT_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use crate::MeshUniform;
    use bevy_render::{
        render_resource::{
            BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, GpuArrayBuffer,
            ShaderStages, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
    };

    fn buffer(binding: u32, size: u64, visibility: ShaderStages) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            count: None,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: BufferSize::new(size),
            },
        }
    }
    pub(super) fn model(render_device: &RenderDevice, binding: u32) -> BindGroupLayoutEntry {
        GpuArrayBuffer::<MeshUniform>::binding_layout(
            binding,
            ShaderStages::VERTEX_FRAGMENT,
            render_device,
        )
    }
    pub(super) fn skinning(binding: u32) -> BindGroupLayoutEntry {
        buffer(binding, JOINT_BUFFER_SIZE as u64, ShaderStages::VERTEX)
    }
    pub(super) fn weights(binding: u32) -> BindGroupLayoutEntry {
        buffer(binding, MORPH_BUFFER_SIZE as u64, ShaderStages::VERTEX)
    }
    pub(super) fn targets(binding: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Texture {
                view_dimension: TextureViewDimension::D3,
                sample_type: TextureSampleType::Float { filterable: false },
                multisampled: false,
            },
            count: None,
        }
    }
}
/// Individual [`BindGroupEntry`](bevy_render::render_resource::BindGroupEntry)
/// for bind groups.
mod entry {
    use super::{JOINT_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use bevy_render::render_resource::{
        BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferSize, TextureView,
    };

    fn entry(binding: u32, size: u64, buffer: &Buffer) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: BindingResource::Buffer(BufferBinding {
                buffer,
                offset: 0,
                size: Some(BufferSize::new(size).unwrap()),
            }),
        }
    }
    pub(super) fn model(binding: u32, resource: BindingResource) -> BindGroupEntry {
        BindGroupEntry { binding, resource }
    }
    pub(super) fn skinning(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        entry(binding, JOINT_BUFFER_SIZE as u64, buffer)
    }
    pub(super) fn weights(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        entry(binding, MORPH_BUFFER_SIZE as u64, buffer)
    }
    pub(super) fn targets(binding: u32, texture: &TextureView) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: BindingResource::TextureView(texture),
        }
    }
}

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wgsl`).
#[derive(Clone)]
pub struct MeshLayouts([BindGroupLayout; MESH_LAYOUT_COUNT]);

impl MeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new(render_device: &RenderDevice) -> Self {
        MeshLayouts(array::from_fn(|mesh_layout_bitmask| {
            Self::create_layout(
                render_device,
                MeshLayoutKey::from_bits_truncate(mesh_layout_bitmask as u8),
            )
        }))
    }

    /// Creates an individual bind group layout.
    fn create_layout(
        render_device: &RenderDevice,
        mesh_layout_key: MeshLayoutKey,
    ) -> BindGroupLayout {
        let mut entries: SmallVec<[BindGroupLayoutEntry; 4]> = SmallVec::new();
        entries.push(layout_entry::model(render_device, 0));

        if mesh_layout_key.contains(MeshLayoutKey::SKINNED) {
            entries.push(layout_entry::skinning(1));
        }
        if mesh_layout_key.contains(MeshLayoutKey::MORPHED) {
            entries.push(layout_entry::weights(2));
            entries.push(layout_entry::targets(3));
        }

        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&*format!("mesh_bind_group_layout_{}", mesh_layout_key)),
            entries: &entries,
        })
    }

    pub(crate) fn get_layout(&self, key: MeshLayoutKey) -> &BindGroupLayout {
        &self.0[key.bits() as usize]
    }

    fn populate_generic_bind_group_entries<'vec, 'entry>(
        &self,
        bind_group_entries: &'vec mut SmallVec<[BindGroupEntry<'entry>; 4]>,
        model: &'entry BindingResource,
        skin: Option<&'entry Buffer>,
        key: MeshLayoutKey,
    ) {
        bind_group_entries.push(entry::model(0, (*model).clone()));

        if key.contains(MeshLayoutKey::SKINNED) {
            bind_group_entries.push(entry::skinning(1, skin.unwrap()));
        }
    }

    // Creates a bind group that isn't associated with a specific mesh.
    pub(crate) fn create_generic_bind_group(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: Option<&Buffer>,
        key: MeshLayoutKey,
    ) -> BindGroup {
        debug_assert!(!key.bind_group_is_mesh_specific());

        let mut bind_group_entries: SmallVec<[BindGroupEntry; 4]> = SmallVec::new();
        self.populate_generic_bind_group_entries(&mut bind_group_entries, model, skin, key);
        render_device.create_bind_group(
            Some(&*format!("mesh_bind_group_{}", key)),
            self.get_layout(key),
            &bind_group_entries,
        )
    }

    // Creates a bind group that needs to be associated with a specific mesh.
    pub(crate) fn create_mesh_specific_bind_group(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: Option<&Buffer>,
        morph: Option<(&Buffer, &TextureView)>,
        key: MeshLayoutKey,
    ) -> BindGroup {
        debug_assert!(key.bind_group_is_mesh_specific());

        let mut bind_group_entries: SmallVec<[BindGroupEntry; 4]> = SmallVec::new();
        self.populate_generic_bind_group_entries(&mut bind_group_entries, model, skin, key);

        if key.contains(MeshLayoutKey::MORPHED) {
            let (weights, targets) = morph.unwrap();
            bind_group_entries.push(entry::weights(2, weights));
            bind_group_entries.push(entry::targets(3, targets));
        }

        render_device.create_bind_group(
            Some(&*format!("mesh_bind_group_{}", key)),
            self.get_layout(key),
            &bind_group_entries,
        )
    }
}

impl MeshLayoutKey {
    pub fn bind_group_is_mesh_specific(&self) -> bool {
        self.intersects(MeshLayoutKey::MORPHED)
    }
}

impl Display for MeshLayoutKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut first = true;
        add(
            formatter,
            &mut first,
            *self,
            MeshLayoutKey::SKINNED,
            "skinned",
        )?;
        add(
            formatter,
            &mut first,
            *self,
            MeshLayoutKey::MORPHED,
            "morphed",
        )?;
        return Ok(());

        fn add(
            formatter: &mut Formatter<'_>,
            first: &mut bool,
            key: MeshLayoutKey,
            flag: MeshLayoutKey,
            name: &str,
        ) -> fmt::Result {
            if !key.contains(flag) {
                return Ok(());
            }
            if !*first {
                formatter.write_str("_")?;
            }
            formatter.write_str(name)?;
            *first = false;
            Ok(())
        }
    }
}
