use bevy_color::Color;
use bevy_ecs::{
    component::Mutable,
    prelude::*,
    system::{Query, SystemParam},
};

use crate::{TextColor, TextFont, TextSpan};

/// Helper trait for using the [`TextReader`] and [`TextWriter`] system params.
pub trait TextSpanAccess: Component<Mutability = Mutable> {
    /// Gets the text span's string.
    fn read_span(&self) -> &str;
    /// Gets mutable reference to the text span's string.
    fn write_span(&mut self) -> &mut String;
}

/// Helper trait for the root text component in a text block.
pub trait TextRoot: TextSpanAccess + From<String> {}

/// Helper trait for the text span components in a text block.
pub trait TextSpanComponent: TextSpanAccess + From<String> {}

/// Scratch buffer used to store intermediate state when iterating over text spans.
#[derive(Resource, Default)]
pub struct TextIterScratch {
    stack: Vec<(&'static Children, usize)>,
}

impl TextIterScratch {
    fn take<'a>(&mut self) -> Vec<(&'a Children, usize)> {
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

/// System parameter for reading text spans in a text block.
///
/// `R` is the root text component.
#[derive(SystemParam)]
pub struct TextReader<'w, 's, R: TextRoot> {
    // This is a local to avoid system ambiguities when TextReaders run in parallel.
    scratch: Local<'s, TextIterScratch>,
    roots: Query<
        'w,
        's,
        (
            &'static R,
            &'static TextFont,
            &'static TextColor,
            Option<&'static Children>,
        ),
    >,
    spans: Query<
        'w,
        's,
        (
            &'static TextSpan,
            &'static TextFont,
            &'static TextColor,
            Option<&'static Children>,
        ),
    >,
}

impl<'w, 's, R: TextRoot> TextReader<'w, 's, R> {
    /// Returns an iterator over text spans in a text block, starting with the root entity.
    pub fn iter(&mut self, root_entity: Entity) -> TextSpanIter<'_, R> {
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
    ) -> Option<(Entity, usize, &str, &TextFont, Color)> {
        self.iter(root_entity).nth(index)
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    pub fn get_text(&mut self, root_entity: Entity, index: usize) -> Option<&str> {
        self.get(root_entity, index).map(|(_, _, text, _, _)| text)
    }

    /// Gets the [`TextFont`] of a text span within a text block at a specific index in the flattened span list.
    pub fn get_font(&mut self, root_entity: Entity, index: usize) -> Option<&TextFont> {
        self.get(root_entity, index).map(|(_, _, _, font, _)| font)
    }

    /// Gets the [`TextColor`] of a text span within a text block at a specific index in the flattened span list.
    pub fn get_color(&mut self, root_entity: Entity, index: usize) -> Option<Color> {
        self.get(root_entity, index)
            .map(|(_, _, _, _, color)| color)
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn text(&mut self, root_entity: Entity, index: usize) -> &str {
        self.get_text(root_entity, index).unwrap()
    }

    /// Gets the [`TextFont`] of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn font(&mut self, root_entity: Entity, index: usize) -> &TextFont {
        self.get_font(root_entity, index).unwrap()
    }

    /// Gets the [`TextColor`] of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn color(&mut self, root_entity: Entity, index: usize) -> Color {
        self.get_color(root_entity, index).unwrap()
    }
}

/// Iterator returned by [`TextReader::iter`].
///
/// Iterates all spans in a text block according to hierarchy traversal order.
/// Does *not* flatten interspersed ghost nodes. Only contiguous spans are traversed.
// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIter<'a, R: TextRoot> {
    scratch: &'a mut TextIterScratch,
    root_entity: Option<Entity>,
    /// Stack of (children, next index into children).
    stack: Vec<(&'a Children, usize)>,
    roots: &'a Query<
        'a,
        'a,
        (
            &'static R,
            &'static TextFont,
            &'static TextColor,
            Option<&'static Children>,
        ),
    >,
    spans: &'a Query<
        'a,
        'a,
        (
            &'static TextSpan,
            &'static TextFont,
            &'static TextColor,
            Option<&'static Children>,
        ),
    >,
}

impl<'a, R: TextRoot> Iterator for TextSpanIter<'a, R> {
    /// Item = (entity in text block, hierarchy depth in the block, span text, span style).
    type Item = (Entity, usize, &'a str, &'a TextFont, Color);
    fn next(&mut self) -> Option<Self::Item> {
        // Root
        if let Some(root_entity) = self.root_entity.take() {
            if let Ok((text, text_font, color, maybe_children)) = self.roots.get(root_entity) {
                if let Some(children) = maybe_children {
                    self.stack.push((children, 0));
                }
                return Some((root_entity, 0, text.read_span(), text_font, color.0));
            }
            return None;
        }

        // Span
        loop {
            let (children, idx) = self.stack.last_mut()?;

            loop {
                let Some(child) = children.get(*idx) else {
                    break;
                };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                let entity = *child;
                let Ok((span, text_font, color, maybe_children)) = self.spans.get(entity) else {
                    continue;
                };

                let depth = self.stack.len();
                if let Some(children) = maybe_children {
                    self.stack.push((children, 0));
                }
                return Some((entity, depth, span.read_span(), text_font, color.0));
            }

            // All children at this stack entry have been iterated.
            self.stack.pop();
        }
    }
}

impl<'a, R: TextRoot> Drop for TextSpanIter<'a, R> {
    fn drop(&mut self) {
        // Return the internal stack.
        let stack = core::mem::take(&mut self.stack);
        self.scratch.recover(stack);
    }
}

/// System parameter for reading and writing text spans in a text block.
///
/// `R` is the root text component, and `S` is the text span component on children.
#[derive(SystemParam)]
pub struct TextWriter<'w, 's, R: TextRoot> {
    // This is a resource because two TextWriters can't run in parallel.
    scratch: ResMut<'w, TextIterScratch>,
    roots: Query<
        'w,
        's,
        (
            &'static mut R,
            &'static mut TextFont,
            &'static mut TextColor,
        ),
        Without<TextSpan>,
    >,
    spans: Query<
        'w,
        's,
        (
            &'static mut TextSpan,
            &'static mut TextFont,
            &'static mut TextColor,
        ),
        Without<R>,
    >,
    children: Query<'w, 's, &'static Children>,
}

impl<'w, 's, R: TextRoot> TextWriter<'w, 's, R> {
    /// Gets a mutable reference to a text span within a text block at a specific index in the flattened span list.
    pub fn get(
        &mut self,
        root_entity: Entity,
        index: usize,
    ) -> Option<(
        Entity,
        usize,
        Mut<'_, String>,
        Mut<'_, TextFont>,
        Mut<'_, TextColor>,
    )> {
        // Root
        if index == 0 {
            let (text, font, color) = self.roots.get_mut(root_entity).ok()?;
            return Some((
                root_entity,
                0,
                text.map_unchanged(|t| t.write_span()),
                font,
                color,
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
        let (text, font, color) = self.spans.get_mut(entity).unwrap();
        Some((
            entity,
            depth,
            text.map_unchanged(|t| t.write_span()),
            font,
            color,
        ))
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    pub fn get_text(&mut self, root_entity: Entity, index: usize) -> Option<Mut<'_, String>> {
        self.get(root_entity, index).map(|(_, _, text, ..)| text)
    }

    /// Gets the [`TextFont`] of a text span within a text block at a specific index in the flattened span list.
    pub fn get_font(&mut self, root_entity: Entity, index: usize) -> Option<Mut<'_, TextFont>> {
        self.get(root_entity, index).map(|(_, _, _, font, _)| font)
    }

    /// Gets the [`TextColor`] of a text span within a text block at a specific index in the flattened span list.
    pub fn get_color(&mut self, root_entity: Entity, index: usize) -> Option<Mut<'_, TextColor>> {
        self.get(root_entity, index)
            .map(|(_, _, _, _, color)| color)
    }

    /// Gets the text value of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn text(&mut self, root_entity: Entity, index: usize) -> Mut<'_, String> {
        self.get_text(root_entity, index).unwrap()
    }

