use alloc::sync::Arc;

use crate::*;
use bevy_app::Plugin;
use bevy_camera::{Camera3d, Projection};
use bevy_core_pipeline::{
    core_3d::{
        AlphaMask3d, Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, Transmissive3d, Transparent3d,
    },
    oit::OrderIndependentTransparencySettings,
    prepass::{
        DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass,
        OpaqueNoLightmap3dBatchSetKey, OpaqueNoLightmap3dBinKey,
    },
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{
    component::Component,
    prelude::*,
    query::{Has, QueryItem},
    system::{Query, ResMut, SystemChangeTick},
};
use bevy_light::{EnvironmentMapLight, IrradianceVolume, ShadowFilteringMethod};
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_render::{
    camera::TemporalJitter,
    extract_component::ExtractComponent,
    render_phase::{
        AddRenderCommand, BinnedPhaseItem, BinnedRenderPhase, BinnedRenderPhasePlugin,
        BinnedRenderPhaseType, DrawFunctions, PhaseItemExtraIndex,
    },
    render_resource::{
        RenderPipelineDescriptor, SpecializedMeshPipeline, SpecializedMeshPipelineError,
    },
    view::{ExtractedView, Msaa},
    Render, RenderApp, RenderDebugFlags, RenderStartup, RenderSystems,
};
use bevy_shader::ShaderDefVal;

#[derive(Default)]
pub struct MainPassPlugin {
    pub debug_flags: RenderDebugFlags,
}
impl Plugin for MainPassPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Camera3d, MainPass>()
            .add_plugins(MeshPassPlugin::<MainPass>::new(self.debug_flags));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_material_pipeline)
            .add_systems(
                Render,
                check_views_need_specialization::<MainPass>.in_set(RenderSystems::PrepareAssets),
            );

        add_prepass_and_shadow_pass(app, self.debug_flags);
    }
}

fn add_prepass_and_shadow_pass(app: &mut App, debug_flags: RenderDebugFlags) {
    app.add_plugins((PrepassPipelinePlugin, PrepassPlugin::new(debug_flags)))
        .add_plugins(BinnedRenderPhasePlugin::<Shadow, MeshPipeline>::new(
            debug_flags,
        ));

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app
        .init_resource::<LightKeyCache>()
        .init_resource::<LightSpecializationTicks>()
        .init_resource::<SpecializedShadowMaterialPipelineCache>()
        .init_resource::<DrawFunctions<Shadow>>()
        .add_render_command::<Shadow, DrawPrepass>()
        .add_systems(
            Render,
            (
                check_views_lights_need_specialization.in_set(RenderSystems::PrepareAssets),
                // specialize_shadows also needs to run after prepare_assets::<PreparedMaterial>,
                // which is fine since ManageViews is after PrepareAssets
                specialize_shadows
                    .in_set(RenderSystems::ManageViews)
                    .after(prepare_lights),
                queue_shadows.in_set(RenderSystems::QueueMeshes),
            ),
        );
}

#[derive(Clone, Copy, Default, Component, ExtractComponent)]
pub struct MainPass;

impl MeshPass for MainPass {
    type ViewKeySource = Self;
    type Specializer = MaterialPipelineSpecializer;
    type PhaseItems = (Opaque3d, AlphaMask3d, Transmissive3d, Transparent3d);
}

pub fn check_views_need_specialization<MP: MeshPass>(
    mut view_key_cache: ResMut<ViewKeyCache<MP>>,
    mut view_specialization_ticks: ResMut<ViewSpecializationTicks<MP>>,
    mut views: Query<
        (
            &ExtractedView,
            &Msaa,
            Option<&Tonemapping>,
            Option<&DebandDither>,
            Option<&ShadowFilteringMethod>,
            Has<ScreenSpaceAmbientOcclusion>,
            (
                Has<NormalPrepass>,
                Has<DepthPrepass>,
                Has<MotionVectorPrepass>,
                Has<DeferredPrepass>,
            ),
            Option<&Camera3d>,
            Has<TemporalJitter>,
            Option<&Projection>,
            Has<DistanceFog>,
            (
                Has<RenderViewLightProbes<EnvironmentMapLight>>,
                Has<RenderViewLightProbes<IrradianceVolume>>,
            ),
            Has<OrderIndependentTransparencySettings>,
        ),
        With<MP>,
    >,
    ticks: SystemChangeTick,
) {
    for (
        view,
        msaa,
        tonemapping,
        dither,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
        camera_3d,
        temporal_jitter,
        projection,
        distance_fog,
        (has_environment_maps, has_irradiance_volumes),
        has_oit,
    ) in views.iter_mut()
    {
        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if temporal_jitter {
            view_key |= MeshPipelineKey::TEMPORAL_JITTER;
        }

        if has_environment_maps {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if has_irradiance_volumes {
            view_key |= MeshPipelineKey::IRRADIANCE_VOLUME;
        }

        if has_oit {
            view_key |= MeshPipelineKey::OIT_ENABLED;
        }

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
                Projection::Custom(_) => MeshPipelineKey::VIEW_PROJECTION_NONSTANDARD,
            };
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Gaussian => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN;
            }
            ShadowFilteringMethod::Temporal => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL;
            }
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }
        if ssao {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }
        if distance_fog {
            view_key |= MeshPipelineKey::DISTANCE_FOG;
        }
        if let Some(camera_3d) = camera_3d {
            view_key |= screen_space_specular_transmission_pipeline_key(
                camera_3d.screen_space_specular_transmission_quality,
            );
        }
        if !view_key_cache
            .get_mut(&view.retained_view_entity)
            .is_some_and(|current_key| *current_key == view_key)
        {
            view_key_cache.insert(view.retained_view_entity, view_key);
            view_specialization_ticks.insert(view.retained_view_entity, ticks.this_run());
        }
    }
}

