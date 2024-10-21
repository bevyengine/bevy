use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{lifetimeless::SRes, SystemParamItem},
    world::{FromWorld, World},
};
use core::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetId, AssetPath};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::{Query, ResMut, Resource};
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    render_resource::{AsBindGroup, AsBindGroupError, BindGroupLayout, PreparedBindGroup},
    renderer::RenderDevice,
    sync_world::{MainEntity, MainEntityHashMap},
    view::ViewVisibility,
    Extract, ExtractSchedule, RenderApp,
};

use crate::renderer::Renderer;

pub enum SpecializeMaterialPipelineError {}

pub trait BaseMaterial: Asset + AsBindGroup + Clone + Sized {}

impl<T: Asset + AsBindGroup + Clone + Sized> BaseMaterial for T {}

pub trait Material<R: Renderer>: BaseMaterial {
    fn properties(&self) -> R::MaterialProperties;
    fn shaders(key: R::ShaderKey) -> Option<AssetPath<'static>>;
    fn specialize(info: R::PipelineInfo<'_, Self>) -> Result<(), SpecializeMaterialPipelineError>;
}

pub struct MaterialPlugin<M: Material<R>, R: Renderer>(PhantomData<fn(M, R)>);

impl<M: Material<R>, R: Renderer> Plugin for MaterialPlugin<M, R> {
    fn build(&self, app: &mut App) {
        let has_any_resource_plugin = app.world().get_resource::<AnyMaterialPlugin<M>>().is_some();

        app.add_plugins(R::material_plugin::<M>);

        if !has_any_resource_plugin {
            if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
                render_app.add_systems(ExtractSchedule, clear_material_instances::<M>);
            }

            app.init_asset::<M>()
                .register_type::<R::SourceComponent<M>>()
                .add_plugins(RenderAssetPlugin::<MaterialBindGroup<M>>::default())
                .init_resource::<AnyMaterialPlugin<M>>();
        }

        app.add_plugins(RenderAssetPlugin::<PreparedMaterialProperties<M, R>>::default());
        app.add_systems(
            ExtractSchedule,
            extract_materials::<M, R>.after(clear_material_instances::<M>),
        );
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<MaterialLayout<M>>();
        }
    }
}

fn clear_material_instances<M: BaseMaterial>(
    mut material_instances: ResMut<RenderMaterialInstances<M>>,
) {
    material_instances.clear();
}

fn extract_materials<M: Material<R>, R: Renderer>(
    mut material_instances: ResMut<RenderMaterialInstances<M>>,
    materials: Extract<Query<(&MainEntity, &ViewVisibility, &R::SourceComponent<M>)>>,
) {
    for (main_entity, view_visibility, material) in &materials {
        if view_visibility.get() {
            material_instances.insert(*main_entity, R::source_asset_id(material));
        }
    }
}

fn queue_materials<M: Material<R>, R: Renderer>() {}

#[derive(Resource)]
pub struct AnyMaterialPlugin<M: BaseMaterial> {
    _data: PhantomData<M>,
}

impl<M: BaseMaterial> Default for AnyMaterialPlugin<M> {
    fn default() -> Self {
        Self { _data: PhantomData }
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
struct PreparedMaterialProperties<M: Material<R>, R: Renderer> {
    #[deref]
    properties: R::MaterialProperties,
    _data: PhantomData<M>,
}

impl<M: Material<R>, R: Renderer> PreparedMaterialProperties<M, R> {
    pub fn new(material: &M) -> Self {
        Self {
            properties: material.properties(),
            _data: PhantomData,
        }
    }
}

impl<M: Material<R>, R: Renderer> RenderAsset for PreparedMaterialProperties<M, R> {
    type SourceAsset = M;

    type Param = ();

    fn prepare_asset(
        material: Self::SourceAsset,
        (): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(PreparedMaterialProperties::new(&material))
    }
}

#[derive(Resource, Deref)]
pub struct MaterialLayout<M: BaseMaterial> {
    #[deref]
    layout: BindGroupLayout,
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
