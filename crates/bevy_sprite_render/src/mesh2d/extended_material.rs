use crate::{AlphaMode2d, Material2d, Material2dKey, Material2dPipeline};
use bevy_asset::Asset;
use bevy_ecs::{error::Result, system::SystemParamItem};
use bevy_material::{
    descriptor::RenderPipelineDescriptor, specialize::SpecializedMeshPipelineError,
};
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupBuilder, BindGroupLayout, BindGroupLayoutEntry,
        BindlessDescriptor, BindlessSlabResourceLimit, CombinedBindGroup as Cbg,
    },
    renderer::RenderDevice,
};
use bevy_shader::ShaderRef;

/// A copy of the [`Material2d`] trait with methods returning `Option`,
/// which can be used to override the base material in an [`ExtendedMaterial2d`]
pub trait MaterialExtension2d: Sized + AsBindGroup + TypePath + Clone + Send + Sync {
    /// The vertex shader to override the base material's vertex shader with, if any
    ///
    /// See [`Material2d::vertex_shader`] for more info
    fn vertex_shader() -> Option<ShaderRef> {
        None
    }

    /// The fragment shader to override the base material's fragment shader with, if any
    ///
    /// See [`Material2d::fragment_shader`] for more info
    fn fragment_shader() -> Option<ShaderRef> {
        None
    }

    /// The depth bias to override the base material's depth bias with, if any
    ///
    /// See [`Material2d::depth_bias`] for more info
    fn depth_bias(&self) -> Option<f32> {
        None
    }

    /// The alpha mode to override the base material's alpha mode with, if any
    ///
    /// See [`Material2d::alpha_mode`] for more info
    fn alpha_mode(&self) -> Option<AlphaMode2d> {
        None
    }

    /// Apply specialization to the render pipeline after the base material's [`specialize`](Material2d::specialize) method was ran
    ///
    /// See [`Material2d::specialize`] for more info
    #[expect(
        unused_variables,
        reason = "The parameters here are unused by the default implementation, but adding underscores to their names will be copied by rust-analyzer's completion"
    )]
    fn specialize(
        pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// A material that extends a base [`Material2d`] with additional shaders and data
///
/// The data from both materials will be combined and made available to the shader
/// so that functions built for the base material and for the extension will work as expected
///
/// Material extensions should make sure that the bindings don't overlap with the base material when implementing [`AsBindGroup`]
///
/// See [`MaterialExtension2d`]
#[derive(Asset, Clone, Debug, Default, Reflect)]
#[reflect(Clone)]
pub struct ExtendedMaterial2d<B: Material2d, E: MaterialExtension2d> {
    pub base: B,
    pub extension: E,
}

impl<B, E> Material2d for ExtendedMaterial2d<B, E>
where
    B: Material2d,
    E: MaterialExtension2d,
{
    fn vertex_shader() -> ShaderRef {
        E::vertex_shader().unwrap_or_else(B::vertex_shader)
    }

    fn fragment_shader() -> ShaderRef {
        E::fragment_shader().unwrap_or_else(B::fragment_shader)
    }

    fn depth_bias(&self) -> f32 {
        self.extension
            .depth_bias()
            .unwrap_or_else(|| self.base.depth_bias())
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.extension
            .alpha_mode()
            .unwrap_or_else(|| self.base.alpha_mode())
    }

    fn specialize(
        pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let base_key = Material2dKey {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.base,
        };

        B::specialize(pipeline, descriptor, layout, base_key)?;

        let extension_key = Material2dKey {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data.extension,
        };

        E::specialize(pipeline, descriptor, layout, extension_key)
    }
}

impl<B: Material2d, E: MaterialExtension2d> AsBindGroup for ExtendedMaterial2d<B, E> {
    type Data = <Cbg<'static, B, E> as AsBindGroup>::Data;
    type Param = <Cbg<'static, B, E> as AsBindGroup>::Param;

    fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
        Cbg::<'static, B, E>::bindless_slot_count()
    }

    fn bindless_supported(render_device: &RenderDevice) -> bool {
        Cbg::<'static, B, E>::bindless_supported(render_device)
    }

    fn label() -> &'static str {
        Cbg::<'static, B, E>::label()
    }

    fn bind_group_data(&self) -> Self::Data {
        Cbg {
            base: &self.base,
            extension: &self.extension,
        }
        .bind_group_data()
    }

    fn build_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        param: &mut SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
        output: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        Cbg {
            base: &self.base,
            extension: &self.extension,
        }
        .build_bind_group(layout, render_device, param, force_no_bindless, output)
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        Cbg::<'static, B, E>::bind_group_layout_entries(render_device, force_no_bindless)
    }

    fn bindless_descriptor() -> Option<BindlessDescriptor> {
        Cbg::<'static, B, E>::bindless_descriptor()
    }
}
