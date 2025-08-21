use alloc::borrow::Cow;

use bevy_asset::Asset;
use bevy_ecs::system::SystemParamItem;
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_platform::{collections::HashSet, hash::FixedHasher};
use bevy_reflect::{impl_type_path, Reflect};
use bevy_render::{
    alpha::AlphaMode,
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupLayout, BindGroupLayoutEntry, BindlessDescriptor,
        BindlessResourceType, BindlessSlabResourceLimit, RenderPipelineDescriptor,
        SpecializedMeshPipelineError, UnpreparedBindGroup,
    },
    renderer::RenderDevice,
};
use bevy_shader::ShaderRef;

use crate::{Material, MaterialPipeline, MaterialPipelineKey, MeshPipeline, MeshPipelineKey};

pub struct MaterialExtensionPipeline {
    pub mesh_pipeline: MeshPipeline,
}

pub struct MaterialExtensionKey<E: MaterialExtension> {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: E::Data,
}

/// A subset of the `Material` trait for defining extensions to a base `Material`, such as the builtin `StandardMaterial`.
///
/// A user type implementing the trait should be used as the `E` generic param in an `ExtendedMaterial` struct.
pub trait MaterialExtension: Asset + AsBindGroup + Clone + Sized {
    /// Returns this material's vertex shader. If [`ShaderRef::Default`] is returned, the base material mesh vertex shader
    /// will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the base material mesh fragment shader
    /// will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    // Returns this material’s AlphaMode. If None is returned, the base material alpha mode will be used.
    fn alpha_mode() -> Option<AlphaMode> {
        None
    }

    /// Returns this material's prepass vertex shader. If [`ShaderRef::Default`] is returned, the base material prepass vertex shader
    /// will be used.
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the base material prepass fragment shader
    /// will be used.
    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's deferred vertex shader. If [`ShaderRef::Default`] is returned, the base material deferred vertex shader
    /// will be used.
    fn deferred_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the base material deferred fragment shader
    /// will be used.
    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh fragment shader will be used.
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] prepass fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh prepass fragment shader will be used.
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] deferred fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh deferred fragment shader will be used.
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayoutRef`] as input.
    /// Specialization for the base material is applied before this function is called.
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    #[inline]
    fn specialize(
        pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// A material that extends a base [`Material`] with additional shaders and data.
///
/// The data from both materials will be combined and made available to the shader
/// so that shader functions built for the base material (and referencing the base material
/// bindings) will work as expected, and custom alterations based on custom data can also be used.
///
/// If the extension `E` returns a non-default result from `vertex_shader()` it will be used in place of the base
/// material's vertex shader.
///
/// If the extension `E` returns a non-default result from `fragment_shader()` it will be used in place of the base
/// fragment shader.
///
/// When used with `StandardMaterial` as the base, all the standard material fields are
/// present, so the `pbr_fragment` shader functions can be called from the extension shader (see
/// the `extended_material` example).
#[derive(Asset, Clone, Debug, Reflect)]
#[reflect(type_path = false)]
#[reflect(Clone)]
pub struct ExtendedMaterial<B: Material, E: MaterialExtension> {
    pub base: B,
    pub extension: E,
}

impl<B, E> Default for ExtendedMaterial<B, E>
where
    B: Material + Default,
    E: MaterialExtension + Default,
{
    fn default() -> Self {
        Self {
            base: B::default(),
            extension: E::default(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C, packed)]
pub struct MaterialExtensionBindGroupData<B, E> {
    pub base: B,
    pub extension: E,
}

// We don't use the `TypePath` derive here due to a bug where `#[reflect(type_path = false)]`
// causes the `TypePath` derive to not generate an implementation.
impl_type_path!((in bevy_pbr::extended_material) ExtendedMaterial<B: Material, E: MaterialExtension>);

impl<B: Material, E: MaterialExtension> AsBindGroup for ExtendedMaterial<B, E> {
    type Data = MaterialExtensionBindGroupData<B::Data, E::Data>;
    type Param = (<B as AsBindGroup>::Param, <E as AsBindGroup>::Param);

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

    fn bind_group_data(&self) -> Self::Data {
        MaterialExtensionBindGroupData {
            base: self.base.bind_group_data(),
            extension: self.extension.bind_group_data(),
        }
    }

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (base_param, extended_param): &mut SystemParamItem<'_, '_, Self::Param>,
        mut force_non_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        force_non_bindless = force_non_bindless || Self::bindless_slot_count().is_none();

