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

/// Adds the necessary systems and resources for a [`SpriteMaterial`] of type `M`.
///
/// See [`SpriteMaterial`] for more information.
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
                (
                    add_material::<M>.before(check_entities_needing_specialization::<SpriteExt<M>>),
                    (
                        update_changed_material_extensions::<M>,
                        clean_sprite_material_cache::<M>,
                    )
                        .after(AssetEventSystems),
                ),
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

/// Adds `MeshMaterial2d<SpriteExt<M>>`s to entities with a `SpriteMaterial<M>`
fn add_material<M>(
    sprites: Query<
        (Entity, &SpriteMesh, &Anchor, &SpriteMaterial<M>),
        Or<(
            Changed<SpriteMaterial<M>>,
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
        let Some(instance) = sprite_materials.get(&sprite_material.0) else {
            continue;
        };

        let sprite_cache = cache.entry(sprite_material.id()).or_default();
        let handle = sprite_cache.get_or_insert_with(sprite, *anchor, &mut materials, || {
            let material =
                super::make_sprite_mesh_material(&texture_atlas_layouts, sprite, *anchor);
            ExtendedMaterial2d {
                base: material,
                extension: SpriteMaterialExtension::new(sprite_material.id(), instance),
            }
        });

        commands
            .entity(entity)
            .remove::<MeshMaterial2d<SpriteMeshMaterial>>()
            .insert(MeshMaterial2d(handle));
    }
}

/// Updates `SpriteExt<M>` assets when `M` is changed
fn update_changed_material_extensions<M>(
    changed: Query<
        (
            &mut Mesh2d,
            &MeshMaterial2d<SpriteExt<M>>,
            &SpriteMaterial<M>,
        ),
        AssetChanged<SpriteMaterial<M>>,
    >,
    mut materials: ResMut<Assets<SpriteExt<M>>>,
    sprite_materials: Res<Assets<M>>,
) where
    M::Data: Clone,
    M: MaterialExtension2d + Asset,
{
    for (mut mesh, mesh_material, sprite_material) in changed {
        if let Some(mut extended) = materials.get_mut(&mesh_material.0)
            && let Some(sprite_material_instance) = sprite_materials.get(&sprite_material.0)
        {
            extended.extension.update(sprite_material_instance);
            mesh.set_changed();
        }
    }
}

/// Allows extending the material of a [`SpriteMesh`] using a [`MaterialExtension2d`].
///
/// Requires adding a [`SpriteMaterialPlugin`] to function.
///
/// ```
/// # use bevy_sprite_render::{SpriteMaterialPlugin, MaterialExtension2d, AlphaMode2d};
/// # use bevy_render::render_resource::AsBindGroup;
/// # use bevy_shader::ShaderRef;
/// # use bevy_asset::Asset;
/// # use bevy_reflect::Reflect;
/// # use bevy_app::App;
/// # use bevy_math::Vec4;
/// #[derive(AsBindGroup, Asset, Reflect, Clone)]
/// struct MySpriteMaterial {
///     // Make sure to use a high enough uniform
///     // to avoid colliding with the sprite's bindings
///     #[uniform(20)]
///     some_binding: Vec4,
/// }
///
/// impl MaterialExtension2d for MySpriteMaterial {
///     fn vertex_shader() -> Option<ShaderRef> {
///         None // Return `Some` to override the vertex shader
///     }
///     fn fragment_shader() -> Option<ShaderRef> {
///         None // Return `Some` to override the fragment shader
///     }
///     fn depth_bias(&self) -> Option<f32> {
///         None // Return `Some` to override the depth bias
///     }
///     fn alpha_mode(&self) -> Option<AlphaMode2d> {
///         None // Return `Some` to override the alpha mode
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     // Make sure to add the plugin!
///     app.add_plugins(SpriteMaterialPlugin::<MySpriteMaterial>::default());
/// }
/// ```
///
/// The fragment shader, if overridden, can import functions from `bevy_sprite_render::sprite_mesh::functions`, including:
/// ```wesl
/// // Applies all the transformations to the UV and samples the sprite's final color, including tint and alpha discard.
/// fn sample_final_color(uv: vec2<f32>, instance_index: u32) -> vec4<f32>;
///
/// // Applies all the necessary transformations to the UV and samples the sprite's texture.
/// fn sample_sprite_texture(uv: vec2<f32>, instance_index: u32) -> vec4<f32>;
///
/// // Applies the tint and alpha discard on the sprite's color.
/// fn get_final_color(sprite_color: vec4<f32>, instance_index: u32) -> vec4<f32>;
/// ```
///
/// See the `sprite_material` example for a complete usage example of sprite materials.
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
struct SpriteMaterialCache<M>(HashMap<AssetId<M>, super::SpriteMaterialCache<SpriteExt<M>>>)
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
    mut cache: ResMut<SpriteMaterialCache<M>>,
    mut asset_events: MessageReader<AssetEvent<M>>,
    mut ext_events: MessageReader<AssetEvent<SpriteExt<M>>>,
) where
    M::Data: Clone,
    M: Asset + MaterialExtension2d,
{
    for message in asset_events.read() {
        if let AssetEvent::Removed { id } = *message {
            cache.retain(|&key_id, _| key_id != id);
        }
    }

    for event in ext_events.read() {
        for el in cache.values_mut() {
            el.clean(event);
        }
    }
}

