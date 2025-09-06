use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_derive::Deref;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    event::EventReader,
    lifecycle::HookContext,
    system::{Query, Res, ResMut},
    world::{DeferredWorld, Ref},
};
use bevy_time::Time;
use cosmic_text::{Buffer, BufferLine, Edit, Editor, Metrics};

use crate::{
    load_font_to_fontdb, CosmicFontSystem, CursorBlink, Font, FontSmoothing, Justify, LineBreak,
    LineHeight, TextCursorBlinkInterval, TextEdit, TextEdits, TextError, TextInputTarget,
    TextLayoutInfo, TextPipeline,
};

/// Common text input properties set by the user.
/// On changes, the text input systems will automatically update the buffer, layout and fonts as required.
#[derive(Component, Debug, PartialEq)]
pub struct TextInputAttributes {
    /// The text input's font, which also applies to any [`crate::Placeholder`] text or password mask.
    /// A text input's glyphs must all be from the same font.
    pub font: Handle<Font>,
    /// The size of the font.
    /// A text input's glyphs must all be the same size.
    pub font_size: f32,
    /// The height of each line.
    /// A text input's lines must all be the same height.
    pub line_height: LineHeight,
    /// Determines how lines will be broken
    pub line_break: LineBreak,
    /// The horizontal alignment for all the text in the text input buffer.
    pub justify: Justify,
    /// Controls text antialiasing
    pub font_smoothing: FontSmoothing,
    /// Maximum number of glyphs the text input buffer can contain.
    /// Any edits that extend the length above `max_chars` are ignored.
    /// If set on a buffer longer than `max_chars` the buffer will be truncated.
    pub max_chars: Option<usize>,
    /// The maximum number of lines the buffer will display without scrolling.
    /// * Clamped between zero and target height divided by line height.
    /// * If None or equal or less than 0, will fill the target space.
    /// * Only restricts the maximum number of visible lines, places no constraint on the text buffer's length.
    /// * Supports fractional values, `visible_lines: Some(2.5)` will display two and a half lines of text.
    pub visible_lines: Option<f32>,
}

/// Default font size
pub const DEFAULT_FONT_SIZE: f32 = 20.;
/// Default line height factor (relative to font size)
///
/// `1.2` corresponds to `normal` in `<https://developer.mozilla.org/en-US/docs/Web/CSS/line-height>`
pub const DEFAULT_LINE_HEIGHT_FACTOR: f32 = 1.2;
/// Default line height
pub const DEFAULT_LINE_HEIGHT: f32 = DEFAULT_FONT_SIZE * DEFAULT_LINE_HEIGHT_FACTOR;
/// Default space advance
pub const DEFAULT_SPACE_ADVANCE: f32 = 20.;

impl Default for TextInputAttributes {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: DEFAULT_FONT_SIZE,
            line_height: LineHeight::RelativeToFont(DEFAULT_LINE_HEIGHT_FACTOR),
            font_smoothing: Default::default(),
            justify: Default::default(),
            line_break: Default::default(),
            max_chars: None,
            visible_lines: None,
        }
    }
}

/// Contains the current text in the text input buffer.
/// Automatically synchronized with the buffer by [`crate::apply_text_edits`] after any edits are applied.
/// On insertion, replaces the current text in the text buffer.
#[derive(Component, PartialEq, Debug, Default, Deref)]
#[component(
    on_insert = on_insert_text_input_value,
)]
pub struct TextInputValue(pub String);

impl TextInputValue {
    /// New text, when inserted replaces the current text in the text buffer
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the current text
    pub fn get(&self) -> &str {
        &self.0
    }
}

/// Set the text input with the text from the `TextInputValue` when inserted.
fn on_insert_text_input_value(mut world: DeferredWorld, context: HookContext) {
    if let Some(value) = world.get::<TextInputValue>(context.entity) {
        let value = value.0.clone();
        if let Some(mut actions) = world.entity_mut(context.entity).get_mut::<TextEdits>() {
            actions.queue(TextEdit::SetText(value));
        }
    }
}

/// Get the text from a cosmic text buffer
pub fn get_cosmic_text_buffer_contents(buffer: &Buffer) -> String {
    buffer
        .lines
        .iter()
        .map(BufferLine::text)
        .fold(String::new(), |mut out, line| {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
            out
        })
}

/// The text input buffer.
/// Primary component that contains the text layout.
///
/// The `needs_redraw` method can be used to check if the buffer's contents have changed and need redrawing.
/// Component change detection is not reliable as the editor buffer needs to be borrowed mutably during updates.
#[derive(Component, Debug)]
#[require(TextInputAttributes, TextInputTarget, TextEdits, TextLayoutInfo)]
pub struct TextInputBuffer {
    /// The cosmic text editor buffer.
    pub editor: Editor<'static>,
    /// Space advance width for the current font, used to determine the width of the cursor when it is at the end of a line
    /// or when the buffer is empty.
    pub space_advance: f32,
}

