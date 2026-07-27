use alloc::borrow::Cow;

use bevy_ecs::system::SystemParamItem;
use bevy_platform::{collections::HashSet, hash::FixedHasher};
use wgpu::BindGroupLayoutEntry;

use crate::{
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupLayout, BindlessDescriptor, BindlessResourceType,
        BindlessSlabResourceLimit, UnpreparedBindGroup,
    },
    renderer::RenderDevice,
};

/// Combines the [`AsBindGroup`] implementations from base `B` and extension `E`
///
/// This does not combine the [`as_bind_group`](AsBindGroup::as_bind_group), [`bind_group_layout`](AsBindGroup::bind_group_layout),
/// and [`bind_group_layout_descriptor`](AsBindGroup::bind_group_layout_descriptor) functions,
/// instead relying on the default implementations from [`AsBindGroup`]
pub struct CombinedBindGroup<'a, B, E> {
    pub base: &'a B,
    pub extension: &'a E,
}

impl<'a, B: AsBindGroup, E: AsBindGroup> AsBindGroup for CombinedBindGroup<'a, B, E> {
    type Data = CombinedBindGroupData<B::Data, E::Data>;
    type Param = (B::Param, E::Param);

    fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
        // We only enable bindless if both the base material and its extension
        // are bindless. If we do enable bindless, we choose the smaller of the
        // two slab size limits.
        match (B::bindless_slot_count()?, E::bindless_slot_count()?) {
            (BindlessSlabResourceLimit::Auto, BindlessSlabResourceLimit::Auto) => {
                Some(BindlessSlabResourceLimit::Auto)
            }
            (BindlessSlabResourceLimit::Auto, BindlessSlabResourceLimit::Custom(limit))
            | (BindlessSlabResourceLimit::Custom(limit), BindlessSlabResourceLimit::Auto) => {
                Some(BindlessSlabResourceLimit::Custom(limit))
            }
            (
                BindlessSlabResourceLimit::Custom(base_limit),
                BindlessSlabResourceLimit::Custom(extended_limit),
            ) => Some(BindlessSlabResourceLimit::Custom(
                base_limit.min(extended_limit),
            )),
        }
    }

    fn bindless_supported(render_device: &RenderDevice) -> bool {
        B::bindless_supported(render_device) && E::bindless_supported(render_device)
    }

    fn label() -> &'static str {
        E::label()
    }

    fn bind_group_data(&self) -> Self::Data {
        CombinedBindGroupData {
            base: self.base.bind_group_data(),
            extension: self.extension.bind_group_data(),
        }
    }

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (base_param, extended_param): &mut SystemParamItem<'_, '_, Self::Param>,
        mut force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        force_no_bindless = force_no_bindless || Self::bindless_slot_count().is_none();

        // add together the bindings of the base material and the extension
        let UnpreparedBindGroup { mut bindings } = B::unprepared_bind_group(
            &self.base,
            layout,
            render_device,
            base_param,
            force_no_bindless,
        )?;
        let UnpreparedBindGroup {
            bindings: extension_bindings,
        } = E::unprepared_bind_group(
            &self.extension,
            layout,
            render_device,
            extended_param,
            force_no_bindless,
        )?;

        bindings.extend(extension_bindings.0);

        Ok(UnpreparedBindGroup { bindings })
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
        mut force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        force_no_bindless = force_no_bindless || Self::bindless_slot_count().is_none();

        // Add together the bindings of the standard material and the user
        // material, skipping duplicate bindings. Duplicate bindings will occur
        // when bindless mode is on, because of the common bindless resource
        // arrays, and we need to eliminate the duplicates or `wgpu` will
        // complain.
        let base_entries = B::bind_group_layout_entries(render_device, force_no_bindless);
        let extension_entries = E::bind_group_layout_entries(render_device, force_no_bindless);

        let mut seen_bindings = HashSet::<u32>::with_hasher(FixedHasher);

        base_entries
            .into_iter()
            .chain(extension_entries)
            .filter(|entry| seen_bindings.insert(entry.binding))
            .collect()
    }

    fn bindless_descriptor() -> Option<BindlessDescriptor> {
        // We're going to combine the two bindless descriptors.
        let base_bindless_descriptor = B::bindless_descriptor()?;
        let extended_bindless_descriptor = E::bindless_descriptor()?;

        // Combining the buffers and index tables is straightforward.

        let mut buffers = base_bindless_descriptor.buffers.to_vec();
        let mut index_tables = base_bindless_descriptor.index_tables.to_vec();

        buffers.extend(extended_bindless_descriptor.buffers.iter().cloned());
        index_tables.extend(extended_bindless_descriptor.index_tables.iter().cloned());

        // Combining the resources is a little trickier because the resource
        // array is indexed by bindless index, so we have to merge the two
        // arrays, not just concatenate them.
        let max_bindless_index = base_bindless_descriptor
            .resources
            .len()
            .max(extended_bindless_descriptor.resources.len());
        let mut resources = Vec::with_capacity(max_bindless_index);
        for bindless_index in 0..max_bindless_index {
            // In the event of a conflicting bindless index, we choose the
            // base's binding.
            match base_bindless_descriptor.resources.get(bindless_index) {
                None | Some(&BindlessResourceType::None) => resources.push(
                    extended_bindless_descriptor
                        .resources
                        .get(bindless_index)
                        .copied()
                        .unwrap_or(BindlessResourceType::None),
                ),
                Some(&resource_type) => resources.push(resource_type),
            }
        }

        Some(BindlessDescriptor {
            resources: Cow::Owned(resources),
            buffers: Cow::Owned(buffers),
            index_tables: Cow::Owned(index_tables),
        })
    }
}

/// The [`AsBindGroup::Data`] used by [`CombinedBindGroup`]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C, packed)]
pub struct CombinedBindGroupData<B, E> {
    pub base: B,
    pub extension: E,
}
