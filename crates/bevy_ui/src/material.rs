use std::{hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, AssetEvent, AssetServer, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Bundle, Entity, EventReader},
    query::ROQueryItem,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Local, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_reflect::{TypePath, TypeUuid};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    render_asset::{PrepareAssetSet, RenderAssets},
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        AsBindGroup, AsBindGroupError, AsBindGroupShaderType, BindGroup, BindGroupLayout,
        OwnedBindingResource, PipelineCache, RenderPipelineDescriptor, Shader, ShaderRef,
        SpecializedRenderPipeline, SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
    texture::{FallbackImage, Image},
    view::{ComputedVisibility, ExtractedView, ViewUniforms, Visibility},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_utils::{FloatOrd, HashMap, HashSet};

use crate::{
    DrawUi, DrawUiNode, FocusPolicy, Node, SetUiTextureBindGroup, SetUiViewBindGroup,
    Style, TransparentUi, UiBatch, UiImageBindGroups, UiMeta, UiPipeline, UiPipelineKey, ZIndex,
};

pub trait UiMaterial: AsBindGroup + Send + Sync + Clone + TypeUuid + TypePath + Sized {
    /// Returns this materials vertex shader. If [`ShaderRef::Default`] is returned, the default UI
    /// vertex shader will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this materials fragment shader. If [`ShaderRef::Default`] is returned, the default
    /// UI fragment shader will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    #[allow(unused_variables)]
    #[inline]
    fn specialize(descriptor: &mut RenderPipelineDescriptor, key: UiMaterialKey<Self>) {}
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given
/// [`UiMaterial`] asset type (which includes [`UiMaterial`] types).
pub struct UiMaterialPlugin<M: UiMaterial>(PhantomData<M>);

impl<M: UiMaterial> Default for UiMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: UiMaterial> Plugin for UiMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut bevy_app::App) {
        app.add_asset::<M>()
            .add_plugins(ExtractComponentPlugin::<Handle<M>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<ExtractedUiMaterials<M>>()
                .init_resource::<RenderUiMaterials<M>>()
                .init_resource::<SpecializedRenderPipelines<UiMaterialPipeline<M>>>()
                .add_systems(ExtractSchedule, extract_ui_materials::<M>)
                .add_systems(
                    Render,
                    (
                        prepare_ui_materials::<M>
                            .in_set(RenderSet::Prepare)
                            .after(PrepareAssetSet::PreAssetPrepare),
                        queue_ui_material_nodes::<M>.in_set(RenderSet::Queue),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiMaterialPipeline<M>>();
        }
    }
}

pub struct UiMaterialKey<M: UiMaterial> {
    pub hdr: bool,
    pub bind_group_data: M::Data,
}

impl<M: UiMaterial> Eq for UiMaterialKey<M> where M::Data: PartialEq {}

impl<M: UiMaterial> PartialEq for UiMaterialKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.hdr == other.hdr && self.bind_group_data == other.bind_group_data
    }
}

impl<M: UiMaterial> Clone for UiMaterialKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            hdr: self.hdr,
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: UiMaterial> Hash for UiMaterialKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hdr.hash(state);
        self.bind_group_data.hash(state);
    }
}

#[derive(Resource)]
pub struct UiMaterialPipeline<M: UiMaterial> {
    pub ui_pipeline: UiPipeline,
    pub ui_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> SpecializedRenderPipeline for UiMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = UiMaterialKey<M>;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.ui_pipeline.specialize(UiPipelineKey { hdr: key.hdr });
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout = vec![
            self.ui_pipeline.view_layout.clone(),
            self.ui_layout.clone(),
            self.ui_pipeline.image_layout.clone(),
        ];

        M::specialize(&mut descriptor, key);

        return descriptor;
    }
}

impl<M: UiMaterial> FromWorld for UiMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let ui_layout = M::bind_group_layout(render_device);

        UiMaterialPipeline {
            ui_pipeline: world.resource::<UiPipeline>().clone(),
            ui_layout,
            vertex_shader: match M::vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: match M::fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            marker: PhantomData,
        }
    }
}

