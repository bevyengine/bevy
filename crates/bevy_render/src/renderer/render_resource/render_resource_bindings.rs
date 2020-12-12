use super::{BindGroup, BindGroupId, BufferId, SamplerId, TextureId};
use crate::{
    pipeline::{BindGroupDescriptor, BindGroupDescriptorId, PipelineDescriptor},
    renderer::RenderResourceContext,
};
use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_utils::{HashMap, HashSet};
use std::{any::TypeId, ops::Range};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RenderResourceBinding {
    Buffer {
        buffer: BufferId,
        range: Range<u64>,
        dynamic_index: Option<u32>,
    },
    Texture(TextureId),
    Sampler(SamplerId),
}

impl RenderResourceBinding {
    pub fn get_texture(&self) -> Option<TextureId> {
        if let RenderResourceBinding::Texture(texture) = self {
            Some(*texture)
        } else {
            None
        }
    }

    pub fn get_buffer(&self) -> Option<BufferId> {
        if let RenderResourceBinding::Buffer { buffer, .. } = self {
            Some(*buffer)
        } else {
            None
        }
    }

    pub fn is_dynamic_buffer(&self) -> bool {
        matches!(
            self,
            RenderResourceBinding::Buffer {
                dynamic_index: Some(_),
                ..
            }
        )
    }

