// pathfinder/renderer/src/allocator.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A simple quadtree-based texture allocator.

use crate::gpu_data::{TextureLocation, TexturePageId};
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2f, vec2i};

const ATLAS_TEXTURE_LENGTH: u32 = 1024;

#[derive(Clone, Debug)]
pub struct TextureAllocator {
    pages: Vec<TexturePage>,
}

#[derive(Clone, Debug)]
pub struct TexturePage {
    allocator: TexturePageAllocator,
    is_new: bool,
}

#[derive(Clone, Debug)]
pub enum TexturePageAllocator {
    // An atlas allocated with our quadtree allocator.
    Atlas(TextureAtlasAllocator),
    // A single image.
    Image { size: Vector2I },
}

#[derive(Clone, Debug)]
pub struct TextureAtlasAllocator {
    root: TreeNode,
    size: u32,
}

#[derive(Clone, Debug)]
enum TreeNode {
    EmptyLeaf,
    FullLeaf,
    // Top left, top right, bottom left, and bottom right, in that order.
    Parent([Box<TreeNode>; 4]),
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(dead_code)]
pub enum AllocationMode {
    Atlas,
    OwnPage,
}

impl TextureAllocator {
    #[inline]
    pub fn new() -> TextureAllocator {
        TextureAllocator { pages: vec![] }
    }

    pub fn allocate(&mut self, requested_size: Vector2I, mode: AllocationMode) -> TextureLocation {
        // If requested, or if the image is too big, use a separate page.
        if mode == AllocationMode::OwnPage ||
                requested_size.x() > ATLAS_TEXTURE_LENGTH as i32 ||
                requested_size.y() > ATLAS_TEXTURE_LENGTH as i32 {
            return self.allocate_image(requested_size);
        }

        // Try to add to each atlas.
        for (page_index, page) in self.pages.iter_mut().enumerate() {
            match page.allocator {
                TexturePageAllocator::Image { .. } => {}
                TexturePageAllocator::Atlas(ref mut allocator) => {
                    if let Some(rect) = allocator.allocate(requested_size) {
                        return TextureLocation { page: TexturePageId(page_index as u32), rect };
                    }
                }
            }
        }

        // Add a new atlas.
        let page = TexturePageId(self.pages.len() as u32);
        let mut allocator = TextureAtlasAllocator::new();
        let rect = allocator.allocate(requested_size).expect("Allocation failed!");
        self.pages.push(TexturePage {
            is_new: true,
            allocator: TexturePageAllocator::Atlas(allocator),
        });
        TextureLocation { page, rect }
    }

    pub fn allocate_image(&mut self, requested_size: Vector2I) -> TextureLocation {
        let page = TexturePageId(self.pages.len() as u32);
        let rect = RectI::new(Vector2I::default(), requested_size);
        self.pages.push(TexturePage {
            is_new: true,
            allocator: TexturePageAllocator::Image { size: rect.size() },
        });
        TextureLocation { page, rect }
    }

    pub fn page_size(&self, page_id: TexturePageId) -> Vector2I {
        match self.pages[page_id.0 as usize].allocator {
            TexturePageAllocator::Atlas(ref atlas) => Vector2I::splat(atlas.size as i32),
            TexturePageAllocator::Image { size, .. } => size,
        }
    }

    pub fn page_scale(&self, page_id: TexturePageId) -> Vector2F {
        vec2f(1.0, 1.0) / self.page_size(page_id).to_f32()
    }

    pub fn page_is_new(&self, page_id: TexturePageId) -> bool {
        self.pages[page_id.0 as usize].is_new
    }

    pub fn mark_page_as_allocated(&mut self, page_id: TexturePageId) {
        self.pages[page_id.0 as usize].is_new = false;
    }

    #[inline]
    pub fn page_count(&self) -> u32 {
        self.pages.len() as u32
    }
}

impl TextureAtlasAllocator {
    #[inline]
    fn new() -> TextureAtlasAllocator {
        TextureAtlasAllocator::with_length(ATLAS_TEXTURE_LENGTH)
    }

    #[inline]
    fn with_length(length: u32) -> TextureAtlasAllocator {
        TextureAtlasAllocator { root: TreeNode::EmptyLeaf, size: length }
    }

    #[inline]
    fn allocate(&mut self, requested_size: Vector2I) -> Option<RectI> {
        let requested_length =
            (requested_size.x().max(requested_size.y()) as u32).next_power_of_two();
        self.root.allocate(Vector2I::default(), self.size, requested_length)
    }

    #[inline]
    #[allow(dead_code)]
    fn free(&mut self, rect: RectI) {
        let requested_length = rect.width() as u32;
        self.root.free(Vector2I::default(), self.size, rect.origin(), requested_length)
    }