        // add together the bindings of the base material and the user material
        let UnpreparedBindGroup { mut bindings } = B::unprepared_bind_group(
            &self.base,
            layout,
            render_device,
            base_param,
            force_non_bindless,
        )?;
        let extended_bindgroup = E::unprepared_bind_group(
            &self.extension,
            layout,
            render_device,
            extended_param,
            force_non_bindless,
        )?;

        bindings.extend(extended_bindgroup.bindings.0);

        Ok(UnpreparedBindGroup { bindings })
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
        mut force_non_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        force_non_bindless = force_non_bindless || Self::bindless_slot_count().is_none();

        // Add together the bindings of the standard material and the user
        // material, skipping duplicate bindings. Duplicate bindings will occur
        // when bindless mode is on, because of the common bindless resource
        // arrays, and we need to eliminate the duplicates or `wgpu` will
        // complain.
        let mut entries = vec![];
        let mut seen_bindings = HashSet::<_>::with_hasher(FixedHasher);
        for entry in B::bind_group_layout_entries(render_device, force_non_bindless)
            .into_iter()
            .chain(E::bind_group_layout_entries(render_device, force_non_bindless).into_iter())
        {
            if seen_bindings.insert(entry.binding) {
                entries.push(entry);
            }
        }
        entries
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

impl<B: Material, E: MaterialExtension> Material for ExtendedMaterial<B, E> {
    fn vertex_shader() -> ShaderRef {
        match E::vertex_shader() {
            ShaderRef::Default => B::vertex_shader(),
            specified => specified,
        }
    }

    fn fragment_shader() -> ShaderRef {
        match E::fragment_shader() {
            ShaderRef::Default => B::fragment_shader(),
            specified => specified,
        }
    }

    fn alpha_mode(&self) -> AlphaMode {
        match E::alpha_mode() {
            Some(specified) => specified,
            None => B::alpha_mode(&self.base),
        }
    }

    fn opaque_render_method(&self) -> crate::OpaqueRendererMethod {
        B::opaque_render_method(&self.base)
    }

    fn depth_bias(&self) -> f32 {
        B::depth_bias(&self.base)
    }

    fn reads_view_transmission_texture(&self) -> bool {
        B::reads_view_transmission_texture(&self.base)
    }

    fn prepass_vertex_shader() -> ShaderRef {
        match E::prepass_vertex_shader() {
            ShaderRef::Default => B::prepass_vertex_shader(),
            specified => specified,
        }
    }

    fn prepass_fragment_shader() -> ShaderRef {
        match E::prepass_fragment_shader() {
            ShaderRef::Default => B::prepass_fragment_shader(),
            specified => specified,
        }
    }

    fn deferred_vertex_shader() -> ShaderRef {
        match E::deferred_vertex_shader() {
            ShaderRef::Default => B::deferred_vertex_shader(),
            specified => specified,
        }
    }

    fn deferred_fragment_shader() -> ShaderRef {
        match E::deferred_fragment_shader() {
            ShaderRef::Default => B::deferred_fragment_shader(),
            specified => specified,
        }
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_fragment_shader() -> ShaderRef {
        match E::meshlet_mesh_fragment_shader() {
            ShaderRef::Default => B::meshlet_mesh_fragment_shader(),
            specified => specified,
        }
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_prepass_fragment_shader() -> ShaderRef {
        match E::meshlet_mesh_prepass_fragment_shader() {
            ShaderRef::Default => B::meshlet_mesh_prepass_fragment_shader(),
            specified => specified,
        }
    }

    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        match E::meshlet_mesh_deferred_fragment_shader() {
            ShaderRef::Default => B::meshlet_mesh_deferred_fragment_shader(),
            specified => specified,
        }
    }

    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Call the base material's specialize function
        let base_key = MaterialPipelineKey::<B> {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.base,
        };
        B::specialize(pipeline, descriptor, layout, base_key)?;

        // Call the extended material's specialize function afterwards
        E::specialize(
            &MaterialExtensionPipeline {
                mesh_pipeline: pipeline.mesh_pipeline.clone(),
            },
            descriptor,
            layout,
            MaterialExtensionKey {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data.extension,
            },
        )
    }
}