impl Default for TextInputBuffer {
    fn default() -> Self {
        Self {
            editor: Editor::new(Buffer::new_empty(Metrics::new(
                DEFAULT_FONT_SIZE,
                DEFAULT_LINE_HEIGHT,
            ))),
            space_advance: DEFAULT_SPACE_ADVANCE,
        }
    }
}

impl TextInputBuffer {
    /// Use the cosmic text buffer mutably
    pub fn with_buffer_mut<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Buffer) -> T,
    {
        self.editor.with_buffer_mut(f)
    }

    /// Use the cosmic text buffer
    pub fn with_buffer<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&Buffer) -> T,
    {
        self.editor.with_buffer(f)
    }

    /// True if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.with_buffer(|buffer| {
            buffer.lines.is_empty()
                || (buffer.lines.len() == 1 && buffer.lines[0].text().is_empty())
        })
    }

    /// Get the text contained in the text buffer
    pub fn get_text(&self) -> String {
        self.editor.with_buffer(get_cosmic_text_buffer_contents)
    }

    /// Returns true if the buffer's contents have changed and need to be redrawn.
    pub fn needs_redraw(&self) -> bool {
        self.editor.redraw()
    }
}

/// Updates the text input buffer in response to changes
/// that require regeneration of the the buffer's
/// metrics and attributes.
pub fn update_text_input_buffers(
    mut text_input_query: Query<(
        &mut TextInputBuffer,
        Ref<TextInputTarget>,
        &TextEdits,
        Ref<TextInputAttributes>,
        Option<&mut CursorBlink>,
    )>,
    time: Res<Time>,
    cursor_blink_interval: Res<TextCursorBlinkInterval>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut text_pipeline: ResMut<TextPipeline>,
    fonts: Res<Assets<Font>>,
    mut font_events: EventReader<AssetEvent<Font>>,
) {
    let font_system = &mut font_system.0;
    let font_id_map = &mut text_pipeline.map_handle_to_font_id;
    for (mut input_buffer, target, edits, attributes, maybe_cursor_blink) in
        text_input_query.iter_mut()
    {
        let TextInputBuffer {
            editor,
            space_advance,
        } = input_buffer.as_mut();

        if let Some(mut cursor_blink) = maybe_cursor_blink {
            cursor_blink.cursor_blink_timer = if edits.queue.is_empty() {
                (cursor_blink.cursor_blink_timer + time.delta_secs())
                    .rem_euclid(cursor_blink_interval.0.as_secs_f32() * 2.)
            } else {
                0.
            };
        }

        let _ = editor.with_buffer_mut(|buffer| {
            if target.is_changed()
                || attributes.is_changed()
                || font_events.read().any(|event| match event {
                    AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                        *id == attributes.font.id()
                    }
                    _ => false,
                })
            {
                let line_height = attributes.line_height.eval(attributes.font_size);
                let metrics =
                    Metrics::new(attributes.font_size, line_height).scale(target.scale_factor);
                buffer.set_metrics(font_system, metrics);

                buffer.set_wrap(font_system, attributes.line_break.into());

                if !fonts.contains(attributes.font.id()) {
                    return Err(TextError::NoSuchFont);
                }

                let face_info =
                    load_font_to_fontdb(attributes.font.clone(), font_system, font_id_map, &fonts);

                let attrs = cosmic_text::Attrs::new()
                    .metadata(0)
                    .family(cosmic_text::Family::Name(&face_info.family_name))
                    .stretch(face_info.stretch)
                    .style(face_info.style)
                    .weight(face_info.weight);

                let mut text = buffer.lines.iter().map(BufferLine::text).fold(
                    String::new(),
                    |mut out, line| {
                        if !out.is_empty() {
                            out.push('\n');
                        }
                        out.push_str(line);
                        out
                    },
                );

                if let Some(max_chars) = attributes.max_chars {
                    text.truncate(max_chars);
                }

                buffer.set_text(font_system, &text, &attrs, cosmic_text::Shaping::Advanced);
                let align = Some(attributes.justify.into());
                for buffer_line in buffer.lines.iter_mut() {
                    buffer_line.set_align(align);
                }

                *space_advance = font_id_map
                    .get(&attributes.font.id())
                    .and_then(|(id, ..)| font_system.get_font(*id))
                    .and_then(|font| {
                        let face = font.rustybuzz();
                        face.glyph_index(' ')
                            .and_then(|gid| face.glyph_hor_advance(gid))
                            .map(|advance| advance as f32 / face.units_per_em() as f32)
                    })
                    .unwrap_or(0.0)
                    * buffer.metrics().font_size;

                let height =
                    if let Some(lines) = attributes.visible_lines.filter(|lines| 0. < *lines) {
                        (metrics.line_height * lines).max(target.size.y)
                    } else {
                        target.size.y
                    };

                buffer.set_size(
                    font_system,
                    Some(target.size.x - *space_advance),
                    Some(height),
                );

                buffer.set_redraw(true);
            }

            Ok(())
        });
    }
}
