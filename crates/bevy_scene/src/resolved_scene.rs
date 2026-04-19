use crate::{ResolveContext, ScenePatch};
use bevy_asset::{AssetId, AssetPath, Assets, Handle, UntypedAssetId};
use bevy_ecs::{
    bundle::{Bundle, BundleScratch, BundleWriter},
    component::{Component, ComponentsRegistrator},
    entity::Entity,
    error::{BevyError, Result},
    relationship::{Relationship, RelationshipTarget},
    template::{EntityScopes, ScopedEntities, ScopedEntityIndex, Template, TemplateContext},
    world::{EntityWorldMut, World},
};
use bevy_platform::collections::HashSet;
use bevy_utils::TypeIdMap;
use core::any::{Any, TypeId};
use thiserror::Error;

/// A final "spawnable" root [`ResolvedScene`]. This includes the [`EntityScopes`] for the whole tree.
pub struct ResolvedSceneRoot {
    /// The root [`ResolvedScene`].
    pub scene: ResolvedScene,
    /// The [`EntityScopes`] associated with the `root` [`ResolvedScene`].
    pub entity_scopes: EntityScopes,
}

impl ResolvedSceneRoot {
    /// This will spawn a new [`Entity`], then call [`ResolvedSceneRoot::apply`] on it.
    /// If this fails mid-spawn, the intermediate entity will be despawned.
    pub fn spawn<'w>(&self, world: &'w mut World) -> Result<EntityWorldMut<'w>, ApplySceneError> {
        let mut entity = world.spawn_empty();
        let result = self.apply(&mut entity, &mut BundleScratch::default());
        match result {
            Ok(_) => Ok(entity),
            Err(err) => {
                entity.despawn();
                Err(err)
            }
        }
    }

    /// Applies this scene to the given [`EntityWorldMut`].
    ///
    /// This will apply all of the [`Template`]s in this root [`ResolvedScene`] to the entity. It will also
    /// spawn all of this [`ResolvedScene`]'s related entities.
    ///
    /// If this root [`ResolvedScene`] inherits from another scene, that scene will be applied _first_.
    pub fn apply(
        &self,
        entity: &mut EntityWorldMut,
        bundle_scratch: &mut BundleScratch,
    ) -> Result<(), ApplySceneError> {
        let mut scoped_entities = self.new_scoped_entities();
        let mut context = TemplateContext::new(entity, &mut scoped_entities, &self.entity_scopes);

        let result = self.scene.apply(&mut context, bundle_scratch);
        if !bundle_scratch.is_empty() {
            // SAFETY: Components comes from the same world as the `context` passed in to self.scene.apply above
            unsafe {
                bundle_scratch.manual_drop(entity.world().components());
            }
        }
        result
    }

    fn new_scoped_entities(&self) -> ScopedEntities {
        ScopedEntities::new(self.entity_scopes.entity_len())
    }
}

/// A final "spawnable" root list of [`ResolvedScene`]s. This includes the [`EntityScopes`] for the whole graph of entities.
pub struct ResolvedSceneListRoot {
    /// The root [`ResolvedScene`] list.
    pub scenes: Vec<ResolvedScene>,
    /// The [`EntityScopes`] associated with the `root` [`ResolvedScene`].
    pub entity_scopes: EntityScopes,
}

impl ResolvedSceneListRoot {
    /// Spawns a new [`Entity`] for each [`ResolvedScene`] in the list, and applies that [`ResolvedScene`] to them.
    pub fn spawn<'w>(&self, world: &'w mut World) -> Result<Vec<Entity>, ApplySceneError> {
        self.spawn_with(world, |_| {})
    }

    pub(crate) fn spawn_with(
        &self,
        world: &mut World,
        func: impl Fn(&mut EntityWorldMut),
    ) -> Result<Vec<Entity>, ApplySceneError> {
        let mut entities = Vec::new();
        let mut scoped_entities = ScopedEntities::new(self.entity_scopes.entity_len());
        let mut bundle_scratch = BundleScratch::default();
        for scene in self.scenes.iter() {
            let mut entity = if let Some(scoped_entity_index) =
                scene.entity_indices.first().copied()
            {
                let entity = scoped_entities.get(world, &self.entity_scopes, scoped_entity_index);
                world.entity_mut(entity)
            } else {
                world.spawn_empty()
            };

            func(&mut entity);
            entities.push(entity.id());
            let result = scene.apply(
                &mut TemplateContext::new(&mut entity, &mut scoped_entities, &self.entity_scopes),
                &mut bundle_scratch,
            );
            if let Err(err) = result {
                // SAFETY: Components comes from the same world as the `context` passed in to self.scene.apply above
                unsafe {
                    bundle_scratch.manual_drop(entity.world().components());
                }
                return Err(err);
            }
        }

        Ok(entities)
    }
}