    /// Gets the [`TextFont`] of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn font(&mut self, root_entity: Entity, index: usize) -> Mut<'_, TextFont> {
        self.get_font(root_entity, index).unwrap()
    }

    /// Gets the [`TextColor`] of a text span within a text block at a specific index in the flattened span list.
    ///
    /// Panics if there is no span at the requested index.
    pub fn color(&mut self, root_entity: Entity, index: usize) -> Mut<'_, TextColor> {
        self.get_color(root_entity, index).unwrap()
    }

    /// Invokes a callback on each span in a text block, starting with the root entity.
    pub fn for_each(
        &mut self,
        root_entity: Entity,
        mut callback: impl FnMut(Entity, usize, Mut<String>, Mut<TextFont>, Mut<TextColor>),
    ) {
        self.for_each_until(root_entity, |a, b, c, d, e| {
            (callback)(a, b, c, d, e);
            true
        });
    }

    /// Invokes a callback on each span's string value in a text block, starting with the root entity.
    pub fn for_each_text(&mut self, root_entity: Entity, mut callback: impl FnMut(Mut<String>)) {
        self.for_each(root_entity, |_, _, text, _, _| {
            (callback)(text);
        });
    }

    /// Invokes a callback on each span's [`TextFont`] in a text block, starting with the root entity.
    pub fn for_each_font(&mut self, root_entity: Entity, mut callback: impl FnMut(Mut<TextFont>)) {
        self.for_each(root_entity, |_, _, _, font, _| {
            (callback)(font);
        });
    }

    /// Invokes a callback on each span's [`TextColor`] in a text block, starting with the root entity.
    pub fn for_each_color(
        &mut self,
        root_entity: Entity,
        mut callback: impl FnMut(Mut<TextColor>),
    ) {
        self.for_each(root_entity, |_, _, _, _, color| {
            (callback)(color);
        });
    }

    /// Invokes a callback on each span in a text block, starting with the root entity.
    ///
    /// Traversal will stop when the callback returns `false`.
    // TODO: find a way to consolidate get and for_each_until, or provide a real iterator. Lifetime issues are challenging here.
    pub fn for_each_until(
        &mut self,
        root_entity: Entity,
        mut callback: impl FnMut(Entity, usize, Mut<String>, Mut<TextFont>, Mut<TextColor>) -> bool,
    ) {
        // Root
        let Ok((text, font, color)) = self.roots.get_mut(root_entity) else {
            return;
        };
        if !(callback)(
            root_entity,
            0,
            text.map_unchanged(|t| t.write_span()),
            font,
            color,
        ) {
            return;
        }

        // Prep stack.
        let mut stack: Vec<(&Children, usize)> = self.scratch.take();
        if let Ok(children) = self.children.get(root_entity) {
            stack.push((children, 0));
        }

        // Span
        loop {
            let depth = stack.len();
            let Some((children, idx)) = stack.last_mut() else {
                self.scratch.recover(stack);
                return;
            };

            loop {
                let Some(child) = children.get(*idx) else {
                    // All children at this stack entry have been iterated.
                    stack.pop();
                    break;
                };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                let entity = *child;
                let Ok((text, font, color)) = self.spans.get_mut(entity) else {
                    continue;
                };

                if !(callback)(
                    entity,
                    depth,
                    text.map_unchanged(|t| t.write_span()),
                    font,
                    color,
                ) {
                    self.scratch.recover(stack);
                    return;
                }

                if let Ok(children) = self.children.get(entity) {
                    stack.push((children, 0));
                    break;
                }
            }
        }
    }
}
