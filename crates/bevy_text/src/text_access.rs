use bevy_ecs::{
    prelude::*,
    system::{Query, SystemParam},
};
use bevy_hierarchy::Children;

use crate::TextStyle;

/// Helper trait for using the [`TextReader`] system param.
pub trait TextSpanAccess: Component {
    /// Gets the text span's string.
    fn read_span(&self) -> &str;
    /// Gets mutable reference to the text span's string.
    fn write_span(&mut self) -> &mut String;
}

#[derive(Resource, Default)]
pub(crate) struct TextIterScratch {
    stack: Vec<(&'static Children, usize)>,
}

impl TextIterScratch {
    fn take<'a, 'b>(&'a mut self) -> Vec<(&'b Children, usize)> {
        core::mem::take(&mut self.stack)
            .into_iter()
            .map(|_| -> (&Children, usize) { unreachable!() })
            .collect()
    }

    fn recover(&mut self, mut stack: Vec<(&Children, usize)>) {
        stack.clear();
        self.stack = stack
            .into_iter()
            .map(|_| -> (&'static Children, usize) { unreachable!() })
            .collect();
    }
}

/// System parameter for reading text spans in a [`TextBlock`].
///
/// `R` is the root text component, and `S` is the text span component on children.
#[derive(SystemParam)]
pub struct TextReader<'w, 's, R: TextSpanAccess, S: TextSpanAccess> {
    scratch: ResMut<'w, TextIterScratch>,
    roots: Query<'w, 's, (&'static R, &'static TextStyle, Option<&'static Children>)>,
    spans: Query<'w, 's, (&'static S, &'static TextStyle, Option<&'static Children>)>,
}

impl<'w, 's, R: TextSpanAccess, S: TextSpanAccess> TextReader<'w, 's, R, S> {
    /// Returns an iterator over text spans in a text block, starting with the root entity.
    pub fn iter<'a>(&'a mut self, root_entity: Entity) -> TextSpanIter<'a, R, S> {
        let stack = self.scratch.take();

        TextSpanIter {
            scratch: &mut self.scratch,
            root_entity: Some(root_entity),
            stack,
            roots: &self.roots,
            spans: &self.spans,
        }
    }

    /// Gets a text span within a text block at a specific index in the flattened span list.
    pub fn get(
        &mut self,
        root_entity: Entity,
        index: usize,
    ) -> Option<(Entity, usize, &str, &TextStyle)> {
        self.iter(root_entity).nth(index)
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    pub fn get_text(&mut self, root_entity: Entity, index: usize) -> Option<&str> {
        self.get(root_entity, index).map(|(_, _, text, _)| text)
    }

    /// Gets the [`TextStyle`] of a text span within a text block at a specific index in the flattened span list.
    pub fn get_style(&mut self, root_entity: Entity, index: usize) -> Option<&TextStyle> {
        self.get(root_entity, index).map(|(_, _, _, style)| style)
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn text(&mut self, root_entity: Entity, index: usize) -> &str {
        self.get_text(root_entity, index).unwrap()
    }

    /// Gets the [`TextStyle`] of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn style(&mut self, root_entity: Entity, index: usize) -> &TextStyle {
        self.get_style(root_entity, index).unwrap()
    }
}

/// Iterator returned by [`TextReader::iter`] and [`TextWriter::iter`].
///
/// Iterates all spans in a text block according to hierarchy traversal order.
/// Does *not* flatten interspersed ghost nodes. Only contiguous spans are traversed.
// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIter<'a, R: TextSpanAccess, S: TextSpanAccess> {
    scratch: &'a mut TextIterScratch,
    root_entity: Option<Entity>,
    /// Stack of (children, next index into children).
    stack: Vec<(&'a Children, usize)>,
    roots: &'a Query<'a, 'a, (&'static R, &'static TextStyle, Option<&'static Children>)>,
    spans: &'a Query<'a, 'a, (&'static S, &'static TextStyle, Option<&'static Children>)>,
}

impl<'a, R: TextSpanAccess, S: TextSpanAccess> Iterator for TextSpanIter<'a, R, S> {
    /// Item = (entity in text block, hierarchy depth in the block, span text, span style).
    type Item = (Entity, usize, &'a str, &'a TextStyle);
    fn next(&mut self) -> Option<Self::Item> {
        // Root
        if let Some(root_entity) = self.root_entity.take() {
            if let Ok((text, style, maybe_children)) = self.roots.get(root_entity) {
                if let Some(children) = maybe_children {
                    self.stack.push((children, 0));
                }
                return Some((root_entity, 0, text.read_span(), style));
            } else {
                return None;
            }
        }

        // Span
        loop {
            let Some((children, idx)) = self.stack.last_mut() else {
                return None;
            };

            loop {
                let Some(child) = children.get(*idx) else {
                    break;
                };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                let entity = *child;
                let Ok((span, style, maybe_children)) = self.spans.get(entity) else {
                    continue;
                };

                let depth = self.stack.len();
                if let Some(children) = maybe_children {
                    self.stack.push((children, 0));
                }
                return Some((entity, depth, span.read_span(), style));
            }

            // All children at this stack entry have been iterated.
            self.stack.pop();
        }
    }
}

impl<'a, R: TextSpanAccess, S: TextSpanAccess> Drop for TextSpanIter<'a, R, S> {
    fn drop(&mut self) {
        // Return the internal stack.
        let stack = std::mem::take(&mut self.stack);
        self.scratch.recover(stack);
    }
}

/// System parameter for reading and writing text spans in a [`TextBlock`].
///
/// `R` is the root text component, and `S` is the text span component on children.
#[derive(SystemParam)]
pub struct TextWriter<'w, 's, R: TextSpanAccess, S: TextSpanAccess> {
    scratch: ResMut<'w, TextIterScratch>,
    roots: Query<'w, 's, (&'static mut R, &'static mut TextStyle), Without<S>>,
    spans: Query<'w, 's, (&'static mut S, &'static mut TextStyle), Without<R>>,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's, R: TextSpanAccess, S: TextSpanAccess> TextWriter<'w, 's, R, S> {
    /// Gets a mutable reference to a text span within a text block at a specific index in the flattened span list.
    pub fn get(
        &mut self,
        root_entity: Entity,
        index: usize,
    ) -> Option<(Entity, usize, Mut<String>, Mut<TextStyle>)> {
        // Root
        if index == 0 {
            let (text, style) = self.roots.get_mut(root_entity).ok()?;
            return Some((
                root_entity,
                0,
                text.map_unchanged(|t| t.write_span()),
                style,
            ));
        }

        // Prep stack.
        let mut stack: Vec<(&Children, usize)> = self.scratch.take();
        if let Ok(children) = self.children.get(root_entity) {
            stack.push((children, 0));
        }

        // Span
        let mut count = 1;
        let (depth, entity) = 'l: loop {
            let Some((children, idx)) = stack.last_mut() else {
                self.scratch.recover(stack);
                return None;
            };

            loop {
                let Some(child) = children.get(*idx) else {
                    // All children at this stack entry have been iterated.
                    stack.pop();
                    break;
                };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                if !self.spans.contains(*child) {
                    continue;
                };
                count += 1;

                if count - 1 == index {
                    let depth = stack.len();
                    self.scratch.recover(stack);
                    break 'l (depth, *child);
                }

                if let Ok(children) = self.children.get(*child) {
                    stack.push((children, 0));
                    break;
                }
            }
        };

        // Note: We do this outside the loop due to borrow checker limitations.
        let (text, style) = self.spans.get_mut(entity).unwrap();
        Some((entity, depth, text.map_unchanged(|t| t.write_span()), style))
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    pub fn get_text(&mut self, root_entity: Entity, index: usize) -> Option<Mut<String>> {
        self.get(root_entity, index).map(|(_, _, text, _)| text)
    }

    /// Gets the [`TextStyle`] of a text span within a text block at a specific index in the flattened span list.
    pub fn get_style(&mut self, root_entity: Entity, index: usize) -> Option<Mut<TextStyle>> {
        self.get(root_entity, index).map(|(_, _, _, style)| style)
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn text(&mut self, root_entity: Entity, index: usize) -> Mut<String> {
        self.get_text(root_entity, index).unwrap()
    }

    /// Gets the [`TextStyle`] of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn style(&mut self, root_entity: Entity, index: usize) -> Mut<TextStyle> {
        self.get_style(root_entity, index).unwrap()
    }
}