/// A final resolved scene (usually produced by calling [`Scene::resolve`]). This consists of:
/// 1. A collection of [`Template`]s to apply to a spawned [`Entity`], which are stored as [`ErasedComponentTemplate`]s and [`ErasedBundleTemplate`]s.
/// 2. A collection of [`RelatedResolvedScenes`], which will be spawned as "related" entities (ex: [`Children`] entities).
/// 3. The inherited [`ScenePatch`] if it exists.
///
/// This uses "copy-on-write" behavior for inherited scenes. If a [`Template`] that the inherited scene has is requested, it will be
/// cloned (using [`Template::clone_template`]) and added to the current [`ResolvedScene`].
///
/// When applying this [`ResolvedScene`] to an [`Entity`], the inherited scene (including its children) is applied _first_. _Then_ this
/// [`ResolvedScene`] is applied.
///
/// [`Scene::resolve`]: crate::Scene::resolve
/// [`Children`]: bevy_ecs::hierarchy::Children
#[derive(Default)]
pub struct ResolvedScene {
    /// The collection of component [`Template`]s to apply to a spawned [`Entity`]. This can have multiple copies of the same [`Template`].
    component_templates: Vec<Box<dyn ErasedComponentTemplate>>,
    /// The collection of Bundle templates to apply to a spawned [`Entity`].
    bundle_templates: Vec<Box<dyn ErasedBundleTemplate>>,
    /// The collection of [`RelatedResolvedScenes`], which will be spawned as "related" entities (ex: [`Children`] entities).
    ///
    /// [`Children`]: bevy_ecs::hierarchy::Children
    // PERF: special casing Children might make sense here to avoid hashing
    related: TypeIdMap<RelatedResolvedScenes>,
    /// The inherited [`ScenePatch`] to apply _first_ before applying this [`ResolvedScene`].
    inherited: Option<InheritedSceneInfo>,
    /// A [`TypeId`] to `templates` index mapping. If a [`Template`] is intended to be shared / patched across scenes, it should be registered
    /// here.
    template_indices: TypeIdMap<usize>,
    /// A list of all [`ScopedEntityIndex`] values associated with this entity. There can be more than one if this scene uses
    /// "flattened" inheritance.
    pub entity_indices: Vec<ScopedEntityIndex>,
}

impl core::fmt::Debug for ResolvedScene {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResolvedScene")
            .field("inherited", &self.inherited)
            .field("template_types", &self.template_indices.keys())
            .field("related", &self.related)
            .field("entity_indices", &self.entity_indices)
            .finish()
    }
}

impl ResolvedScene {
    /// Applies this scene to the given [`TemplateContext`] (which holds an already-spawned [`EntityWorldMut`]).
    ///
    /// This will apply all of the [`Template`]s in this [`ResolvedScene`] to the entity in the [`TemplateContext`]. It will also
    /// spawn all of this [`ResolvedScene`]'s related entities.
    ///
    /// If this [`ResolvedScene`] inherits from another scene, that scene will be applied _first_.
    fn apply(
        &self,
        context: &mut TemplateContext,
        bundle_scratch: &mut BundleScratch,
    ) -> Result<(), ApplySceneError> {
        self.apply_with(context, bundle_scratch, |_, _| {})
    }

