//! Utilities for combining [`AsBindGroup`] implementations

use alloc::borrow::Cow;

use bevy_ecs::system::SystemParamItem;
use bevy_platform::{collections::HashSet, hash::FixedHasher};
use wgpu::BindGroupLayoutEntry;

use crate::{
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupBuilder, BindGroupLayout, BindlessDescriptor,
        BindlessResourceType, BindlessSlabResourceLimit,
    },
    renderer::RenderDevice,
};

/// Combines [`AsBindGroup::bindless_slot_count`] implementations from base `B` and extension `E` by choosing the smallest count
pub fn bindless_slot_count<B: AsBindGroup, E: AsBindGroup>() -> Option<BindlessSlabResourceLimit> {
    // We only enable bindless if both the base material and its extension
    // are bindless. If we do enable bindless, we choose the smaller of the
    // two slab size limits.
    Some(combine_bindless_slot_count(
        B::bindless_slot_count()?,
        E::bindless_slot_count()?,
    ))
}

/// Combines [`AsBindGroup::bindless_supported`] implementations from base `B` and extension `E`
pub fn bindless_supported<B: AsBindGroup, E: AsBindGroup>(render_device: &RenderDevice) -> bool {
    bindless_slot_count::<B, E>().is_some()
        && B::bindless_supported(render_device)
        && E::bindless_supported(render_device)
}

/// Combines two [`BindlessSlabResourceLimit`]s, choosing the smallest limit
fn combine_bindless_slot_count(
    base: BindlessSlabResourceLimit,
    extension: BindlessSlabResourceLimit,
) -> BindlessSlabResourceLimit {
    match (base, extension) {
        (BindlessSlabResourceLimit::Auto, BindlessSlabResourceLimit::Auto) => {
            BindlessSlabResourceLimit::Auto
        }
        (BindlessSlabResourceLimit::Auto, BindlessSlabResourceLimit::Custom(limit))
        | (BindlessSlabResourceLimit::Custom(limit), BindlessSlabResourceLimit::Auto) => {
            BindlessSlabResourceLimit::Custom(limit)
        }
        (
            BindlessSlabResourceLimit::Custom(base_limit),
            BindlessSlabResourceLimit::Custom(extended_limit),
        ) => BindlessSlabResourceLimit::Custom(base_limit.min(extended_limit)),
    }
}

/// Combines [`AsBindGroup::bind_group_data`] implementations from base `B` and extension `E`
pub fn bind_group_data<B: AsBindGroup, E: AsBindGroup>(
    base: &B,
    extension: &E,
) -> CombinedBindGroupData<B::Data, E::Data> {
    CombinedBindGroupData {
        base: base.bind_group_data(),
        extension: extension.bind_group_data(),
    }
}

/// Combines [`AsBindGroup::build_bind_group`] implementations from base `B` and extension `E` by concatenating their bindings
pub fn build_bind_group<B: AsBindGroup, E: AsBindGroup>(
    base: &B,
    extension: &E,
    layout: &BindGroupLayout,
    render_device: &RenderDevice,
    (base_param, extended_param): &mut SystemParamItem<'_, '_, (B::Param, E::Param)>,
    mut force_no_bindless: bool,
    output: &mut BindGroupBuilder,
) -> Result<(), AsBindGroupError> {
    force_no_bindless = force_no_bindless || !bindless_supported::<B, E>(render_device);

    B::build_bind_group(
        base,
        layout,
        render_device,
        base_param,
        force_no_bindless,
        output,
    )?;
    E::build_bind_group(
        extension,
        layout,
        render_device,
        extended_param,
        force_no_bindless,
        output,
    )?;

    Ok(())
}

/// Combines [`AsBindGroup::bind_group_layout_entries`] implementations from base `B` and extension `E` by deduplicatinig identical bindings
pub fn bind_group_layout_entries<B: AsBindGroup, E: AsBindGroup>(
    render_device: &RenderDevice,
    mut force_no_bindless: bool,
) -> Vec<BindGroupLayoutEntry> {
    force_no_bindless = force_no_bindless || !bindless_supported::<B, E>(render_device);

    let base_entries = B::bind_group_layout_entries(render_device, force_no_bindless);
    let extension_entries = E::bind_group_layout_entries(render_device, force_no_bindless);

    combine_bind_group_layout_entries(base_entries, extension_entries)
}