pub fn init_material_pipeline(mut commands: Commands, mesh_pipeline: Res<MeshPipeline>) {
    commands.insert_resource(MaterialPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
    });
}
pub struct MaterialPipelineSpecializer {
    pub(crate) pipeline: MaterialPipeline,
    pub(crate) properties: Arc<MaterialProperties>,
}

impl MeshPassSpecializer for MaterialPipelineSpecializer {
    type Pipeline = MaterialPipeline;

    fn create_key(context: &SpecializerKeyContext) -> Self::Key {
        let mut mesh_pipeline_key_bits = context.material.properties.mesh_pipeline_key_bits;
        mesh_pipeline_key_bits.insert(alpha_mode_pipeline_key(
            context.material.properties.alpha_mode,
            &Msaa::from_samples(context.view_key.msaa_samples()),
        ));
        let mut mesh_key = context.view_key
            | MeshPipelineKey::from_bits_retain(context.mesh_pipeline_key.bits())
            | mesh_pipeline_key_bits;

        if let Some(lightmap) = context.lightmap {
            mesh_key |= MeshPipelineKey::LIGHTMAPPED;

            if lightmap.bicubic_sampling {
                mesh_key |= MeshPipelineKey::LIGHTMAP_BICUBIC_SAMPLING;
            }
        }

        if context.has_crossfade {
            mesh_key |= MeshPipelineKey::VISIBILITY_RANGE_DITHER;
        }

        if context
            .view_key
            .contains(MeshPipelineKey::MOTION_VECTOR_PREPASS)
        {
            // If the previous frame have skins or morph targets, note that.
            if context
                .mesh_instance_flags
                .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_SKIN)
            {
                mesh_key |= MeshPipelineKey::HAS_PREVIOUS_SKIN;
            }
            if context
                .mesh_instance_flags
                .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_MORPH)
            {
                mesh_key |= MeshPipelineKey::HAS_PREVIOUS_MORPH;
            }
        }

        let material_key = context.material.properties.material_key.clone();

        Self::Key {
            mesh_key,
            material_key,
            type_id: context.material_asset_id,
            pass_id: context.pass_id,
        }
    }

    fn new(pipeline: &Self::Pipeline, material: &PreparedMaterial) -> Self {
        MaterialPipelineSpecializer {
            pipeline: pipeline.clone(),
            properties: material.properties.clone(),
        }
    }
}

impl SpecializedMeshPipeline for MaterialPipelineSpecializer {
    type Key = ErasedMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self
            .pipeline
            .mesh_pipeline
            .specialize(key.mesh_key, layout)?;
        descriptor.vertex.shader_defs.push(ShaderDefVal::UInt(
            "MATERIAL_BIND_GROUP".into(),
            MATERIAL_BIND_GROUP_INDEX as u32,
        ));
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader_defs.push(ShaderDefVal::UInt(
                "MATERIAL_BIND_GROUP".into(),
                MATERIAL_BIND_GROUP_INDEX as u32,
            ));
        };
        if let Some(vertex_shader) = self
            .properties
            .get_shader(MaterialVertexShader(key.pass_id))
        {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = self
            .properties
            .get_shader(MaterialFragmentShader(key.pass_id))
        {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor
            .layout
            .insert(3, self.properties.material_layout.as_ref().unwrap().clone());

        if let Some(specialize) = self.properties.specialize {
            specialize(&self.pipeline, &mut descriptor, layout, key)?;
        }

        // If bindless mode is on, add a `BINDLESS` define.
        if self.properties.bindless {
            descriptor.vertex.shader_defs.push("BINDLESS".into());
            if let Some(ref mut fragment) = descriptor.fragment {
                fragment.shader_defs.push("BINDLESS".into());
            }
        }

        Ok(descriptor)
    }
}

pub struct NoExtractCondition;

impl ExtractCondition for NoExtractCondition {
    type ViewQuery = ();

    #[inline]
    fn should_extract(_item: QueryItem<'_, '_, Self::ViewQuery>) -> bool {
        true
    }
}

impl PhaseItemExt for Opaque3d {
    type PhaseFamily = BinnedPhaseFamily;
    type ExtractCondition = NoExtractCondition;
    type RenderCommand = DrawMaterial;
    const PHASE_TYPES: RenderPhaseType = RenderPhaseType::Opaque;
}

