
/// Helper trait for setting up [`TextSpanPlugin`].
pub trait TextSpanReader: Component {
    /// Gets the text span's string.
    fn read_span(&self) -> &str;
}

#[derive(Resource, Default)]
pub(crate) struct TextSpansScratch {
    stack: Vec<(&'static Children, usize)>
}

#[derive(SystemParam)]
pub struct TextSpans<'w, 's, T: TextSpanReader> {
    scratch: ResMut<'w, TextSpansScratch>,
    spans: Query<'w, 's, (Entity, &'static T, &'static TextStyle, Option<&'static Children>)>
}

impl<'w, 's, T: TextSpanReader> TextSpans<'w, 's, T> {
    pub fn iter_from_base(
        &mut self,
        root_entity: Entity,
        root_text: &str,
        root_style: &TextStyle,
        root_children: Option<&Children>
    ) -> TextSpanIter<'w, 's, T> {
        let mut stack = core::mem::take(&mut self.scratch.stack)
            .into_iter()
            .map(|_| -> (&Children, usize) { unreachable!() })
            .collect();

        TextSpanIter{
            scratch: &mut self.scratch,
            root: Some((root_entity, root_text, root_style, root_children)),
            stack,
            spans: &self.spans,
        }
    }
}

// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIter<'w, 's, T: TextSpanReader> {
    scratch: &'s mut TextSpansScratch,
    root: Option<(Entity, &'s str, &'s TextStyle, Option<&'s Children>)>,
    /// Stack of (children, next index into children).
    stack: Vec<(&'s Children, usize)>,
    spans: &'s Query<'w, 's, (Entity, &'static T, &'static TextStyle, Option<&'static Children>)>,
}

impl<'w, 's, T: TextSpanReader> Iterator for TextSpanIter<'w, 's, T> {
    /// Item = (entity in text block, depth in the block, span text, span style).
    type Item = (Entity, usize, &str, &TextStyle>);
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
            let Some((children, idx)) = self.stack.last_mut() else { return None };

            loop {
                let Some(child) = children.get(idx) else { break };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                let Some((entity, span, style, maybe_children)) = self.spans.get(*child) else { continue };

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
        self.stack.clear();
        self.scratch.stack = self.stack
            .into_iter()
            .map(
                |_| -> (&'static Children, usize) {
                    unreachable!()
                },
            )
            .collect();
    }
}

/// Detects changes to text blocks and sets `ComputedTextBlock::should_rerender`.
pub fn detect_text_needs_rerender<Root: Component, Span: Component>(
    changed_roots: Query<Entity, (
        Or<(Changed<Root>, Changed<TextStyle>, Changed<TextBlock>, Changed<Children>)>,
        With<Root>, With<TextStyle>, With<TextBlock>,
    )>,
    changed_spans: Query<
        &Parent,
        Or<(Changed<Span>, Changed<TextStyle>, Changed<Children>)>,
        With<Span>, With<TextStyle>,
    >,
    mut computed: Query<(Option<&Parent>, Option<&mut ComputedTextBlock>)>,
)
{
    // Root entity:
    // - Root component changed.
    // - TextStyle on root changed.
    // - TextBlock changed.
    // - Root children changed (can include additions and removals).
    for root in changed_roots.iter() {
        // TODO: ComputedTextBlock *should* be here, log a warning?
        let Ok((_, Some(mut computed))) = computed.get_mut(root) else { continue };
        computed.needs_rerender = true;
    }

    // Span entity:
    // - Span component changed.
    // - Span TextStyle changed.
    // - Span children changed (can include additions and removals).
    for span_parent in changed_spans.iter() {
        let mut parent: Entity = **span_parent;

        // Search for the nearest ancestor with ComputedTextBlock.
        // Note: We assume the perf cost from duplicate visits in the case that multiple spans in a block are visited
        // is outweighed by the expense of tracking visited spans.
        loop {
            // TODO: If this lookup fails then there is a hierarchy error. Log a warning?
            let Ok((maybe_parent, maybe_computed)) = computed.get_mut(parent) else { break };
            if let Some(computed) = maybe_computed {
                computed.needs_rerender = true;
                break;
            }
            // TODO: If there is no parent then a span is floating without an owning TextBlock. Log a warning?
            let Some(next_parent) = maybe_parent else { break };
            parent = **next_parent;
        }
    }
}