    #[inline]
    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        match self.root {
            TreeNode::EmptyLeaf => true,
            _ => false,
        }
    }
}

impl TreeNode {
    // Invariant: `requested_size` must be a power of two.
    fn allocate(&mut self, this_origin: Vector2I, this_size: u32, requested_size: u32)
                -> Option<RectI> {
        if let TreeNode::FullLeaf = *self {
            // No room here.
            return None;
        }
        if this_size < requested_size {
            // Doesn't fit.
            return None;
        }

        // Allocate here or split, as necessary.
        if let TreeNode::EmptyLeaf = *self {
            // Do we have a perfect fit?
            if this_size == requested_size {
                *self = TreeNode::FullLeaf;
                return Some(RectI::new(this_origin, Vector2I::splat(this_size as i32)));
            }

            // Split.
            *self = TreeNode::Parent([
                Box::new(TreeNode::EmptyLeaf),
                Box::new(TreeNode::EmptyLeaf),
                Box::new(TreeNode::EmptyLeaf),
                Box::new(TreeNode::EmptyLeaf),
            ]);
        }

        // Recurse into children.
        match *self {
            TreeNode::Parent(ref mut kids) => {
                let kid_size = this_size / 2;
                if let Some(origin) = kids[0].allocate(this_origin, kid_size, requested_size) {
                    return Some(origin);
                }
                if let Some(origin) = kids[1].allocate(this_origin + vec2i(kid_size as i32, 0),
                                                       kid_size,
                                                       requested_size) {
                    return Some(origin);
                }
                if let Some(origin) = kids[2].allocate(this_origin + vec2i(0, kid_size as i32),
                                                       kid_size,
                                                       requested_size) {
                    return Some(origin);
                }
                if let Some(origin) = kids[3].allocate(this_origin + kid_size as i32,
                                                       kid_size,
                                                       requested_size) {
                    return Some(origin);
                }

                self.merge_if_necessary();
                return None;
            }
            TreeNode::EmptyLeaf | TreeNode::FullLeaf => unreachable!(),
        }
    }

    #[allow(dead_code)]
    fn free(&mut self,
            this_origin: Vector2I,
            this_size: u32,
            requested_origin: Vector2I,
            requested_size: u32) {
        if this_size <= requested_size {
            if this_size == requested_size && this_origin == requested_origin {
                *self = TreeNode::EmptyLeaf;
            }
            return;
        }

        let child_size = this_size / 2;
        let this_center = this_origin + child_size as i32;

        let child_index;
        let mut child_origin = this_origin;

        if requested_origin.y() < this_center.y() {
            if requested_origin.x() < this_center.x() {
                child_index = 0;
            } else {
                child_index = 1;
                child_origin += vec2i(child_size as i32, 0);
            }
        } else {
            if requested_origin.x() < this_center.x() {
                child_index = 2;
                child_origin += vec2i(0, child_size as i32);
            } else {
                child_index = 3;
                child_origin = this_center;
            }
        }

        match *self {
            TreeNode::Parent(ref mut kids) => {
                kids[child_index].free(child_origin, child_size, requested_origin, requested_size);
                self.merge_if_necessary();
            }
            TreeNode::EmptyLeaf | TreeNode::FullLeaf => unreachable!(),
        }
    }

    fn merge_if_necessary(&mut self) {
        match *self {
            TreeNode::Parent(ref mut kids) => {
                if kids.iter().all(|kid| {
                    match **kid {
                        TreeNode::EmptyLeaf => true,
                        _ => false,
                    }
                }) {
                    *self = TreeNode::EmptyLeaf;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod test {
    use pathfinder_geometry::vector::vec2i;
    use quickcheck;
    use std::u32;

    use super::TextureAtlasAllocator;

    #[test]
    fn test_allocation_and_freeing() {
        quickcheck::quickcheck(prop_allocation_and_freeing_work as
                               fn(u32, Vec<(u32, u32)>) -> bool);

        fn prop_allocation_and_freeing_work(mut length: u32, mut sizes: Vec<(u32, u32)>) -> bool {
            length = u32::next_power_of_two(length).max(1);

            for &mut (ref mut width, ref mut height) in &mut sizes {
                *width = (*width).min(length).max(1);
                *height = (*height).min(length).max(1);
            }

            let mut allocator = TextureAtlasAllocator::with_length(length);
            let mut locations = vec![];
            for &(width, height) in &sizes {
                let size = vec2i(width as i32, height as i32);
                if let Some(location) = allocator.allocate(size) {
                    locations.push(location);
                }
            }

            for location in locations {
                allocator.free(location);
            }

            assert!(allocator.is_empty());

            true
        }
    }
}
