use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_hierarchy::Children;

use crate::TextStyle;

/// Helper trait for using the [`TextBlocks`] system param.
pub trait TextSpanReader: Component {
    /// Gets the text span's string.
    fn read_span(&self) -> &str;
}

#[derive(Resource, Default)]
pub(crate) struct TextSpansScratch {
    stack: Vec<(&'static Children, usize)>,
}

/// System parameter for iterating over text spans in a [`TextBlock`].
///
/// `R` is the root text component, and `S` is the text span component on children.
#[derive(SystemParam)]
pub struct TextBlocks<'w, 's, R: TextSpanReader, S: TextSpanReader> {
    scratch: ResMut<'w, TextSpansScratch>,
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

impl<'w, 's, R: TextSpanReader, S: TextSpanReader> TextBlocks<'w, 's, R, S> {
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
}

/// Iterator returned by [`TextBlocks::iter`].
///
/// Iterates all spans in a text block according to hierarchy traversal order.
/// Does *not* flatten interspersed ghost nodes. Only contiguous spans are traversed.
// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIter<'a, R: TextSpanReader, S: TextSpanReader> {
    scratch: &'a mut TextSpansScratch,
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

impl<'a, R: TextSpanReader, S: TextSpanReader> Iterator for TextSpanIter<'a, R, S> {
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

impl<'a, R: TextSpanReader, S: TextSpanReader> Drop for TextSpanIter<'a, R, S> {
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
