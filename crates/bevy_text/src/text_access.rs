use bevy_ecs::{prelude::*, system::{Query, SystemParam}};
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

/// System parameter for reading text spans in a [`TextBlock`].
///
/// `R` is the root text component, and `S` is the text span component on children.
#[derive(SystemParam)]
pub struct TextReader<'w, 's, R: TextSpanAccess, S: TextSpanAccess> {
    scratch: ResMut<'w, TextIterScratch>,
    roots: Query<'w, 's, (&'static R, &'static TextStyle, Option<&'static Children>)>,
    spans: Query<
        'w,
        's,
        (
            Entity,
            &'static S,
            &'static TextStyle,
            Option<&'static Children>,
        ),
    >,
}

impl<'w, 's, R: TextSpanAccess, S: TextSpanAccess> TextReader<'w, 's, R, S> {
    /// Returns an iterator over text spans in a text block, starting with the root entity.
    pub fn iter<'a>(&'a mut self, root_entity: Entity) -> TextSpanIter<'a, R, S> {
        let stack = core::mem::take(&mut self.scratch.stack)
            .into_iter()
            .map(|_| -> (&Children, usize) { unreachable!() })
            .collect();

        TextSpanIter {
            scratch: &mut self.scratch,
            root_entity: Some(root_entity),
            stack,
            roots: &self.roots,
            spans: &self.spans,
        }
    }

    /// Gets a text span within a text block at a specific index in the flattened span list.
    pub fn get_by_index<'a>(&'a mut self, root_entity: Entity, index: usize) -> Option<(&'a str, &'a TextStyle)> {
        self.iter(root_entity).nth(index).map(|(_, _, text, style)| (text, style))
    }
}

/// System parameter for reading and writing text spans in a [`TextBlock`].
///
/// `R` is the root text component, and `S` is the text span component on children.
#[derive(SystemParam)]
pub struct TextWriter<'w, 's, R: TextSpanAccess, S: TextSpanAccess> {
    scratch: ResMut<'w, TextIterScratch>,
    roots: Query<'w, 's, (&'static mut R, &'static mut TextStyle), Without<S>>,
    spans: Query<
        'w,
        's,
        (
            Entity,
            &'static mut S,
            &'static mut TextStyle,
        ),
        Without<R>
    >,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's, R: TextSpanAccess, S: TextSpanAccess> TextWriter<'w, 's, R, S> {
    /// Returns a mutatable iterator over text spans in a text block, starting with the root entity.
    pub fn iter<'a>(&'a mut self, root_entity: Entity) -> TextSpanIterMut<'a, R, S> {
        let stack = core::mem::take(&mut self.scratch.stack)
            .into_iter()
            .map(|_| -> (&Children, usize) { unreachable!() })
            .collect();

        TextSpanIterMut {
            scratch: &mut self.scratch,
            root_entity: Some(root_entity),
            stack,
            roots: &mut self.roots,
            spans: &mut self.spans,
            children: &self.children,
        }
    }

    /// Gets a mutable reference to a text span within a text block at a specific index in the flattened span list.
    pub fn get_by_index<'a>(&'a mut self, root_entity: Entity, index: usize) -> Option<(Mut<'a, String>, Mut<'a, TextStyle>)> {
        self.iter(root_entity).nth(index).map(|(_, _, text, style)| (text, style))
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
    spans: &'a Query<
        'a,
        'a,
        (
            Entity,
            &'static S,
            &'static TextStyle,
            Option<&'static Children>,
        ),
    >,
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

                let Ok((entity, span, style, maybe_children)) = self.spans.get(*child) else {
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
        let mut stack = std::mem::take(&mut self.stack);
        stack.clear();
        self.scratch.stack = stack
            .into_iter()
            .map(|_| -> (&'static Children, usize) { unreachable!() })
            .collect();
    }
}

/// Iterator returned by [`TextWriter::iter_mut`].
///
/// Iterates all spans in a text block according to hierarchy traversal order.
/// Does *not* flatten interspersed ghost nodes. Only contiguous spans are traversed.
// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIterMut<'a, R: TextSpanAccess, S: TextSpanAccess> {
    scratch: &'a mut TextIterScratch,
    root_entity: Option<Entity>,
    /// Stack of (children, next index into children).
    stack: Vec<(&'a Children, usize)>,
    roots: &'a mut Query<'a, 'a, (&'a mut R, &'a mut TextStyle), Without<S>>,
    spans: &'a mut Query<'a,'a, (Entity, &'a mut S, &'a mut TextStyle), Without<R>>,
    children: &'a Query<'a, 'a, &'a Children>,
}

impl<'a, R: TextSpanAccess, S: TextSpanAccess> Iterator for TextSpanIterMut<'a, R, S> {
    /// Item = (entity in text block, hierarchy depth in the block, span text, span style).
    type Item = (Entity, usize, Mut<'a, String>, Mut<'a, TextStyle>);
    fn next(&mut self) -> Option<Self::Item> {
        // Root
        if let Some(root_entity) = self.root_entity.take() {
            if let Ok((text, style)) = self.roots.get_mut(root_entity) {
                if let Ok(children) = self.children.get(root_entity) {
                    self.stack.push((children, 0));
                }
                return Some((root_entity, 0, text.map_unchanged(|t| t.write_span()), style));
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

                let Ok((entity, span, style)) = self.spans.get_mut(*child) else {
                    continue;
                };

                let depth = self.stack.len();
                if let Ok(children) = self.children.get(entity) {
                    self.stack.push((children, 0));
                }
                return Some((entity, depth, span.map_unchanged(|t| t.write_span()), style));
            }

            // All children at this stack entry have been iterated.
            self.stack.pop();
        }
    }
}

impl<'a, R: TextSpanAccess, S: TextSpanAccess> Drop for TextSpanIterMut<'a, R, S> {
    fn drop(&mut self) {
        // Return the internal stack.
        let mut stack = std::mem::take(&mut self.stack);
        stack.clear();
        self.scratch.stack = stack
            .into_iter()
            .map(|_| -> (&'static Children, usize) { unreachable!() })
            .collect();
    }
}
