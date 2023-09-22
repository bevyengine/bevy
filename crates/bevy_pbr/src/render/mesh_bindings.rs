//! Bind group layout related definitions for the mesh pipeline.
use std::{array, fmt};

use bevy_asset::AssetId;
use bevy_ecs::prelude::Resource;
use bevy_math::Mat4;
use bevy_render::mesh::{morph::MAX_MORPH_WEIGHTS, Mesh};
use bevy_render::{render_resource::*, renderer::RenderDevice};
use bevy_utils::HashMap;
use bitflags::bitflags;
use smallvec::SmallVec;

use crate::render::skin::MAX_JOINTS;
use crate::MeshUniform;

const MORPH_WEIGHT_SIZE: usize = std::mem::size_of::<f32>();
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
pub(crate) const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

// --- Individual layout entries and individual bind group entries ---

fn buffer_layout(binding: u32, size: u64, visibility: ShaderStages) -> BindGroupLayoutEntry {
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
fn buffer_entry(binding: u32, size: u64, buffer: &Buffer) -> BindGroupEntry {
    BindGroupEntry {
        binding,
        resource: BindingResource::Buffer(BufferBinding {
            buffer,
            offset: 0,
            size: Some(BufferSize::new(size).unwrap()),
        }),
    }
}

fn model_layout(render_device: &RenderDevice, binding: u32) -> BindGroupLayoutEntry {
    GpuArrayBuffer::<MeshUniform>::binding_layout(
        binding,
        ShaderStages::VERTEX_FRAGMENT,
        render_device,
    )
}
fn model_entry(binding: u32, resource: BindingResource) -> BindGroupEntry {
    BindGroupEntry { binding, resource }
}

fn skinning_layout(binding: u32) -> BindGroupLayoutEntry {
    buffer_layout(binding, JOINT_BUFFER_SIZE as u64, ShaderStages::VERTEX)
}
fn skinning_entry(binding: u32, buffer: &Buffer) -> BindGroupEntry {
    buffer_entry(binding, JOINT_BUFFER_SIZE as u64, buffer)
}

fn weights_layout(binding: u32) -> BindGroupLayoutEntry {
    buffer_layout(binding, MORPH_BUFFER_SIZE as u64, ShaderStages::VERTEX)
}
fn weights_entry(binding: u32, buffer: &Buffer) -> BindGroupEntry {
    buffer_entry(binding, MORPH_BUFFER_SIZE as u64, buffer)
}

fn targets_layout(binding: u32) -> BindGroupLayoutEntry {
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
fn targets_entry(binding: u32, texture: &TextureView) -> BindGroupEntry {
    BindGroupEntry {
        binding,
        resource: BindingResource::TextureView(texture),
    }
}

/// Create a layout for a specific [`ActiveVariant`] by combining the layouts of
/// all the active shader features.
fn variant_layout(active_variant: ActiveVariant, device: &RenderDevice) -> BindGroupLayout {
    let skin = active_variant.contains(ActiveVariant::SKIN);
    let morph = active_variant.contains(ActiveVariant::MORPH);
    let motion_vectors = active_variant.contains(ActiveVariant::MOTION_VECTORS);

    let mut layout_entries = SmallVec::<[BindGroupLayoutEntry; 6]>::new();

    layout_entries.push(model_layout(device, 0));

    if skin {
        layout_entries.push(skinning_layout(1));
    }
    if morph {
        layout_entries.extend([weights_layout(2), targets_layout(3)]);
    }
    if skin && motion_vectors {
        layout_entries.push(skinning_layout(4));
    }
    if morph && motion_vectors {
        layout_entries.push(weights_layout(5));
    }

    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &layout_entries,
        label: Some(&*format!("{active_variant}_layout")),
    })
}

