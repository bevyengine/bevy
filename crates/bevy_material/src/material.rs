use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{lifetimeless::SRes, SystemParamItem},
    world::{FromWorld, World},
};
use bevy_utils::HashMap;
use core::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetId, AssetPath, AssetServer, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::{Query, ResMut, Resource};
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    render_resource::{AsBindGroup, AsBindGroupError, BindGroupLayout, PreparedBindGroup, Shader},
    renderer::RenderDevice,
    sync_world::{MainEntity, MainEntityHashMap},
    view::ViewVisibility,
    Extract, ExtractSchedule, RenderApp,
};

use crate::component::MaterialComponent;
use crate::material_pipeline::MaterialPipeline;

pub enum SpecializeMaterialError {}

pub trait BaseMaterial: Asset + AsBindGroup + Clone + Sized {}

impl<T: Asset + AsBindGroup + Clone + Sized> BaseMaterial for T {}

pub trait Material<P: MaterialPipeline>: BaseMaterial {
    fn properties(&self) -> P::MaterialProperties;
    fn shaders() -> impl IntoIterator<Item = (P::ShaderKey, AssetPath<'static>)>;
    fn specialize(info: P::PipelineInfo<'_, Self>) -> Result<(), SpecializeMaterialError>;
}

pub struct BaseMaterialPlugin<M: BaseMaterial>(PhantomData<fn(M)>);

impl<M: BaseMaterial> Default for BaseMaterialPlugin<M> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<M: BaseMaterial> Plugin for BaseMaterialPlugin<M> {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, clear_material_instances::<M>);
        }

        app.init_asset::<M>()
            .add_plugins(RenderAssetPlugin::<MaterialBindGroup<M>>::default());
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<MaterialLayout<M>>();
        }
    }
}

pub struct MaterialPlugin<M: Material<P>, P: MaterialPipeline>(PhantomData<fn(M, P)>);

impl<M: Material<P>, P: MaterialPipeline> Default for MaterialPlugin<M, P> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<M: Material<P>, P: MaterialPipeline> Plugin for MaterialPlugin<M, P> {
    fn build(&self, app: &mut App) {
        app.register_type::<MaterialComponent<M, P>>()
            .add_plugins((
                BaseMaterialPlugin::<M>::default(),
                RenderAssetPlugin::<MaterialProperties<M, P>>::default(),
            ))
            .init_resource::<MaterialShaders<M, P>>()
            .add_systems(
                ExtractSchedule,
                extract_materials::<M, P>.after(clear_material_instances::<M>),
            )
            .add_plugins(P::material_plugin::<M>());
    }
}

fn clear_material_instances<M: BaseMaterial>(
    mut material_instances: ResMut<RenderMaterialInstances<M>>,
) {
    material_instances.clear();
}

fn extract_materials<M: Material<R>, R: MaterialPipeline>(
    mut material_instances: ResMut<RenderMaterialInstances<M>>,
    materials: Extract<Query<(&MainEntity, &ViewVisibility, &MaterialComponent<M, R>)>>,
) {
    for (main_entity, view_visibility, material) in &materials {
        if view_visibility.get() {
            material_instances.insert(*main_entity, material.id());
        }
    }
}

#[macro_export]
macro_rules! material_trait_alias {
    ($name: ident, $renderer: ident) => {
        pub trait $name: $crate::material::Material<$renderer> {}
        impl<T: $crate::material::Material<$renderer>> $name for T {}
    };
}

/// Stores all extracted instances of a [`Material`] in the render world.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterialInstances<M: BaseMaterial>(#[deref] pub MainEntityHashMap<AssetId<M>>);

impl<M: BaseMaterial> Default for RenderMaterialInstances<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Data prepared for a [`Material`] instance.
#[derive(Deref)]
pub struct MaterialBindGroup<M: BaseMaterial> {
    pub bind_group: PreparedBindGroup<M::Data>,
}

impl<M: BaseMaterial> RenderAsset for MaterialBindGroup<M> {
    type SourceAsset = M;

    type Param = (SRes<RenderDevice>, SRes<MaterialLayout<M>>, M::Param);

    fn prepare_asset(
        material: Self::SourceAsset,
        (render_device, layout, ref mut material_param): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        match material.as_bind_group(layout, render_device, material_param) {
            Ok(bind_group) => Ok(MaterialBindGroup { bind_group }),
            Err(AsBindGroupError::RetryNextUpdate) => {
                Err(PrepareAssetError::RetryNextUpdate(material))
            }
            Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
        }
    }
}

#[derive(Deref)]
pub struct MaterialProperties<M: Material<R>, R: MaterialPipeline> {
    #[deref]
    pub properties: R::MaterialProperties,
    _data: PhantomData<M>,
}

impl<M: Material<R>, R: MaterialPipeline> MaterialProperties<M, R> {
    pub fn new(material: &M) -> Self {
        Self {
            properties: material.properties(),
            _data: PhantomData,
        }
    }
}

impl<M: Material<P>, P: MaterialPipeline> RenderAsset for MaterialProperties<M, P> {
    type SourceAsset = M;

    type Param = ();

    fn prepare_asset(
        material: Self::SourceAsset,
        (): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(MaterialProperties::new(&material))
    }
}

#[derive(Resource, Deref)]
pub struct MaterialLayout<M: BaseMaterial> {
    #[deref]
    pub layout: BindGroupLayout,
    _data: PhantomData<M>,
}

impl<M: BaseMaterial> FromWorld for MaterialLayout<M> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self {
            layout: M::bind_group_layout(render_device),
            _data: PhantomData,
        }
    }
}

#[derive(Deref, Resource)]
pub struct MaterialShaders<M: Material<P>, P: MaterialPipeline> {
    #[deref]
    pub shaders: HashMap<P::ShaderKey, Handle<Shader>>,
    _data: PhantomData<fn(M)>,
}

impl<M: Material<P>, P: MaterialPipeline> FromWorld for MaterialShaders<M, P> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            shaders: M::shaders()
                .into_iter()
                .map(|(key, path)| (key, asset_server.load(path)))
                .collect(),
            _data: PhantomData,
        }
    }
}
