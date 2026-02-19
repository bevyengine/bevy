//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::{
    asset::{AsAssetId, AssetEventSystems},
    core_pipeline::core_3d::Opaque3d,
    ecs::system::{
        lifetimeless::{SRes, SResMut},
        SystemChangeTick, SystemParamItem,
    },
    pbr::{
        DrawMaterial, EntitiesNeedingSpecialization, EntitySpecializationTicks,
        MaterialBindGroupAllocator, MaterialBindGroupAllocators, MaterialDrawFunction,
        MaterialFragmentShader, MaterialProperties, PreparedMaterial, RenderMaterialBindings,
        RenderMaterialInstance, RenderMaterialInstances, SpecializedMaterialPipelineCache,
    },
    platform::collections::hash_map::Entry,
    prelude::*,
    render::{
        erased_render_asset::{ErasedRenderAsset, ErasedRenderAssetPlugin, PrepareAssetError},
        render_asset::RenderAssets,
        render_phase::DrawFunctions,
        render_resource::{
            binding_types::{sampler, texture_2d},
            AsBindGroup, BindGroupLayout, BindGroupLayoutEntries, BindingResources,
            OwnedBindingResource, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            TextureSampleType, TextureViewDimension, UnpreparedBindGroup,
        },
        renderer::RenderDevice,
        sync_world::MainEntity,
        texture::GpuImage,
        view::ExtractedView,
        Extract, RenderApp, RenderStartup,
    },
    utils::Parallel,
};
use std::{any::TypeId, sync::Arc};

const SHADER_ASSET_PATH: &str = "shaders/manual_material.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ImageMaterialPlugin))
        .add_systems(Startup, setup)
        .run();
}

struct ImageMaterialPlugin;

impl Plugin for ImageMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ImageMaterial>()
            .add_plugins(ErasedRenderAssetPlugin::<ImageMaterial>::default())
            .add_systems(
                PostUpdate,
                check_entities_needing_specialization.after(AssetEventSystems),
            )
            .init_resource::<EntitiesNeedingSpecialization<ImageMaterial>>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_image_material_resources)
            .add_systems(
                ExtractSchedule,
                (
                    extract_image_materials,
                    extract_image_materials_needing_specialization,
                ),
            );
    }
}