impl QueueBinnedPhaseItem for Opaque3d {
    #[inline]
    fn queue_item<BPI>(context: &PhaseContext, render_phase: &mut BinnedRenderPhase<BPI>)
    where
        BPI: BinnedPhaseItem<BatchSetKey = Self::BatchSetKey, BinKey = Self::BinKey>,
    {
        if context.material.properties.render_method == OpaqueRendererMethod::Deferred {
            // Even though we aren't going to insert the entity into
            // a bin, we still want to update its cache entry. That
            // way, we know we don't need to re-examine it in future
            // frames.
            render_phase.update_cache(context.main_entity, None, context.current_change_tick);
            return;
        }
        let (vertex_slab, index_slab) = context
            .mesh_allocator
            .mesh_slabs(&context.mesh_instance.mesh_asset_id);

        render_phase.add(
            Opaque3dBatchSetKey {
                pipeline: context.pipeline_id,
                draw_function: context.draw_function,
                material_bind_group_index: Some(context.material.binding.group.0),
                vertex_slab: vertex_slab.unwrap_or_default(),
                index_slab,
                lightmap_slab: context
                    .mesh_instance
                    .shared
                    .lightmap_slab_index
                    .map(|index| *index),
            },
            Opaque3dBinKey {
                asset_id: context.mesh_instance.mesh_asset_id.into(),
            },
            (context.entity, context.main_entity),
            context.mesh_instance.current_uniform_index,
            BinnedRenderPhaseType::mesh(
                context.mesh_instance.should_batch(),
                &context.gpu_preprocessing_support,
            ),
            context.current_change_tick,
        );
    }
}

impl PhaseItemExt for AlphaMask3d {
    type PhaseFamily = BinnedPhaseFamily;
    type ExtractCondition = NoExtractCondition;
    type RenderCommand = DrawMaterial;
    const PHASE_TYPES: RenderPhaseType = RenderPhaseType::AlphaMask;
}

impl QueueBinnedPhaseItem for AlphaMask3d {
    #[inline]
    fn queue_item<BPI>(context: &PhaseContext, render_phase: &mut BinnedRenderPhase<BPI>)
    where
        BPI: BinnedPhaseItem<BatchSetKey = Self::BatchSetKey, BinKey = Self::BinKey>,
    {
        let (vertex_slab, index_slab) = context
            .mesh_allocator
            .mesh_slabs(&context.mesh_instance.mesh_asset_id);

        render_phase.add(
            OpaqueNoLightmap3dBatchSetKey {
                pipeline: context.pipeline_id,
                draw_function: context.draw_function,
                material_bind_group_index: Some(context.material.binding.group.0),
                vertex_slab: vertex_slab.unwrap_or_default(),
                index_slab,
            },
            OpaqueNoLightmap3dBinKey {
                asset_id: context.mesh_instance.mesh_asset_id.into(),
            },
            (context.entity, context.main_entity),
            context.mesh_instance.current_uniform_index,
            BinnedRenderPhaseType::mesh(
                context.mesh_instance.should_batch(),
                &context.gpu_preprocessing_support,
            ),
            context.current_change_tick,
        );
    }
}

impl PhaseItemExt for Transmissive3d {
    type PhaseFamily = SortedPhaseFamily;
    type ExtractCondition = NoExtractCondition;
    type RenderCommand = DrawMaterial;
    const PHASE_TYPES: RenderPhaseType = RenderPhaseType::Transmissive;
}

impl QueueSortedPhaseItem for Transmissive3d {
    #[inline]
    fn get_item(context: &PhaseContext) -> Option<Self> {
        let (_, index_slab) = context
            .mesh_allocator
            .mesh_slabs(&context.mesh_instance.mesh_asset_id);
        let distance = context.rangefinder.distance(&context.mesh_instance.center)
            + context.material.properties.depth_bias;

        Some(Transmissive3d {
            entity: (context.entity, context.main_entity),
            draw_function: context.draw_function,
            pipeline: context.pipeline_id,
            distance,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            indexed: index_slab.is_some(),
        })
    }
}

impl PhaseItemExt for Transparent3d {
    type PhaseFamily = SortedPhaseFamily;
    type ExtractCondition = NoExtractCondition;
    type RenderCommand = DrawMaterial;
    const PHASE_TYPES: RenderPhaseType = RenderPhaseType::Transparent;
}

impl QueueSortedPhaseItem for Transparent3d {
    #[inline]
    fn get_item(context: &PhaseContext) -> Option<Self> {
        let (_, index_slab) = context
            .mesh_allocator
            .mesh_slabs(&context.mesh_instance.mesh_asset_id);
        let distance = context.rangefinder.distance(&context.mesh_instance.center)
            + context.material.properties.depth_bias;

        Some(Transparent3d {
            entity: (context.entity, context.main_entity),
            draw_function: context.draw_function,
            pipeline: context.pipeline_id,
            distance,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            indexed: index_slab.is_some(),
        })
    }
}