type SpriteExt<M> = ExtendedMaterial2d<SpriteMeshMaterial, SpriteMaterialExtension<M>>;

#[cfg(test)]
mod tests {
    use bevy_asset::DirectAssetAccessExt;

    use super::*;

    #[derive(AsBindGroup, Asset, Reflect, Hash, PartialEq, Clone, Copy)]
    struct TestMaterial {}

    impl MaterialExtension2d for TestMaterial {}

    #[derive(AsBindGroup, Asset, Reflect, Hash, PartialEq, Clone, Copy)]
    struct TestMaterial2 {}

    impl MaterialExtension2d for TestMaterial2 {}

    #[test]
    fn reused_handles() {
        let mut app = test_app();

        let sprite_mesh = SpriteMesh::default();
        let handle = app.world_mut().add_asset::<TestMaterial>(TestMaterial {});

        let one = app
            .world_mut()
            .spawn((sprite_mesh.clone(), SpriteMaterial(handle.clone())))
            .id();

        let two = app
            .world_mut()
            .spawn((sprite_mesh.clone(), SpriteMaterial(handle.clone())))
            .id();

        app.update();
        let mat1 = app
            .world()
            .entity(one)
            .get::<MeshMaterial2d<SpriteExt<TestMaterial>>>()
            .unwrap()
            .0
            .clone();

        let mat2 = app
            .world()
            .entity(two)
            .get::<MeshMaterial2d<SpriteExt<TestMaterial>>>()
            .unwrap()
            .0
            .clone();

        assert_eq!(mat1, mat2);
    }

    #[test]
    fn material_insertion() {
        let mut app = test_app();

        let handle = app.world_mut().add_asset::<TestMaterial>(TestMaterial {});
        let handle2 = app.world_mut().add_asset::<TestMaterial2>(TestMaterial2 {});

        let entity = app.world_mut().spawn(SpriteMesh::default()).id();
        app.update();

        let test = |app: &mut App, sprite: bool, test: bool, test2: bool| {
            app.update();
            let ent = app.world().entity(entity);
            assert_eq!(
                ent.get::<MeshMaterial2d<SpriteMeshMaterial>>().is_some(),
                sprite
            );
            assert_eq!(
                ent.get::<MeshMaterial2d<SpriteExt<TestMaterial>>>()
                    .is_some(),
                test
            );
            assert_eq!(
                ent.get::<MeshMaterial2d<SpriteExt<TestMaterial2>>>()
                    .is_some(),
                test2
            );
        };

        test(&mut app, true, false, false);

        app.world_mut()
            .entity_mut(entity)
            .insert(SpriteMaterial(handle));

        test(&mut app, false, true, false);

        app.world_mut()
            .entity_mut(entity)
            .insert(SpriteMaterial(handle2.clone()));

        test(&mut app, false, true, true);

        // make sure re-insertions don't break it
        app.world_mut()
            .entity_mut(entity)
            .insert(SpriteMaterial(handle2.clone()));

        test(&mut app, false, true, true);

        app.world_mut()
            .entity_mut(entity)
            .remove::<SpriteMaterial<TestMaterial2>>();

        test(&mut app, false, true, false);

        app.world_mut()
            .entity_mut(entity)
            .remove::<SpriteMaterial<TestMaterial>>();

        test(&mut app, true, false, false);
    }

    #[test]
    fn cache_cleanup() {
        let mut app = test_app();

        let handle = app.world_mut().add_asset::<TestMaterial>(TestMaterial {});
        let entity = app
            .world_mut()
            .spawn((SpriteMesh::default(), SpriteMaterial(handle)))
            .id();

        app.update();
        assert_eq!(
            app.world()
                .resource::<SpriteMaterialCache<TestMaterial>>()
                .len(),
            1
        );

        app.world_mut().entity_mut(entity).despawn();
        app.update();

        assert_eq!(
            app.world()
                .resource::<SpriteMaterialCache<TestMaterial>>()
                .len(),
            0
        );
    }

    fn test_app() -> App {
        let mut app = App::new();

        app.add_plugins(bevy_asset::AssetPlugin::default())
            .init_asset::<bevy_shader::Shader>()
            .add_plugins((
                bevy_app::TaskPoolPlugin::default(),
                bevy_mesh::MeshPlugin,
                bevy_image::TextureAtlasPlugin,
                crate::SpriteMeshPlugin,
                SpriteMaterialPlugin::<TestMaterial>::default(),
                SpriteMaterialPlugin::<TestMaterial2>::default(),
            ));

        app
    }
}