    /// Applies this scene to the given [`TemplateContext`] (which holds an already-spawned [`EntityWorldMut`]).
    ///
    /// This will apply all of the [`Template`]s in this [`ResolvedScene`] to the entity in the [`TemplateContext`]. It will also
    /// spawn all of this [`ResolvedScene`]'s related entities.
    ///
    /// If this [`ResolvedScene`] inherits from another scene, that scene will be applied _first_.
    ///
    /// This will call `writer_ops` right before calling [`BundleWriter::write`]. This will pass in the `context` value,
    /// which is the same context used to write all of the scene components to the [`BundleWriter`]. This ensures that
    /// writing to [`BundleWriter`] with the [`TemplateContext`] is safe (although those functions, if they are called, are still
    /// unsafe functions / the caller should verify they are using the passed in `context`).
    fn apply_with(
        &self,
        context: &mut TemplateContext,
        bundle_scratch: &mut BundleScratch,
        writer_ops: impl FnOnce(&mut TemplateContext, &mut BundleWriter),
    ) -> Result<(), ApplySceneError> {
        let mut bundle_writer = bundle_scratch.writer();
        if let Some(inherited) = &self.inherited {
            let scene_patches = context.resource::<Assets<ScenePatch>>();
            let Some(patch) = scene_patches.get(&inherited.handle) else {
                return Err(ApplySceneError::MissingInheritedScene {
                    path: inherited.handle.path().cloned(),
                    id: inherited.handle.id(),
                });
            };
            let Some(resolved_inherited) = &patch.resolved else {
                return Err(ApplySceneError::UnresolvedInheritedScene {
                    path: inherited.handle.path().cloned(),
                    id: inherited.handle.id(),
                });
            };
            let resolved_inherited = resolved_inherited.clone();
            let mut inherited_scoped_entities = resolved_inherited.new_scoped_entities();
            let mut inherited_context = TemplateContext::new(
                context.entity,
                &mut inherited_scoped_entities,
                &resolved_inherited.entity_scopes,
            );
            // SAFETY: bundle_writer is used with the same World across all template.apply calls,
            // and the next bundle_writer.write call
            unsafe {
                resolved_inherited
                    .scene
                    .apply_templates_without_bundle_write(
                        &mut inherited_context,
                        &mut bundle_writer,
                        // this will skip building / inserting templates that
                        // have local copies in the current scene
                        // (inherited templates are copy-on-write)()
                        &inherited.duplicate_templates,
                    )
                    .map_err(|e| ApplySceneError::InheritedSceneApplyError {
                        inherited: inherited.handle.path().cloned(),
                        error: Box::new(e),
                    })?;
                self.apply_templates_without_bundle_write(context, &mut bundle_writer, ())?;
                // SAFETY: World is only used for component registration, which does not affect
                // the entity location
                let components = &mut context.entity.world_mut().components_registrator();
                // This inserts empty RelationshipTarget collections to avoid archetype moves when then related entities are spawned
                // It pre-allocates space in the collection to avoid reallocs as related entities are added.
                for related in self.related.values() {
                    (related.insert_relationship_target)(
                        &mut bundle_writer,
                        components,
                        related.scenes.len(),
                    );
                }

                (writer_ops)(context, &mut bundle_writer);

                bundle_writer.write(context.entity);

                let mut inherited_context = TemplateContext::new(
                    context.entity,
                    &mut inherited_scoped_entities,
                    &resolved_inherited.entity_scopes,
                );
                resolved_inherited
                    .scene
                    .apply_related(&mut inherited_context, bundle_scratch)?;
                self.apply_related(context, bundle_scratch)?;
            }
        } else {
            // SAFETY: bundle_writer was used with the same World across all cases in this function,
            unsafe {
                self.apply_templates_without_bundle_write(context, &mut bundle_writer, ())?;
                // SAFETY: World is only used for component registration, which does not affect
                // the entity location
                let components = &mut context.entity.world_mut().components_registrator();
                // This inserts empty RelationshipTarget collections to avoid archetype moves when then related entities are spawned
                // It pre-allocates space in the collection to avoid reallocs as related entities are added.
                for related in self.related.values() {
                    (related.insert_relationship_target)(
                        &mut bundle_writer,
                        components,
                        related.scenes.len(),
                    );
                }
                (writer_ops)(context, &mut bundle_writer);
                bundle_writer.write(context.entity);
                self.apply_related(context, bundle_scratch)?;
            }
        };

        Ok(())
    }

    fn set_current_entity_in_scope(&self, context: &mut TemplateContext) {
        if let Some(scoped_entity_index) = self.entity_indices.first().copied() {
            context.scoped_entities.set(
                context.entity_scopes,
                scoped_entity_index,
                context.entity.id(),
            );
        }
    }