/// Combines two sets of [`BindGroupLayoutEntry`]s, deduplicating identical bindings
pub fn combine_bind_group_layout_entries(
    base_entries: Vec<BindGroupLayoutEntry>,
    extension_entries: Vec<BindGroupLayoutEntry>,
) -> Vec<BindGroupLayoutEntry> {
    // Add together the bindings of the base material and the extension
    // material, skipping duplicate bindings. Duplicate bindings will occur
    // when bindless mode is on, because of the common bindless resource
    // arrays, and we need to eliminate the duplicates or `wgpu` will
    // complain.
    let mut seen_bindings = HashSet::<u32>::with_hasher(FixedHasher);

    base_entries
        .into_iter()
        .chain(extension_entries)
        .filter(|entry| seen_bindings.insert(entry.binding))
        .collect()
}

/// Combines [`AsBindGroup::bindless_descriptor`] implementations from base `B` and extension `E` by merging their contents together
pub fn bindless_descriptor<B: AsBindGroup, E: AsBindGroup>() -> Option<BindlessDescriptor> {
    let base_bindless_descriptor = B::bindless_descriptor()?;
    let extended_bindless_descriptor = E::bindless_descriptor()?;

    Some(combine_bindless_descriptors(
        base_bindless_descriptor,
        extended_bindless_descriptor,
    ))
}

/// Combines two [`BindlessDescriptor`]s by merging their contents together
pub fn combine_bindless_descriptors(
    base_bindless_descriptor: BindlessDescriptor,
    extended_bindless_descriptor: BindlessDescriptor,
) -> BindlessDescriptor {
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

    BindlessDescriptor {
        resources: Cow::Owned(resources),
        buffers: Cow::Owned(buffers),
        index_tables: Cow::Owned(index_tables),
    }
}

/// The [`AsBindGroup::Data`] used for combining two [`AsBindGroup`] implementations
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C, packed)]
pub struct CombinedBindGroupData<B, E> {
    pub base: B,
    pub extension: E,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_dummy_device;
    use wgpu::{BindingType, BufferBindingType, ShaderStages};

    fn dummy_entry(binding: u32, storage: bool) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: if storage {
                    BufferBindingType::Storage { read_only: true }
                } else {
                    BufferBindingType::Uniform
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    struct BindlessSupportedBase;
    impl AsBindGroup for BindlessSupportedBase {
        type Data = ();
        type Param = ();

        fn label() -> &'static str {
            "BindlessSupportedBase"
        }

        fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
            Some(BindlessSlabResourceLimit::Auto)
        }

        fn bindless_supported(_: &RenderDevice) -> bool {
            true
        }

        fn bind_group_data(&self) -> Self::Data {}

