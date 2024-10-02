/// Helper trait for using the [`TextSpans`] system param.
pub trait TextSpanReader: Component {
    /// Gets the text span's string.
    fn read_span(&self) -> &str;
}

#[derive(Resource, Default)]
pub(crate) struct TextSpansScratch {
    stack: Vec<(&'static Children, usize)>,
}

#[derive(SystemParam)]
pub struct TextSpans<'w, 's, T: TextSpanReader> {
    scratch: ResMut<'w, TextSpansScratch>,
    spans: Query<
        'w,
        's,
        (
            Entity,
            &'static T,
            &'static TextStyle,
            Option<&'static Children>,
        ),
    >,
}

impl<'w, 's, T: TextSpanReader> TextSpans<'w, 's, T> {
    /// Returns an iterator over text spans in a text block, starting with the root entity.
    pub fn iter_from_base(
        &'s mut self,
        root_entity: Entity,
        root_text: &str,
        root_style: &TextStyle,
        root_children: Option<&Children>,
    ) -> TextSpanIter<'w, 's, T> {
        let mut stack = core::mem::take(&mut self.scratch.stack)
            .into_iter()
            .map(|_| -> (&Children, usize) { unreachable!() })
            .collect();

        TextSpanIter {
            scratch: &mut self.scratch,
            root: Some((root_entity, root_text, root_style, root_children)),
            stack,
            spans: &self.spans,
        }
    }
}

/// Iterator returned by [`TextSpans::iter_from_base`].
///
/// Iterates all spans in a text block according to hierarchy traversal order.
/// Does *not* flatten interspersed ghost nodes. Only contiguous spans are traversed.
// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIter<'w, 's, T: TextSpanReader> {
    scratch: &'s mut TextSpansScratch,
    root: Option<(Entity, &'s str, &'s TextStyle, Option<&'s Children>)>,
    /// Stack of (children, next index into children).
    stack: Vec<(&'s Children, usize)>,
    spans: &'s Query<
        'w,
        's,
        (
            Entity,
            &'static T,
            &'static TextStyle,
            Option<&'static Children>,
        ),
    >,
}

impl<'w, 's, T: TextSpanReader> Iterator for TextSpanIter<'w, 's, T> {
    /// Item = (entity in text block, hierarchy depth in the block, span text, span style).
    type Item = (Entity, usize, &str, &TextStyle);
    fn next(&mut self) -> Option<Self::Item> {
        // Root
        if let Some((entity, text, style, maybe_children)) = self.root.take() {
            if let Some(children) = maybe_children {
                self.stack.push((children, 0));
            }
            return Some((entity, 0, text, style));
        }

        // Span
        loop {
            let Some((children, idx)) = self.stack.last_mut() else {
                return None;
            };

            loop {
                let Some(child) = children.get(idx) else {
                    break;
                };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                let Some((entity, span, style, maybe_children)) = self.spans.get(*child) else {
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

impl<'w, 's> Drop for TextSpanIter {
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