    /// # Safety
    ///
    /// `bundle_writer` must either be empty or only contain components registered with the given
    /// `context`'s World.
    unsafe fn apply_templates_without_bundle_write(
        &self,
        context: &mut TemplateContext,
        bundle_writer: &mut BundleWriter,
        skip_templates: impl SkipTemplate,
    ) -> Result<(), ApplySceneError> {
        self.set_current_entity_in_scope(context);
        for template in &self.component_templates {
            if skip_templates.should_skip((**template).type_id()) {
                continue;
            }
            // SAFETY: bundle_writer is used with the same World across all template.apply calls,
            // and the next bundle_writer.write call
            unsafe {
                template
                    .apply(context, bundle_writer)
                    .map_err(ApplySceneError::TemplateBuildError)?;
            }
        }

        for template in &self.bundle_templates {
            // SAFETY: bundle_writer is used with the same World across all template.apply calls,
            // and the next bundle_writer.write call
            unsafe {
                template
                    .apply(context)
                    .map_err(ApplySceneError::TemplateBuildError)?;
            }
        }
        Ok(())
    }

    fn apply_related(
        &self,
        context: &mut TemplateContext,
        bundle_scratch: &mut BundleScratch,
    ) -> Result<(), ApplySceneError> {
        for related_resolved_scenes in self.related.values() {
            let target = context.entity.id();
            context
                .entity
                .world_scope(|world| -> Result<(), ApplySceneError> {
                    for (index, scene) in related_resolved_scenes.scenes.iter().enumerate() {
                        let mut entity = if let Some(scoped_entity_index) =
                            scene.entity_indices.first().copied()
                        {
                            let entity = context.scoped_entities.get(
                                world,
                                context.entity_scopes,
                                scoped_entity_index,
                            );
                            world.entity_mut(entity)
                        } else {
                            world.spawn_empty()
                        };

                        scene
                            .apply_with(
                                &mut TemplateContext::new(
                                    &mut entity,
                                    context.scoped_entities,
                                    context.entity_scopes,
                                ),
                                bundle_scratch,
                                |context, bundle_writer| {
                                    // SAFETY: `context` is used to write all previous `bundle_writer` components
                                    // and is also used to write this relationship component
                                    unsafe {
                                        (related_resolved_scenes.insert_relationship)(
                                            bundle_writer,
                                            // SAFETY: World is only used for component registration, which does not affect
                                            // the entity location
                                            &mut context
                                                .entity
                                                .world_mut()
                                                .components_registrator(),
                                            target,
                                        );
                                    }
                                },
                            )
                            .map_err(|e| ApplySceneError::RelatedSceneError {
                                relationship_type_name: related_resolved_scenes.relationship_name,
                                index,
                                error: Box::new(e),
                            })?;
                    }
                    Ok(())
                })?;
        }

        Ok(())
    }

