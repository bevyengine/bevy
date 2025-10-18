use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    error::Result,
    relationship::Relationship,
    template::{ErasedTemplate, Template, TemplateContext},
    world::EntityWorldMut,
};
use bevy_utils::TypeIdMap;
use std::any::TypeId;

#[derive(Default)]
pub struct ResolvedScene {
    pub template_indices: TypeIdMap<usize>,
    pub templates: Vec<Box<dyn ErasedTemplate>>,
    // PERF: special casing children probably makes sense here
    pub related: TypeIdMap<ResolvedRelatedScenes>,
    pub entity_references: Vec<(usize, usize)>,
}

impl std::fmt::Debug for ResolvedScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedScene")
            .field("related", &self.related)
            .field("entity_references", &self.entity_references)
            .finish()
    }
}

impl ResolvedScene {
    pub fn apply(&mut self, context: &mut TemplateContext) -> Result {
        if let Some((scope, index)) = self.entity_references.first().copied() {
            context
                .scoped_entities
                .set(context.entity_scopes, scope, index, context.entity.id());
        }
        for template in self.templates.iter_mut() {
            template.apply(context)?;
        }

        for related in self.related.values_mut() {
            let target = context.entity.id();
            context.entity.world_scope(|world| -> Result {
                // TODO: I think we need to scan the scene and resolve entities ahead of time, in order to dedupe? Or is there a way to do that
                // at patch time?
                for scene in &mut related.scenes {
                    let mut entity = if let Some((scope, index)) =
                        scene.entity_references.first().copied()
                    {
                        let entity =
                            context
                                .scoped_entities
                                .get(world, context.entity_scopes, scope, index);
                        world.entity_mut(entity)
                    } else {
                        world.spawn_empty()
                    };
                    (related.insert)(&mut entity, target);
                    // PERF: this will result in an archetype move
                    scene.apply(&mut TemplateContext::new(
                        &mut entity,
                        context.scoped_entities,
                        context.entity_scopes,
                    ))?;
                }
                Ok(())
            })?;
        }

        Ok(())
    }

    pub fn get_or_insert_template<T: Template<Output: Bundle> + Default + Send + Sync + 'static>(
        &mut self,
    ) -> &mut T {
        let index = self
            .template_indices
            .entry(TypeId::of::<T>())
            .or_insert_with(|| {
                let index = self.templates.len();
                self.templates.push(Box::new(T::default()));
                index
            });
        self.templates[*index].downcast_mut::<T>().unwrap()
    }

    pub fn push_template<T: Template<Output: Bundle> + Send + Sync + 'static>(
        &mut self,
        template: T,
    ) {
        self.templates.push(Box::new(template));
    }
}

pub struct ResolvedRelatedScenes {
    pub scenes: Vec<ResolvedScene>,
    pub insert: fn(&mut EntityWorldMut, target: Entity),
}

impl std::fmt::Debug for ResolvedRelatedScenes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedRelatedScenes")
            .field("scenes", &self.scenes)
            .finish()
    }
}

impl ResolvedRelatedScenes {
    pub fn new<R: Relationship>() -> Self {
        Self {
            scenes: Vec::new(),
            insert: |entity, target| {
                entity.insert(R::from(target));
            },
        }
    }
}