type DrawUiMaterial<M> = (
    SetItemPipeline,
    SetUiViewBindGroup<0>,
    SetUiMaterialBindGroup<M, 1>,
    SetUiTextureBindGroup<2>,
    DrawUiNode,
);

pub struct SetUiMaterialBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P>
    for SetUiMaterialBindGroup<M, I>
{
    type Param = SRes<RenderUiMaterials<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<M>>;

    fn render<'w>(
        item: &P,
        view: (),
        ui_material_handle: ROQueryItem<'_, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let ui_material = materials.into_inner().get(ui_material_handle).unwrap();
        pass.set_bind_group(I, &ui_material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_ui_material_nodes<M: UiMaterial>(
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut ui_meta: ResMut<UiMeta>,
    view_uniforms: Res<ViewUniforms>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    mut image_bind_groups: ResMut<UiImageBindGroups>,
    render_materials: Res<RenderUiMaterials<M>>,
    ui_batches: Query<(&Handle<M>, Entity, &UiBatch)>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<TransparentUi>)>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    if ui_batches.is_empty() {
        return;
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let draw_ui_functrion = draw_functions.read().id::<DrawUiMaterial<M>>();
        for (view, mut transparent_phase) in &mut views {
            for (handle, entity, batch) in &ui_batches {
                if let Some(ui_material) = render_materials.get(handle) {
                    let pipeline_id = pipelines.specialize(
                        &pipeline_cache,
                        &ui_material_pipeline,
                        UiMaterialKey {
                            hdr: view.hdr,
                            bind_group_data: ui_material.key.clone(),
                        },
                    );
                    transparent_phase.add(TransparentUi {
                        sort_key: FloatOrd(batch.z),
                        entity,
                        pipeline: pipeline_id,
                        draw_function: draw_ui_functrion,
                    });
                }
            }
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct RenderUiMaterials<T: UiMaterial>(HashMap<Handle<T>, PreparedUiMaterial<T>>);

impl<T: UiMaterial> Default for RenderUiMaterials<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct PreparedUiMaterial<T: UiMaterial> {
    pub bindings: Vec<OwnedBindingResource>,
    pub bind_group: BindGroup,
    pub key: T::Data,
}

#[derive(Resource)]
pub struct ExtractedUiMaterials<M: UiMaterial> {
    extracted: Vec<(Handle<M>, M)>,
    removed: Vec<Handle<M>>,
}

impl<M: UiMaterial> Default for ExtractedUiMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

pub fn extract_ui_materials<M: UiMaterial>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(&handle) {
            extracted_assets.push((handle, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedUiMaterials {
        extracted: extracted_assets,
        removed,
    });
}

pub struct PrepareNextFrameMaterials<M: UiMaterial> {
    assets: Vec<(Handle<M>, M)>,
}

impl<M: UiMaterial> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

pub fn prepare_ui_materials<M: UiMaterial>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedUiMaterials<M>>,
    mut render_materials: ResMut<RenderUiMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<UiMaterialPipeline<M>>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, material) in queued_assets {
        match prepare_ui_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }
}

fn prepare_ui_material<M: UiMaterial>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &Res<FallbackImage>,
    pipeline: &UiMaterialPipeline<M>,
) -> Result<PreparedUiMaterial<M>, AsBindGroupError> {
    let prepared =
        material.as_bind_group(&pipeline.ui_layout, render_device, images, fallback_image)?;
    Ok(PreparedUiMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        key: prepared.data,
    })
}

#[derive(Bundle, Clone, Debug)]
pub struct MaterialNodeBundle<M: UiMaterial> {
    pub node: Node,
    pub style: Style,
    pub material: Handle<M>,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub z_index: ZIndex,
}

impl<M: UiMaterial> Default for MaterialNodeBundle<M> {
    fn default() -> Self {
        Self {
            node: Default::default(),
            style: Default::default(),
            material: Default::default(),
            focus_policy: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}