    /// This will get the [`Template`], if it already exists in this [`ResolvedScene`]. If it doesn't exist,
    /// it will use [`Default`] to create a new [`Template`].
    ///
    /// This uses "copy-on-write" behavior for inherited scenes. If a [`Template`] that the inherited scene has is requested, it will be
    /// cloned (using [`Template::clone_template`]), added to the current [`ResolvedScene`], and returned.
    ///
    /// This will ignore [`Template`]s added to this scene using [`ResolvedScene::push_template`], as these are not registered as the "canonical"
    /// [`Template`] for a given [`TypeId`].
    pub fn get_or_insert_template<
        'a,
        T: Template<Output: Component> + Default + Send + Sync + 'static,
    >(
        &'a mut self,
        context: &mut ResolveContext,
    ) -> &'a mut T {
        (self.get_or_insert_erased_template(context, TypeId::of::<T>(), || Box::new(T::default()))
            as &mut dyn Any)
            // PERF: this could be unchecked, given that we control what is stored here
            // The method isn't stable yet, and it would require making get_or_insert_erased_template unsafe
            .downcast_mut()
            .unwrap()
    }

    /// This will get the [`ErasedComponentTemplate`] for the given [`TypeId`], if it already exists in this [`ResolvedScene`]. If it doesn't exist,
    /// it will use the `default` function to create a new [`ErasedComponentTemplate`]. _For correctness, the [`TypeId`] of the [`Template`] returned
    /// by `default` should match the passed in `type_id`_.
    ///
    /// This uses "copy-on-write" behavior for inherited scenes. If a [`Template`] that the inherited scene has is requested, it will be
    /// cloned (using [`Template::clone_template`]), added to the current [`ResolvedScene`], and returned.
    ///
    /// This will ignore [`Template`]s added to this scene using [`ResolvedScene::push_template`], as these are not registered as the "canonical"
    /// [`Template`] for a given [`TypeId`].
    pub fn get_or_insert_erased_template<'a>(
        &'a mut self,
        context: &mut ResolveContext,
        type_id: TypeId,
        default: fn() -> Box<dyn ErasedComponentTemplate>,
    ) -> &'a mut dyn ErasedComponentTemplate {
        let mut is_inherited = false;
        let index = self.template_indices.entry(type_id).or_insert_with(|| {
            let index = self.component_templates.len();
            let value = if let Some(inherited_patch) = &mut context.inherited
                && let Some(resolved_inherited) = &inherited_patch.resolved
                && let Some(inherited_template) =
                    resolved_inherited.scene.get_direct_erased_template(type_id)
            {
                is_inherited = true;
                inherited_template.clone_template()
            } else {
                default()
            };
            self.component_templates.push(value);
            index
        });
        let template = self
            .component_templates
            .get_mut(*index)
            .map(|value| &mut **value)
            .unwrap();

        if is_inherited {
            self.inherited
                .as_mut()
                .unwrap()
                .duplicate_templates
                .insert(type_id);
        }

        template
    }

    /// Returns the [`ErasedComponentTemplate`] for the given `type_id`, if it exists in this [`ResolvedScene`]. This ignores scene inheritance.
    pub fn get_direct_erased_template(
        &self,
        type_id: TypeId,
    ) -> Option<&dyn ErasedComponentTemplate> {
        let index = self.template_indices.get(&type_id)?;
        Some(&*self.component_templates[*index])
    }

    /// Adds the `template` to the "back" of the [`ResolvedScene`] (it will applied later than earlier [`Template`]s).
    pub fn push_template<T: Template<Output: Component> + Send + Sync + 'static>(
        &mut self,
        template: T,
    ) {
        self.push_template_erased(Box::new(template));
    }

    /// Adds the `template` to the "back" of the [`ResolvedScene`] (it will applied later than earlier [`Template`]s).
    pub fn push_template_erased(&mut self, template: Box<dyn ErasedComponentTemplate>) {
        self.component_templates.push(template);
    }

    /// Adds the `template` to the "back" of the [`ResolvedScene`] (it will applied later than earlier [`Template`]s).
    pub fn push_bundle_template<T: Template<Output: Bundle> + Send + Sync + 'static>(
        &mut self,
        template: T,
    ) {
        self.push_bundle_template_erased(Box::new(template));
    }

    /// Adds the `template` to the "back" of the [`ResolvedScene`] (it will applied later than earlier [`Template`]s).
    pub fn push_bundle_template_erased(&mut self, template: Box<dyn ErasedBundleTemplate>) {
        self.bundle_templates.push(template);
    }
    /// This will return the existing [`RelatedResolvedScenes`], if it exists. If not, a new empty [`RelatedResolvedScenes`] will be inserted and returned.
    ///
    /// This is used to add new related scenes and read existing related scenes.
    pub fn get_or_insert_related_resolved_scenes<R: Relationship>(
        &mut self,
    ) -> &mut RelatedResolvedScenes {
        self.related
            .entry(TypeId::of::<R>())
            .or_insert_with(RelatedResolvedScenes::new::<R>)
    }

    /// Configures this [`ResolvedScene`] to inherit from the given [`ScenePatch`].
    ///
    /// If this [`ResolvedScene`] already inherits from a scene, it will return [`InheritSceneError::MultipleInheritance`].
    /// If this [`ResolvedScene`] already has [`Template`]s or related scenes, it will return [`InheritSceneError::LateInheritance`].
    pub fn inherit(&mut self, handle: Handle<ScenePatch>) -> Result<(), InheritSceneError> {
        if let Some(inherited) = &self.inherited {
            return Err(InheritSceneError::MultipleInheritance {
                id: inherited.handle.id().untyped(),
                path: inherited.handle.path().cloned(),
            });
        }
        if !(self.component_templates.is_empty() && self.related.is_empty()) {
            return Err(InheritSceneError::LateInheritance {
                id: handle.id().untyped(),
                path: handle.path().cloned(),
            });
        }
        self.inherited = Some(InheritedSceneInfo {
            handle,
            duplicate_templates: HashSet::default(),
        });
        Ok(())
    }
}