fn init_image_material_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut bind_group_allocators: ResMut<MaterialBindGroupAllocators>,
) {
    let bind_group_layout = render_device.create_bind_group_layout(
        "image_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: false }),
                sampler(SamplerBindingType::NonFiltering),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    commands.insert_resource(ImageMaterialBindGroupLayout(bind_group_layout.clone()));
    commands.insert_resource(ImageMaterialBindGroupSampler(sampler));

    bind_group_allocators.insert(
        TypeId::of::<ImageMaterial>(),
        MaterialBindGroupAllocator::new(&render_device, None, None, bind_group_layout, None),
    );
}

#[derive(Resource)]
struct ImageMaterialBindGroupLayout(BindGroupLayout);

#[derive(Resource)]
struct ImageMaterialBindGroupSampler(Sampler);

#[derive(Component)]
struct ImageMaterial3d(Handle<ImageMaterial>);

impl AsAssetId for ImageMaterial3d {
    type Asset = ImageMaterial;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ImageMaterial {
    image: Handle<Image>,
}

impl ErasedRenderAsset for ImageMaterial {
    type SourceAsset = ImageMaterial;
    type ErasedAsset = PreparedMaterial;
    type Param = (
        SRes<DrawFunctions<Opaque3d>>,
        SRes<ImageMaterialBindGroupLayout>,
        SRes<AssetServer>,
        SResMut<MaterialBindGroupAllocators>,
        SResMut<RenderMaterialBindings>,
        SRes<RenderAssets<GpuImage>>,
        SRes<ImageMaterialBindGroupSampler>,
    );

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        asset_id: AssetId<Self::SourceAsset>,
        (
            opaque_draw_functions,
            material_layout,
            asset_server,
            bind_group_allocators,
            render_material_bindings,
            gpu_images,
            image_material_sampler,
        ): &mut SystemParamItem<Self::Param>,
    ) -> std::result::Result<Self::ErasedAsset, PrepareAssetError<Self::SourceAsset>> {
        let material_layout = material_layout.0.clone();
        let draw_function_id = opaque_draw_functions.read().id::<DrawMaterial>();
        let bind_group_allocator = bind_group_allocators
            .get_mut(&TypeId::of::<ImageMaterial>())
            .unwrap();
        let Some(image) = gpu_images.get(&source_asset.image) else {
            return Err(PrepareAssetError::RetryNextUpdate(source_asset));
        };
        let unprepared = UnpreparedBindGroup {
            bindings: BindingResources(vec![
                (
                    0,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        image.texture_view.clone(),
                    ),
                ),
                (
                    1,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::NonFiltering,
                        image_material_sampler.0.clone(),
                    ),
                ),
            ]),
        };
        let binding = match render_material_bindings.entry(asset_id.into()) {
            Entry::Occupied(mut occupied_entry) => {
                bind_group_allocator.free(*occupied_entry.get());
                let new_binding =
                    bind_group_allocator.allocate_unprepared(unprepared, &material_layout);
                *occupied_entry.get_mut() = new_binding;
                new_binding
            }
            Entry::Vacant(vacant_entry) => *vacant_entry
                .insert(bind_group_allocator.allocate_unprepared(unprepared, &material_layout)),
        };

        let mut properties = MaterialProperties {
            material_layout: Some(material_layout),
            ..Default::default()
        };
        properties.add_draw_function(MaterialDrawFunction, draw_function_id);
        properties.add_shader(MaterialFragmentShader, asset_server.load(SHADER_ASSET_PATH));

        Ok(PreparedMaterial {
            binding,
            properties: Arc::new(properties),
        })
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ImageMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        ImageMaterial3d(materials.add(ImageMaterial {
            image: asset_server.load("branding/icon.png"),
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn extract_image_materials(
    mut material_instances: ResMut<RenderMaterialInstances>,
    changed_meshes_query: Extract<
        Query<
            (Entity, &ViewVisibility, &ImageMaterial3d),
            Or<(Changed<ViewVisibility>, Changed<ImageMaterial3d>)>,
        >,
    >,
) {
    let last_change_tick = material_instances.current_change_tick;

    for (entity, view_visibility, material) in &changed_meshes_query {
        if view_visibility.get() {
            material_instances.instances.insert(
                entity.into(),
                RenderMaterialInstance {
                    asset_id: material.0.id().untyped(),
                    last_change_tick,
                },
            );
        } else {
            material_instances
                .instances
                .remove(&MainEntity::from(entity));
        }
    }
}

fn check_entities_needing_specialization(
    needs_specialization: Query<
        Entity,
        (
            Or<(
                Changed<Mesh3d>,
                AssetChanged<Mesh3d>,
                Changed<ImageMaterial3d>,
                AssetChanged<ImageMaterial3d>,
            )>,
            With<ImageMaterial3d>,
        ),
    >,
    mut par_local: Local<Parallel<Vec<Entity>>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<ImageMaterial>>,
) {
    entities_needing_specialization.clear();

    needs_specialization
        .par_iter()
        .for_each(|entity| par_local.borrow_local_mut().push(entity));

    par_local.drain_into(&mut entities_needing_specialization);
}

fn extract_image_materials_needing_specialization(
    entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<ImageMaterial>>>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks>,
    mut removed_mesh_material_components: Extract<RemovedComponents<ImageMaterial3d>>,
    mut specialized_material_pipeline_cache: ResMut<SpecializedMaterialPipelineCache>,
    views: Query<&ExtractedView>,
    ticks: SystemChangeTick,
) {
    // Clean up any despawned entities, we do this first in case the removed material was re-added
    // the same frame, thus will appear both in the removed components list and have been added to
    // the `EntitiesNeedingSpecialization` collection by triggering the `Changed` filter
    for entity in removed_mesh_material_components.read() {
        entity_specialization_ticks.remove(&MainEntity::from(entity));
        for view in views {
            if let Some(cache) =
                specialized_material_pipeline_cache.get_mut(&view.retained_view_entity)
            {
                cache.remove(&MainEntity::from(entity));
            }
        }
    }

    for entity in entities_needing_specialization.iter() {
        // Update the entity's specialization tick with this run's tick
        entity_specialization_ticks.insert((*entity).into(), ticks.this_run());
    }
}
