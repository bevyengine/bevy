use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{
    asset_changed::AssetChanged, AsAssetId, Asset, AssetApp, AssetEvent, AssetEventSystems,
    AssetId, Assets, Handle,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    error::Result,
    lifecycle::HookContext,
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
    world::DeferredWorld,
};
use bevy_image::TextureAtlasLayout;
use bevy_material::{
    descriptor::{BindGroupLayoutDescriptor, RenderPipelineDescriptor},
    specialize::SpecializedMeshPipelineError,
};
use bevy_mesh::{Mesh2d, MeshVertexBufferLayoutRef};
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroupBuilder, BindGroupLayout, BindGroupLayoutEntry,
        BindlessDescriptor, BindlessSlabResourceLimit,
    },
    renderer::RenderDevice,
};
use bevy_shader::ShaderRef;
use bevy_sprite::{Anchor, SpriteMesh};
use core::hash::Hash;

use crate::{
    check_entities_needing_specialization, AlphaMode2d, ExtendedMaterial2d, Material2dKey,
    Material2dPlugin, MaterialExtension2d, MeshMaterial2d, SpriteMeshMaterial,
};

pub struct SpriteMaterialPlugin<M>(core::marker::PhantomData<M>)
where
    M::Data: Clone + Hash + Eq,
    M: Asset + MaterialExtension2d;

impl<M> Plugin for SpriteMaterialPlugin<M>
where
    <SpriteExt<M> as AsBindGroup>::Data: Clone + Hash + Eq,
    M::Data: Clone + Hash + Eq,
    M: Asset + MaterialExtension2d,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<M>()
            .register_type::<SpriteMaterial<M>>()
            .init_resource::<SpriteMaterialCache<M>>()
            .add_plugins((
                Material2dPlugin::<SpriteExt<M>>::default(),
                RenderAssetPlugin::<ExtractedSpriteMaterial<M>>::default(),
            ))
            .add_systems(
                PostUpdate,
                clean_sprite_material_cache::<M>.after(AssetEventSystems),
            )
            .add_systems(
                PostUpdate,
                add_material::<M>.before(check_entities_needing_specialization::<SpriteExt<M>>),
            );
    }
}

impl<M> Default for SpriteMaterialPlugin<M>
where
    M::Data: Clone + Hash + Eq,
    M: Asset + MaterialExtension2d,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

fn add_material<M>(
    sprites: Query<
        (Entity, &SpriteMesh, &Anchor, &SpriteMaterial<M>),
        Or<(
            Changed<SpriteMaterial<M>>,
            AssetChanged<SpriteMaterial<M>>,
            Changed<SpriteMesh>,
            Changed<Anchor>,
            Added<Mesh2d>,
        )>,
    >,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    sprite_materials: Res<Assets<M>>,
    mut cache: ResMut<SpriteMaterialCache<M>>,
    mut materials: ResMut<Assets<SpriteExt<M>>>,
    mut commands: Commands,
) where
    M: Asset + MaterialExtension2d,
    M::Data: Clone,
{
    for (entity, sprite, anchor, sprite_material) in sprites {
        let handle = if let Some(handle) =
            cache.get(&(sprite_material.0.id(), sprite.clone(), *anchor))
        {
            if let Some(mut extended) = materials.get_mut(handle)
                && let Some(sprite_material_instance) = sprite_materials.get(&sprite_material.0)
            {
                extended.extension.update(sprite_material_instance);
            }

            handle.clone()
        } else {
            let material = super::make_sprite_mesh_material(&texture_atlas_layouts, sprite, anchor);
            let Some(sprite_material_instance) = sprite_materials.get(&sprite_material.0) else {
                continue;
            };

            let extended = ExtendedMaterial2d {
                base: material,
                extension: SpriteMaterialExtension::new(
                    sprite_material.id(),
                    sprite_material_instance,
                ),
            };

            let handle = materials.add(extended);
            cache.insert(
                (sprite_material.id(), sprite.clone(), *anchor),
                handle.clone(),
            );

            handle
        };

        commands
            .entity(entity)
            .remove::<MeshMaterial2d<SpriteMeshMaterial>>()
            .insert(MeshMaterial2d(handle));
    }
}

#[derive(Component, Reflect, Deref, DerefMut, PartialEq, Debug, FromTemplate, Clone)]
#[reflect(Component)]
#[component(on_add, on_remove)]
pub struct SpriteMaterial<M>(pub Handle<M>)
where
    M::Data: Clone,
    M: Asset + MaterialExtension2d;

impl<M> SpriteMaterial<M>
where
    M::Data: Clone,
    M: Asset + MaterialExtension2d,
{
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let mut count = world
            .entity(context.entity)
            .get::<SpriteMaterialCount>()
            .map(|c| c.0)
            .unwrap_or(0);

        count += 1;

        world
            .commands()
            .entity(context.entity)
            .try_insert(SpriteMaterialCount(count));
    }

    fn on_remove(mut world: DeferredWorld, context: HookContext) {
        let mut count = world
            .entity(context.entity)
            .get::<SpriteMaterialCount>()
            .map(|c| c.0)
            .unwrap_or(1);

        count = count.saturating_sub(1);

        world
            .commands()
            .entity(context.entity)
            .try_remove::<MeshMaterial2d<SpriteExt<M>>>()
            .try_insert(SpriteMaterialCount(count));
    }
}