/// Information about a [`ResolvedScene`]'s inherited scene.
#[derive(Debug)]
pub(crate) struct InheritedSceneInfo {
    /// The handle of the inherited scene.
    pub(crate) handle: Handle<ScenePatch>,
    /// Template types that occur in _both_ the current scene and its inherited scene.
    /// This is used to skip insertion of these types when applying the inherited
    /// resolved scene.
    pub(crate) duplicate_templates: HashSet<TypeId>,
}

/// The error returned by [`ResolvedScene::inherit`].
#[derive(Error, Debug)]
pub enum InheritSceneError {
    /// Caused when attempting to inherit from a second scene.
    #[error("Attempted to inherit from a second scene (id {id:?}, path: {path:?}), which is not allowed.")]
    MultipleInheritance {
        /// The asset id of the second inherited scene.
        id: UntypedAssetId,
        /// The path of the second inherited scene.
        path: Option<AssetPath<'static>>,
    },
    /// Caused when attempting to inherit when a [`ResolvedScene`] already has [`Template`]s or related scenes.
    #[error("Attempted to inherit from (id {id:?}, path: {path:?}), but the resolved scene already has templates. For correctness, inheritance should always come first.")]
    LateInheritance {
        /// The asset id of the scene that was inherited late.
        id: UntypedAssetId,
        /// The path of the scene that was inherited late.
        path: Option<AssetPath<'static>>,
    },
}

/// An error produced when applying a [`ResolvedScene`].
#[derive(Error, Debug)]
pub enum ApplySceneError {
    /// Caused when a [`Template`] fails to build
    #[error("Failed to build a Template in the current Scene: {0}")]
    TemplateBuildError(BevyError),
    /// Caused when the inherited [`ResolvedScene`] fails to apply a [`ResolvedScene`].
    #[error("Failed to apply the inherited Scene (asset path: \"{inherited:?}\"): {error}")]
    InheritedSceneApplyError {
        /// The asset path of the inherited scene that failed to apply.
        inherited: Option<AssetPath<'static>>,
        /// The error that occurred while applying the inherited scene.
        error: Box<ApplySceneError>,
    },
    /// Caused when an inherited scene is not present.
    #[error("The inherited scene (id: {id:?}, path: \"{path:?}\") does not exist.")]
    MissingInheritedScene {
        /// The path of the inherited scene.
        path: Option<AssetPath<'static>>,
        /// The asset id of the inherited scene.
        id: AssetId<ScenePatch>,
    },
    /// Caused when an inherited scene has not been resolved yet.
    #[error("The inherited scene (id: {id:?}, path: \"{path:?}\") has not been resolved yet.")]
    UnresolvedInheritedScene {
        /// The path of the inherited scene.
        path: Option<AssetPath<'static>>,
        /// The asset id of the inherited scene.
        id: AssetId<ScenePatch>,
    },
    /// Caused when a related [`ResolvedScene`] fails to apply.
    #[error(
        "Failed to apply the related {relationship_type_name} Scene at index {index}: {error}"
    )]
    RelatedSceneError {
        /// The type name of the relationship.
        relationship_type_name: &'static str,
        /// The index of the related scene that failed to apply.
        index: usize,
        /// The error that occurred when applying the related scene.
        error: Box<ApplySceneError>,
    },
}

/// A collection of [`ResolvedScene`]s that are related to a given [`ResolvedScene`] by a [`Relationship`].
/// Each [`ResolvedScene`] added here will be spawned as a new [`Entity`] when the "parent" [`ResolvedScene`] is spawned.
pub struct RelatedResolvedScenes {
    /// The related resolved scenes. Each entry in the list corresponds to a new related entity that will be spawned with the given scene.
    pub scenes: Vec<ResolvedScene>,
    /// The function that will be called to add the relationship to the spawned related scene.
    pub insert_relationship:
        unsafe fn(&mut BundleWriter, &mut ComponentsRegistrator, target: Entity),
    /// The function that will be called to add the relationship target to the spawned scene with the given capacity.
    pub insert_relationship_target: unsafe fn(&mut BundleWriter, &mut ComponentsRegistrator, usize),
    /// The type name of the relationship. This is used for more helpful error message.
    pub relationship_name: &'static str,
}