        fn build_bind_group(
            &self,
            _: &BindGroupLayout,
            render_device: &RenderDevice,
            _: &mut SystemParamItem<'_, '_, Self::Param>,
            force_no_bindless: bool,
            output: &mut BindGroupBuilder,
        ) -> Result<(), AsBindGroupError> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            output.data_buffer.push(u8::from(is_bindless));
            Ok(())
        }

        fn bind_group_layout_entries(
            render_device: &RenderDevice,
            force_no_bindless: bool,
        ) -> Vec<BindGroupLayoutEntry> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            vec![dummy_entry(0, is_bindless)]
        }
    }

    struct BindlessSupportedExt;
    impl AsBindGroup for BindlessSupportedExt {
        type Data = ();
        type Param = ();

        fn label() -> &'static str {
            "BindlessSupportedExt"
        }

        fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
            Some(BindlessSlabResourceLimit::Auto)
        }

        fn bindless_supported(_: &RenderDevice) -> bool {
            true
        }

        fn bind_group_data(&self) -> Self::Data {}

        fn build_bind_group(
            &self,
            _: &BindGroupLayout,
            render_device: &RenderDevice,
            _: &mut SystemParamItem<'_, '_, Self::Param>,
            force_no_bindless: bool,
            output: &mut BindGroupBuilder,
        ) -> Result<(), AsBindGroupError> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            output.data_buffer.push(u8::from(is_bindless));
            Ok(())
        }

        fn bind_group_layout_entries(
            render_device: &RenderDevice,
            force_no_bindless: bool,
        ) -> Vec<BindGroupLayoutEntry> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            vec![dummy_entry(100, is_bindless)]
        }
    }

    struct BindlessUnsupportedExt;
    impl AsBindGroup for BindlessUnsupportedExt {
        type Data = ();
        type Param = ();

        fn label() -> &'static str {
            "BindlessUnsupportedExt"
        }

        fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
            Some(BindlessSlabResourceLimit::Auto)
        }

        fn bindless_supported(_: &RenderDevice) -> bool {
            false
        }

        fn bind_group_data(&self) -> Self::Data {}

        fn build_bind_group(
            &self,
            _: &BindGroupLayout,
            render_device: &RenderDevice,
            _: &mut SystemParamItem<'_, '_, Self::Param>,
            force_no_bindless: bool,
            output: &mut BindGroupBuilder,
        ) -> Result<(), AsBindGroupError> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            output.data_buffer.push(u8::from(is_bindless));
            Ok(())
        }

        fn bind_group_layout_entries(
            render_device: &RenderDevice,
            force_no_bindless: bool,
        ) -> Vec<BindGroupLayoutEntry> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            vec![dummy_entry(100, is_bindless)]
        }
    }

    struct NonBindlessExt;
    impl AsBindGroup for NonBindlessExt {
        type Data = ();
        type Param = ();

        fn label() -> &'static str {
            "NonBindlessExt"
        }

        fn bind_group_data(&self) -> Self::Data {}

        fn build_bind_group(
            &self,
            _: &BindGroupLayout,
            render_device: &RenderDevice,
            _: &mut SystemParamItem<'_, '_, Self::Param>,
            force_no_bindless: bool,
            output: &mut BindGroupBuilder,
        ) -> Result<(), AsBindGroupError> {
            let is_bindless = Self::bindless_supported(render_device) && !force_no_bindless;
            output.data_buffer.push(u8::from(is_bindless));
            Ok(())
        }

        fn bind_group_layout_entries(_: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry> {
            vec![dummy_entry(100, false)]
        }
    }

    #[test]
    fn combined_bindless_support_and_layout_fallback() {
        let (device, _queue) = create_dummy_device();

        // Both base and extension support bindless -> bindless is used for both.
        assert!(bindless_supported::<
            BindlessSupportedBase,
            BindlessSupportedExt,
        >(&device));
        let entries = bind_group_layout_entries::<BindlessSupportedBase, BindlessSupportedExt>(
            &device, false,
        );
        assert_eq!(entries, vec![dummy_entry(0, true), dummy_entry(100, true)]);

        // Extension declares bindless slots, but hardware support is false ->
        // combined material must report bindless_supported == false and force
        // the base material to fall back to non-bindless layout entries.
        assert!(!bindless_supported::<
            BindlessSupportedBase,
            BindlessUnsupportedExt,
        >(&device));
        let entries = bind_group_layout_entries::<BindlessSupportedBase, BindlessUnsupportedExt>(
            &device, false,
        );
        assert_eq!(
            entries,
            vec![dummy_entry(0, false), dummy_entry(100, false)]
        );

        // Extension does not use bindless at all (bindless_slot_count is None,
        // default bindless_supported is true) -> combined material must report
        // bindless_supported == false and fall back to non-bindless.
        assert!(!bindless_supported::<BindlessSupportedBase, NonBindlessExt>(&device));
        let entries =
            bind_group_layout_entries::<BindlessSupportedBase, NonBindlessExt>(&device, false);
        assert_eq!(
            entries,
            vec![dummy_entry(0, false), dummy_entry(100, false)]
        );

        let layout = device.create_bind_group_layout("test_layout", &[]);
        let mut param = ((), ());

        let mut builder = BindGroupBuilder::default();
        build_bind_group(
            &BindlessSupportedBase,
            &BindlessSupportedExt,
            &layout,
            &device,
            &mut param,
            false,
            &mut builder,
        )
        .unwrap();
        assert_eq!(builder.data_buffer, vec![1, 1]);

        let mut builder = BindGroupBuilder::default();
        build_bind_group(
            &BindlessSupportedBase,
            &BindlessUnsupportedExt,
            &layout,
            &device,
            &mut param,
            false,
            &mut builder,
        )
        .unwrap();
        assert_eq!(builder.data_buffer, vec![0, 0]);

        let mut builder = BindGroupBuilder::default();
        build_bind_group(
            &BindlessSupportedBase,
            &NonBindlessExt,
            &layout,
            &device,
            &mut param,
            false,
            &mut builder,
        )
        .unwrap();
        assert_eq!(builder.data_buffer, vec![0, 0]);
    }
}