/// Create [`BindGroup`]s and add them to a [`MeshBindGroups`].
///
/// Use [`MeshBindGroups::bind_group_builder`] to get a [`MeshBindGroupBuilder`],
/// and use [`MeshBindGroupBuilder::add_variant`] to add new bind group variants.
pub struct MeshBindGroupBuilder<'a> {
    layouts: &'a MeshLayouts,
    device: &'a RenderDevice,
    model: BindingResource<'a>,
    pub bind_groups: &'a mut MeshBindGroups,
}
impl<'a> MeshBindGroupBuilder<'a> {
    /// Create a [`BindGroup`] with the provided active features, and add it to [`Self::bind_groups`].
    ///
    /// Each parameter to `add_variant` is a shader feature. When the parameter
    /// is `None`, it means that the shader feature is disabled.
    pub fn add_variant(
        &mut self,
        skin: Option<&Buffer>,
        morph: Option<(AssetId<Mesh>, &Buffer, &TextureView)>,
        previous_skin: Option<&Buffer>,
        previous_weights: Option<&Buffer>,
    ) {
        let mut bind_group_entries = SmallVec::<[BindGroupEntry; 6]>::new();

        bind_group_entries.push(model_entry(0, self.model.clone()));

        if let Some(skin) = skin {
            bind_group_entries.push(skinning_entry(1, skin));
        }
        if let Some((_, weights, targets)) = morph {
            bind_group_entries.extend([weights_entry(2, weights), targets_entry(3, targets)]);
        }
        if let Some(skin) = previous_skin {
            bind_group_entries.push(skinning_entry(4, skin));
        }
        if let Some(weights) = previous_weights {
            bind_group_entries.push(weights_entry(5, weights));
        }

        let skin = skin.is_some();
        let motion_vectors = previous_skin.is_some() || previous_weights.is_some();
        let active_variant = ActiveVariant::new(skin, morph.is_some(), motion_vectors);

        let bind_group = self.device.create_bind_group(
            Some(&*format!("{active_variant}_bind_group")),
            self.layouts.get_variant(active_variant),
            &bind_group_entries,
        );
        self.bind_groups
            .insert(skin, morph.map(|m| m.0), motion_vectors, bind_group);
    }
}

/// The [`BindGroup`]s for individual existing mesh shader variants.
///
/// Morph targets allow several different bind groups, because individual mesh
/// may have a different [`TextureView`] that represents the morph target's pose
/// vertex attribute values.
///
/// Non-morph target bind groups are optional. We don't know at compile time
/// whether motion vectors or skinned meshes will be used.
#[derive(Default, Resource)]
pub struct MeshBindGroups {
    shared: HashMap<ActiveVariant, BindGroup>,
    distinct: HashMap<(AssetId<Mesh>, ActiveVariant), BindGroup>,
}