impl core::fmt::Debug for RelatedResolvedScenes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResolvedRelatedScenes")
            .field("scenes", &self.scenes)
            .finish()
    }
}

impl RelatedResolvedScenes {
    /// Creates a new empty [`RelatedResolvedScenes`] for the given relationship type.
    pub fn new<R: Relationship>() -> Self {
        Self {
            scenes: Vec::new(),
            insert_relationship: |bundle_writer, components_registrator, target| {
                // SAFETY: caller ensures bundler_writer is always used with the same World
                unsafe { bundle_writer.push_component(components_registrator, R::from(target)) };
            },
            insert_relationship_target: |bundle_writer, components_registrator, capacity| {
                let relationship_target =
                    <<R as Relationship>::RelationshipTarget as RelationshipTarget>::with_capacity(
                        capacity,
                    );
                // SAFETY: caller ensures bundler_writer is always used with the same World
                unsafe {
                    bundle_writer.push_component(components_registrator, relationship_target);
                };
            },
            relationship_name: core::any::type_name::<R>(),
        }
    }
}

/// A type-erased, object-safe, downcastable version of [`Template`] that produces a [`Component`], which will be added to the
/// given [`BundleWriter`].
pub trait ErasedComponentTemplate: Any + Send + Sync {
    /// Applies this template to the given `entity`.
    ///
    /// # Safety
    ///
    /// `bundle_writer` must always be used with the same World that is stored in `context`. This
    /// is intended to be used by a scene system in a scoped / controlled / easily verifiable context.
    /// If you are calling it outside of that context, you are almost certainly doing something wrong!
    unsafe fn apply(
        &self,
        context: &mut TemplateContext,
        bundle_writer: &mut BundleWriter,
    ) -> Result<(), BevyError>;

    /// Clones this template. See [`Clone`].
    fn clone_template(&self) -> Box<dyn ErasedComponentTemplate>;
}

impl<T: Template<Output: Component> + Send + Sync + 'static> ErasedComponentTemplate for T {
    unsafe fn apply(
        &self,
        context: &mut TemplateContext,
        bundle_writer: &mut BundleWriter,
    ) -> Result<(), BevyError> {
        let component = self.build_template(context)?;
        // SAFETY: world_mut is only used to register components, which does not affect entity location
        let mut components = unsafe { context.entity.world_mut().components_registrator() };
        // SAFETY: The caller verifies that `bundle_writer` is always used with the same World.
        unsafe { bundle_writer.push_component(&mut components, component) };

        Ok(())
    }

    fn clone_template(&self) -> Box<dyn ErasedComponentTemplate> {
        Box::new(Template::clone_template(self))
    }
}

/// A type-erased, object-safe, downcastable version of [`Template`] that produces a [`Bundle`], which will be added
/// immediately to a given `entity`.
pub trait ErasedBundleTemplate: Any + Send + Sync {
    /// Applies this template to the given `entity`.
    ///
    /// # Safety
    ///
    /// `bundle_writer` must always be used with the same World that is stored in `context`. This
    /// is intended to be used by a scene system in a scoped / controlled / easily verifiable context.
    /// If you are calling it outside of that context, you are almost certainly doing something wrong!
    unsafe fn apply(&self, context: &mut TemplateContext) -> Result<(), BevyError>;

    /// Clones this template. See [`Clone`].
    fn clone_template(&self) -> Box<dyn ErasedBundleTemplate>;
}

impl<T: Template<Output: Bundle> + Send + Sync + 'static> ErasedBundleTemplate for T {
    unsafe fn apply(&self, context: &mut TemplateContext) -> Result<(), BevyError> {
        let bundle = self.build_template(context)?;
        context.entity.insert(bundle);
        Ok(())
    }

    fn clone_template(&self) -> Box<dyn ErasedBundleTemplate> {
        Box::new(Template::clone_template(self))
    }
}

/// A filter to skip the template for a given `TypeId`
trait SkipTemplate {
    /// Returns true if the template with `type_id` should be skipped.
    fn should_skip(&self, type_id: TypeId) -> bool;
}

impl SkipTemplate for &HashSet<TypeId> {
    #[inline]
    fn should_skip(&self, type_id: TypeId) -> bool {
        self.contains(&type_id)
    }
}

impl SkipTemplate for () {
    #[inline]
    fn should_skip(&self, _type_id: TypeId) -> bool {
        false
    }
}
