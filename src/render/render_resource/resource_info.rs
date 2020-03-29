use super::RenderResourceAssignmentsId;
use crate::render::render_resource::BufferUsage;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct BufferArrayInfo {
    pub item_count: usize,
    pub item_size: usize,
    pub item_capacity: usize,
    pub indices: HashMap<RenderResourceAssignmentsId, usize>,
    pub current_index: usize,
}

impl BufferArrayInfo {
    pub fn get_index(&self, id: RenderResourceAssignmentsId) -> Option<usize> {
        self.indices.get(&id).map(|offset| *offset)
    }

    pub fn get_or_assign_index(&mut self, id: RenderResourceAssignmentsId) -> usize {
        if let Some(offset) = self.indices.get(&id) {
            *offset
        } else {
            if self.current_index == self.item_capacity {
                panic!("no empty slots available in array");
            }

            let index = self.current_index;
            self.indices.insert(id, index);
            self.current_index += 1;
            index
        }
    }
}

#[derive(Debug)]
pub struct BufferInfo {
    pub size: usize,
    pub buffer_usage: BufferUsage,
    pub array_info: Option<BufferArrayInfo>,
    pub is_dynamic: bool,
}

impl Default for BufferInfo {
    fn default() -> Self {
        BufferInfo {
            size: 0,
            buffer_usage: BufferUsage::NONE,
            array_info: None,
            is_dynamic: false,
        }
    }
}

#[derive(Debug)]
pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture,
    Sampler,
}