impl MeshBindGroups {
    pub fn new() -> Self {
        MeshBindGroups::default()
    }
    /// Get a specific [`BindGroup`] that was previously added.
    pub fn get(
        &self,
        skin: bool,
        morph_id: Option<AssetId<Mesh>>,
        motion_vectors: bool,
    ) -> Option<&BindGroup> {
        let variant = ActiveVariant::new(skin, morph_id.is_some(), motion_vectors);
        if let Some(id) = morph_id {
            self.distinct.get(&(id, variant))
        } else {
            self.shared.get(&variant)
        }
    }
    fn insert(
        &mut self,
        skin: bool,
        morph_id: Option<AssetId<Mesh>>,
        motion_vectors: bool,
        bind_group: BindGroup,
    ) {
        let variant = ActiveVariant::new(skin, morph_id.is_some(), motion_vectors);

        if let Some(id) = morph_id {
            self.distinct.insert((id, variant), bind_group);
        } else {
            self.shared.insert(variant, bind_group);
        }
    }
    pub fn clear(&mut self) {
        self.shared.clear();
        self.distinct.clear();
    }
    /// Clears `self` and returns a [`MeshBindGroupBuilder`].
    pub fn bind_group_builder<'a>(
        &'a mut self,
        device: &'a RenderDevice,
        model: BindingResource<'a>,
        layouts: &'a MeshLayouts,
    ) -> MeshBindGroupBuilder<'a> {
        self.clear();

        MeshBindGroupBuilder {
            layouts,
            device,
            model,
            bind_groups: self,
        }
    }
}

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wgsl`).
#[derive(Clone)]
pub struct MeshLayouts([BindGroupLayout; ActiveVariant::COUNT]);

impl MeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new(device: &RenderDevice) -> Self {
        let layout = |bits| variant_layout(ActiveVariant::from_bits_truncate(bits), device);
        MeshLayouts(array::from_fn(layout))
    }
    // TODO: Passing 3 bools is pretty bad
    pub fn get(&self, skin: bool, morph: bool, motion_vectors: bool) -> &BindGroupLayout {
        let variant = ActiveVariant::new(skin, morph, motion_vectors);
        self.get_variant(variant)
    }
    fn get_variant(&self, active_variant: ActiveVariant) -> &BindGroupLayout {
        &self.0[active_variant.bits()]
    }
}

bitflags! {
    /// The set of active features for a given mesh shader instance.
    ///
    /// Individual meshes may have different features. For example,
    /// one mesh may have morph targets, another skinning.
    ///
    /// Each of those features enable different shader code and bind groups through
    /// the naga C-like pre-processor (`#ifdef FOO`).
    ///
    /// As a result, different meshes may use different variants of the shader and need
    /// different bind group layouts.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct ActiveVariant: usize {
        /// Whether this mesh uses skeletal skinning.
        ///
        /// It is true whenever the mesh being rendered has both `Mesh::ATTRIBUTE_JOINT_INDEX`
        /// and `Mesh::ATTRIBUTE_JOINT_WEIGHT` vertex attributes.
        const SKIN = 1 << 0;

        /// Whether this mesh uses morph targets.
        ///
        /// This is determined by the `MeshPipelineKey::MORPH_TARGETS` pipeline key
        /// flag.
        ///
        /// This pipeline key flag is set whenever the mesh being rendered has the
        /// `morph_targets` field set to `Some`.
        const MORPH = 1 << 1;

        /// Whether this mesh is being rendered with motion vectors.
        ///
        /// This is determined by the `MeshPipelineKey::MOTION_VECTOR_PREPASS` pipeline key
        /// flag.
        ///
        /// This pipeline key flag is set whenever the view rendering the mesh has the
        /// `MotionVectorPrepass` component, **and** the mesh, if it has morph targets
        /// or skinning, was rendered last frame.
        ///
        /// Note that the same mesh can be rendered by different views, some of them
        /// with or without motion vectors.
        const MOTION_VECTORS =  1 << 2;

        // NOTE: ADDING A NEW VARIANT
        // You'll have to add handling for the new variants in the various places
        // in this file:
        // - fmt::Display for ActiveVariant
        // - ActiveVariant::new
        // - MeshBindGroupBuilder::add_variant
        // - variant_layout
        // - setup_morph_and_skinning_defs in render/mesh.rs
        // - RenderCommand<P> for SetMeshBindGroup in render/mesh.rs
    }
}
impl ActiveVariant {
    const COUNT: usize = Self::all().bits() + 1;

    fn new(skin: bool, morph: bool, mut motion_vectors: bool) -> Self {
        let if_enabled = |flag, value| if flag { value } else { Self::empty() };

        // Meshes with and without motion vectors, but also without skins/morphs
        // share the same bind group.
        if !skin && !morph {
            motion_vectors = false;
        }
        if_enabled(skin, Self::SKIN)
            | if_enabled(morph, Self::MORPH)
            | if_enabled(motion_vectors, Self::MOTION_VECTORS)
    }
}
impl fmt::Display for ActiveVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.contains(Self::SKIN) {
            f.write_str("skinned_")?;
        }
        if self.contains(Self::MORPH) {
            f.write_str("morphed_")?;
        }
        if self.contains(Self::MOTION_VECTORS) {
            f.write_str("motion_vectors_")?;
        }
        f.write_str("mesh")
    }
}