impl<M> AsAssetId for SpriteMaterial<M>
where
    M::Data: Clone,
    M: Asset + MaterialExtension2d,
{
    type Asset = M;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// Tracks the number of [`SpriteMaterial`]s on a given entity in order to accurately add and remove the default sprite's material
#[derive(Component)]
#[component(immutable)]
pub(super) struct SpriteMaterialCount(pub u32);

// We need to directly extract `M` to the render world to access it in the `AsBindGroup` implementation
struct ExtractedSpriteMaterial<M>(M);

impl<M: Asset + Clone + Send> RenderAsset for ExtractedSpriteMaterial<M> {
    type SourceAsset = M;

    type Param = ();

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        _param: &mut SystemParamItem<Self::Param>,
        _previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(Self(source_asset))
    }
}

// We only store the `AssetId` because we don't want to keep it alive
#[derive(Reflect, Clone)]
struct SpriteMaterialExtension<M: Asset + AsBindGroup> {
    asset: AssetId<M>,
    data: M::Data,
    depth_bias: Option<f32>,
    alpha_mode: Option<AlphaMode2d>,
}

impl<M: Asset + MaterialExtension2d> SpriteMaterialExtension<M> {
    fn new(asset: AssetId<M>, instance: &M) -> Self {
        Self {
            asset,
            data: instance.bind_group_data(),
            depth_bias: instance.depth_bias(),
            alpha_mode: instance.alpha_mode(),
        }
    }

    fn update(&mut self, instance: &M) {
        self.data = instance.bind_group_data();
        self.depth_bias = instance.depth_bias();
        self.alpha_mode = instance.alpha_mode();
    }
}

impl<M> MaterialExtension2d for SpriteMaterialExtension<M>
where
    M::Data: Clone,
    M: Asset + MaterialExtension2d,
{
    fn vertex_shader() -> Option<ShaderRef> {
        M::vertex_shader()
    }

    fn fragment_shader() -> Option<ShaderRef> {
        M::fragment_shader()
    }

    fn depth_bias(&self) -> Option<f32> {
        self.depth_bias
    }

    fn alpha_mode(&self) -> Option<AlphaMode2d> {
        self.alpha_mode
    }

    fn specialize(
        pipeline: &crate::Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        M::specialize(
            pipeline,
            descriptor,
            layout,
            Material2dKey {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data,
            },
        )
    }
}

impl<M> AsBindGroup for SpriteMaterialExtension<M>
where
    M::Data: Clone,
    M: Asset + AsBindGroup + Clone,
{
    type Data = M::Data;

    type Param = (SRes<RenderAssets<ExtractedSpriteMaterial<M>>>, M::Param);

    fn label() -> &'static str {
        M::label()
    }

    fn bind_group_data(&self) -> Self::Data {
        self.data.clone()
    }

    fn build_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        param: &mut SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
        output: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        let Some(ExtractedSpriteMaterial(asset)) = param.0.get(self.asset) else {
            return Err(AsBindGroupError::RetryNextUpdate);
        };

        asset.build_bind_group(
            layout,
            render_device,
            &mut param.1,
            force_no_bindless,
            output,
        )
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        M::bind_group_layout_entries(render_device, force_no_bindless)
    }

    fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
        M::bindless_slot_count()
    }

    fn bindless_supported(render_device: &RenderDevice) -> bool {
        M::bindless_supported(render_device)
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout
    where
        Self: Sized,
    {
        M::bind_group_layout(render_device)
    }

    fn bind_group_layout_descriptor(render_device: &RenderDevice) -> BindGroupLayoutDescriptor
    where
        Self: Sized,
    {
        M::bind_group_layout_descriptor(render_device)
    }

    fn bindless_descriptor() -> Option<BindlessDescriptor> {
        M::bindless_descriptor()
    }
}

/// Keeps an index of cached `SpriteExt` handles based on the [`SpriteMesh`], [`Anchor`], and the asset id of the material
#[derive(Resource, Deref, DerefMut)]
struct SpriteMaterialCache<M>(HashMap<(AssetId<M>, SpriteMesh, Anchor), Handle<SpriteExt<M>>>)
where
    M::Data: Clone,
    M: Asset + MaterialExtension2d;

impl<M> Default for SpriteMaterialCache<M>
where
    M::Data: Clone,
    M: Asset + MaterialExtension2d,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

fn clean_sprite_material_cache<M>(
    mut index: ResMut<SpriteMaterialCache<M>>,
    mut asset_events: MessageReader<AssetEvent<M>>,
) where
    M::Data: Clone,
    M: Asset + MaterialExtension2d,
{
    for message in asset_events.read() {
        if let AssetEvent::Removed { id } = *message {
            index.retain(|(key_id, ..), _| *key_id != id);
        }
    }
}

type SpriteExt<M> = ExtendedMaterial2d<SpriteMeshMaterial, SpriteMaterialExtension<M>>;