    pub fn get_sampler(&self) -> Option<SamplerId> {
        if let RenderResourceBinding::Sampler(sampler) = self {
            Some(*sampler)
        } else {
            None
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum BindGroupStatus {
    Changed(BindGroupId),
    Unchanged(BindGroupId),
    NoMatch,
}

// PERF: if the bindings are scoped to a specific pipeline layout, then names could be replaced with indices here for a perf boost
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub struct RenderResourceBindings {
    pub bindings: HashMap<String, RenderResourceBinding>,
    /// A Buffer that contains all attributes a mesh has defined
    pub vertex_attribute_buffer: Option<BufferId>,
    /// A Buffer that is filled with zeros that will be used for attributes required by the shader, but undefined by the mesh.
    pub vertex_fallback_buffer: Option<BufferId>,
    pub index_buffer: Option<BufferId>,
    assets: HashSet<(HandleUntyped, TypeId)>,
    bind_groups: HashMap<BindGroupId, BindGroup>,
    bind_group_descriptors: HashMap<BindGroupDescriptorId, Option<BindGroupId>>,
    dirty_bind_groups: HashSet<BindGroupId>,
    dynamic_bindings_generation: usize,
}

impl RenderResourceBindings {
    pub fn get(&self, name: &str) -> Option<&RenderResourceBinding> {
        self.bindings.get(name)
    }

    pub fn set(&mut self, name: &str, binding: RenderResourceBinding) {
        self.try_set_dirty(name, &binding);
        self.bindings.insert(name.to_string(), binding);
    }

    /// The current "generation" of dynamic bindings. This number increments every time a dynamic binding changes
    pub fn dynamic_bindings_generation(&self) -> usize {
        self.dynamic_bindings_generation
    }

    fn try_set_dirty(&mut self, name: &str, binding: &RenderResourceBinding) {
        if let Some(current_binding) = self.bindings.get(name) {
            if current_binding != binding {
                if current_binding.is_dynamic_buffer() {
                    self.dynamic_bindings_generation += 1;
                }
                // TODO: this is crude. we shouldn't need to invalidate all bind groups
                for id in self.bind_groups.keys() {
                    self.dirty_bind_groups.insert(*id);
                }
            }
        } else {
            // unmatched bind group descriptors might now match
            self.bind_group_descriptors
                .retain(|_, value| value.is_some());
        }
    }

    pub fn extend(&mut self, render_resource_bindings: &RenderResourceBindings) {
        for (name, binding) in render_resource_bindings.bindings.iter() {
            self.set(name, binding.clone());
        }
    }

    pub fn set_index_buffer(&mut self, index_buffer: BufferId) {
        self.index_buffer = Some(index_buffer);
    }

    fn create_bind_group(&mut self, descriptor: &BindGroupDescriptor) -> BindGroupStatus {
        let bind_group = self.build_bind_group(descriptor);
        if let Some(bind_group) = bind_group {
            let id = bind_group.id;
            self.bind_groups.insert(id, bind_group);
            self.bind_group_descriptors.insert(descriptor.id, Some(id));
            BindGroupStatus::Changed(id)
        } else {
            self.bind_group_descriptors.insert(descriptor.id, None);
            BindGroupStatus::NoMatch
        }
    }

    fn update_bind_group_status(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
    ) -> BindGroupStatus {
        if let Some(id) = self.bind_group_descriptors.get(&bind_group_descriptor.id) {
            if let Some(id) = id {
                if self.dirty_bind_groups.contains(id) {
                    self.dirty_bind_groups.remove(id);
                    self.create_bind_group(bind_group_descriptor)
                } else {
                    BindGroupStatus::Unchanged(*id)
                }
            } else {
                BindGroupStatus::NoMatch
            }
        } else {
            self.create_bind_group(bind_group_descriptor)
        }
    }

    pub fn add_asset(&mut self, handle: HandleUntyped, type_id: TypeId) {
        self.dynamic_bindings_generation += 1;
        self.assets.insert((handle, type_id));
    }

    pub fn remove_asset_with_type(&mut self, type_id: TypeId) {
        self.dynamic_bindings_generation += 1;
        self.assets.retain(|(_, current_id)| *current_id != type_id);
    }

    pub fn iter_assets(&self) -> impl Iterator<Item = &(HandleUntyped, TypeId)> {
        self.assets.iter()
    }

    pub fn update_bind_group(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_context: &dyn RenderResourceContext,
    ) -> Option<&BindGroup> {
        let status = self.update_bind_group_status(bind_group_descriptor);
        match status {
            BindGroupStatus::Changed(id) => {
                let bind_group = self
                    .get_bind_group(id)
                    .expect("`RenderResourceSet` was just changed, so it should exist.");
                render_resource_context.create_bind_group(bind_group_descriptor.id, bind_group);
                Some(bind_group)
            }
            BindGroupStatus::Unchanged(id) => {
                // PERF: this is only required because RenderResourceContext::remove_stale_bind_groups doesn't inform RenderResourceBindings
                // when a stale bind group has been removed
                let bind_group = self
                    .get_bind_group(id)
                    .expect("`RenderResourceSet` was just changed, so it should exist.");
                render_resource_context.create_bind_group(bind_group_descriptor.id, bind_group);
                Some(bind_group)
            }
            BindGroupStatus::NoMatch => {
                // ignore unchanged / unmatched render resource sets
                None
            }
        }
    }

    pub fn update_bind_groups(
        &mut self,
        pipeline: &PipelineDescriptor,
        render_resource_context: &dyn RenderResourceContext,
    ) {
        let layout = pipeline.get_layout().unwrap();
        for bind_group_descriptor in layout.bind_groups.iter() {
            self.update_bind_group(bind_group_descriptor, render_resource_context);
        }
    }

    pub fn get_bind_group(&self, id: BindGroupId) -> Option<&BindGroup> {
        self.bind_groups.get(&id)
    }

    pub fn get_descriptor_bind_group(&self, id: BindGroupDescriptorId) -> Option<&BindGroup> {
        self.bind_group_descriptors
            .get(&id)
            .and_then(|bind_group_id| {
                if let Some(bind_group_id) = bind_group_id {
                    self.get_bind_group(*bind_group_id)
                } else {
                    None
                }
            })
    }

    fn build_bind_group(&self, bind_group_descriptor: &BindGroupDescriptor) -> Option<BindGroup> {
        let mut bind_group_builder = BindGroup::build();
        for binding_descriptor in bind_group_descriptor.bindings.iter() {
            if let Some(binding) = self.get(&binding_descriptor.name) {
                bind_group_builder =
                    bind_group_builder.add_binding(binding_descriptor.index, binding.clone());
            } else {
                return None;
            }
        }

        Some(bind_group_builder.finish())
    }

    pub fn iter_dynamic_bindings(&self) -> impl Iterator<Item = &str> {
        self.bindings
            .iter()
            .filter(|(_, binding)| {
                matches!(
                    binding,
                    RenderResourceBinding::Buffer {
                        dynamic_index: Some(_),
                        ..
                    }
                )
            })
            .map(|(name, _)| name.as_str())
    }
}

#[derive(Debug, Default)]
pub struct AssetRenderResourceBindings {
    pub bindings: HashMap<HandleUntyped, RenderResourceBindings>,
}

impl AssetRenderResourceBindings {
    pub fn get<T: Asset>(&self, handle: &Handle<T>) -> Option<&RenderResourceBindings> {
        self.get_untyped(&handle.clone_weak_untyped())
    }

    pub fn get_untyped(&self, handle: &HandleUntyped) -> Option<&RenderResourceBindings> {
        self.bindings.get(handle)
    }

    pub fn get_or_insert_mut<T: Asset>(
        &mut self,
        handle: &Handle<T>,
    ) -> &mut RenderResourceBindings {
        self.bindings
            .entry(handle.clone_weak_untyped())
            .or_insert_with(RenderResourceBindings::default)
    }

    pub fn get_mut<T: Asset>(&mut self, handle: &Handle<T>) -> Option<&mut RenderResourceBindings> {
        self.get_mut_untyped(&handle.clone_weak_untyped())
    }

    pub fn get_mut_untyped(
        &mut self,
        handle: &HandleUntyped,
    ) -> Option<&mut RenderResourceBindings> {
        self.bindings.get_mut(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{BindType, BindingDescriptor, BindingShaderStage, UniformProperty};

    #[test]
    fn test_bind_groups() {
        let bind_group_descriptor = BindGroupDescriptor::new(
            0,
            vec![
                BindingDescriptor {
                    index: 0,
                    name: "a".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: false,
                        property: UniformProperty::Struct(vec![UniformProperty::Mat4]),
                    },
                    shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
                },
                BindingDescriptor {
                    index: 1,
                    name: "b".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: false,
                        property: UniformProperty::Float,
                    },
                    shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
                },
            ],
        );

        let resource1 = RenderResourceBinding::Texture(TextureId::new());
        let resource2 = RenderResourceBinding::Texture(TextureId::new());
        let resource3 = RenderResourceBinding::Texture(TextureId::new());
        let resource4 = RenderResourceBinding::Texture(TextureId::new());

        let mut bindings = RenderResourceBindings::default();
        bindings.set("a", resource1.clone());
        bindings.set("b", resource2.clone());

        let mut different_bindings = RenderResourceBindings::default();
        different_bindings.set("a", resource3.clone());
        different_bindings.set("b", resource4.clone());

        let mut equal_bindings = RenderResourceBindings::default();
        equal_bindings.set("a", resource1.clone());
        equal_bindings.set("b", resource2.clone());

        let status = bindings.update_bind_group_status(&bind_group_descriptor);
        let id = if let BindGroupStatus::Changed(id) = status {
            id
        } else {
            panic!("Expected a changed bind group.");
        };

        let different_bind_group_status =
            different_bindings.update_bind_group_status(&bind_group_descriptor);
        if let BindGroupStatus::Changed(different_bind_group_id) = different_bind_group_status {
            assert_ne!(
                id, different_bind_group_id,
                "different bind group shouldn't have the same id"
            );
            different_bind_group_id
        } else {
            panic!("Expected a changed bind group.");
        };

        let equal_bind_group_status =
            equal_bindings.update_bind_group_status(&bind_group_descriptor);
        if let BindGroupStatus::Changed(equal_bind_group_id) = equal_bind_group_status {
            assert_eq!(
                id, equal_bind_group_id,
                "equal bind group should have the same id"
            );
        } else {
            panic!("Expected a changed bind group.");
        };

        let mut unmatched_bindings = RenderResourceBindings::default();
        unmatched_bindings.set("a", resource1.clone());
        let unmatched_bind_group_status =
            unmatched_bindings.update_bind_group_status(&bind_group_descriptor);
        assert_eq!(unmatched_bind_group_status, BindGroupStatus::NoMatch);
    }
}
