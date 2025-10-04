use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    error::Result,
    relationship::Relationship,
    template::{ErasedTemplate, Template},
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
}

impl ResolvedScene {
    pub fn spawn(&mut self, entity: &mut EntityWorldMut) -> Result {
        for template in self.templates.iter_mut() {
            template.apply(entity)?;
        }

        for related in self.related.values_mut() {
            let target = entity.id();
            entity.world_scope(|world| -> Result {
                for scene in &mut related.scenes {
                    let mut entity = world.spawn_empty();
                    (related.insert)(&mut entity, target);
                    // PERF: this will result in an archetype move
                    scene.spawn(&mut entity)?;
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
