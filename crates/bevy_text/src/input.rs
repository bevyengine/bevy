use std::time::Duration;

use crate::buffer_dimensions;
use crate::load_font_to_fontdb;
use crate::CosmicFontSystem;
use crate::Font;
use crate::FontAtlasSets;
use crate::FontSmoothing;
use crate::Justify;
use crate::LineBreak;
use crate::LineHeight;
use crate::PositionedGlyph;
use crate::TextBounds;
use crate::TextError;
use crate::TextFont;
use crate::TextLayoutInfo;
use crate::TextPipeline;
use alloc::collections::VecDeque;
use bevy_asset::Assets;
use bevy_asset::Handle;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::event::EntityEvent;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::query::Changed;
use bevy_ecs::query::Or;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::SystemSet;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::DeferredWorld;
use bevy_ecs::world::Ref;
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_math::IVec2;
use bevy_math::Rect;
use bevy_math::UVec2;
use bevy_math::Vec2;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_time::Time;
use cosmic_text::Action;
use cosmic_text::BorrowedWithFontSystem;
use cosmic_text::Buffer;
use cosmic_text::BufferLine;
use cosmic_text::Edit;
use cosmic_text::Editor;
use cosmic_text::Metrics;
pub use cosmic_text::Motion;
use cosmic_text::Selection;
/// Systems handling text input update and layout
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct TextInputSystems;

/// Basic clipboard implementation that only works within the bevy app.
#[derive(Resource, Default)]
pub struct Clipboard(pub String);

/// Get the text from a cosmic text buffer
fn get_cosmic_text_buffer_contents(buffer: &Buffer) -> String {
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
    /// Controls cursor blinking.
    /// If the value is none or greater than the `blink_interval` in `TextCursorStyle` then the cursor
    /// is not displayed.
    /// The timer is reset when a `TextEdit` is applied.
    pub cursor_blink_timer: Option<f32>,
}

impl Default for TextInputBuffer {
    fn default() -> Self {
        Self {
            editor: Editor::new(Buffer::new_empty(Metrics::new(20.0, 20.0))),
            space_advance: 20.,
            cursor_blink_timer: None,
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

/// Component containing the change history for a text input.
/// Text input entities without this component will ignore undo and redo actions.
#[derive(Component, Debug, Default)]
pub struct UndoHistory {
    /// The commands to undo and undo
    pub changes: cosmic_undo_2::Commands<cosmic_text::Change>,
}

impl UndoHistory {
    /// Clear the history for the text input
    pub fn clear(&mut self) {
        self.changes.clear();
    }
}

/// Details of the target the text input will be rendered to
#[derive(Component, PartialEq, Debug, Default)]
pub struct TextInputTarget {
    /// Size of the target in physical pixels
    pub size: Vec2,
    /// Scale factor of the target
    pub scale_factor: f32,
}

impl TextInputTarget {
    /// Returns true if the target has zero or negative size.
    pub fn is_empty(&self) -> bool {
        (self.scale_factor * self.size).cmple(Vec2::ZERO).all()
    }
}

/// Contains the current text in the text input buffer.
/// Automatically synchronized with the buffer by [`apply_text_edits`] after any edits are applied.
/// On insertion, replaces the current text in the text buffer.
#[derive(Component, PartialEq, Debug, Default, Deref)]
#[component(
    on_insert = on_insert_text_input_value,
)]
pub struct TextInputValue(String);

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

/// The time taken for the cursor to blink
#[derive(Resource)]
pub struct TextCursorBlinkInterval(pub Duration);

impl Default for TextCursorBlinkInterval {
    fn default() -> Self {
        Self(Duration::from_secs_f32(0.5))
    }
}

/// Common text input properties set by the user.
/// On changes, the text input systems will automatically update the buffer, layout and fonts as required.
#[derive(Component, Debug, PartialEq)]
pub struct TextInputAttributes {
    /// The text input's font, also used for any [`Placeholder`] text or password mask.
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
    /// Clear on submit (Triggered when [`apply_text_edits`] receives a [`TextEdit::Submit`] edit for an entity).
    pub clear_on_submit: bool,
}

impl Default for TextInputAttributes {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.,
            line_height: LineHeight::RelativeToFont(1.2),
            font_smoothing: Default::default(),
            justify: Default::default(),
            line_break: Default::default(),
            max_chars: None,
            visible_lines: None,
            clear_on_submit: false,
        }
    }
}

/// If a text input entity has a `TextInputFilter` component, after each [TextEdit] is applied, the [TextInputBuffer]’s text is checked
/// against the filter, and if it fails, the `TextEdit is rolled back.
#[derive(Component)]
pub enum TextInputFilter {
    /// Positive integer input
    /// accepts only digits
    PositiveInteger,
    /// Integer input
    /// accepts only digits and a leading sign
    Integer,
    /// Decimal input
    /// accepts only digits, a decimal point and a leading sign
    Decimal,
    /// Hexadecimal input
    /// accepts only `0-9`, `a-f` and `A-F`
    Hex,
    /// Alphanumeric input
    /// accepts only `0-9`, `a-z` and `A-Z`
    Alphanumeric,
    /// Custom filter
    Custom(Box<dyn Fn(&str) -> bool + Send + Sync>),
}

impl TextInputFilter {
    /// Returns `true` if the given `text` passes the filter
    pub fn is_match(&self, text: &str) -> bool {
        // Always passes if the input is empty unless using a custom filter
        if text.is_empty() && !matches!(self, Self::Custom(_)) {
            return true;
        }

        match self {
            TextInputFilter::PositiveInteger => text.chars().all(|c| c.is_ascii_digit()),
            TextInputFilter::Integer => text
                .strip_prefix('-')
                .unwrap_or(text)
                .chars()
                .all(|c| c.is_ascii_digit()),
            TextInputFilter::Decimal => text
                .strip_prefix('-')
                .unwrap_or(text)
                .chars()
                .try_fold(true, |is_int, c| match c {
                    '.' if is_int => Ok(false),
                    c if c.is_ascii_digit() => Ok(is_int),
                    _ => Err(()),
                })
                .is_ok(),
            TextInputFilter::Hex => text.chars().all(|c| c.is_ascii_hexdigit()),
            TextInputFilter::Alphanumeric => text.chars().all(|c| c.is_ascii_alphanumeric()),
            TextInputFilter::Custom(is_match) => is_match(text),
        }
    }

    /// Create a custom filter
    pub fn custom(filter_fn: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        Self::Custom(Box::new(filter_fn))
    }
}

/// Add this component to hide the text input buffer contents
/// by replacing the characters with `mask_char`.
///
/// It is strongly recommended to only use a `PasswordMask` with fixed-width fonts.
/// With variable width fonts mouse picking and horizontal scrolling
/// may not work correctly.
#[derive(Component)]
pub struct PasswordMask {
    /// If true the password will not be hidden
    pub show_password: bool,
    /// Char that will replace the masked input characters, by default `*`
    pub mask_char: char,
    /// Buffer mirroring the actual text input buffer but only containing `mask_char`s
    editor: Editor<'static>,
}

impl Default for PasswordMask {
    fn default() -> Self {
        Self {
            show_password: false,
            mask_char: '*',
            editor: Editor::new(Buffer::new_empty(Metrics::new(20.0, 20.0))),
        }
    }
}

/// Text input commands queue
#[derive(Component, Default)]
pub struct TextEdits {
    /// Commands to be applied before the text input is updated
    pub queue: VecDeque<TextEdit>,
}

impl TextEdits {
    /// queue an action
    pub fn queue(&mut self, command: TextEdit) {
        self.queue.push_back(command);
    }
}

/// Deferred text input edit and navigation actions applied by the `apply_text_edits` system.
#[derive(Debug, Clone)]
pub enum TextEdit {
    /// Copy the selected text into the clipboard. Does nothing if no text is selected.
    Copy,
    /// Copy the selected text into the clipboard, then delete the selected text. Does nothing if no text is selected.
    Cut,
    /// Insert the contents of the clipboard at the current cursor position. Does nothing if the clipboard is empty.
    Paste,
    /// Move the cursor with some motion.
    Motion {
        /// The motion to perform.
        motion: Motion,
        /// Select the text from the initial cursor position to the end of the motion.
        with_select: bool,
    },
    /// Insert a character at the cursor. If there is a selection, replaces the selection with the character instead.
    Insert(char),
    /// Set the character at the cursor, overwriting the previous character. Inserts if cursor is at the end of a line.
    /// If there is a selection, replaces the selection with the character instead.
    Overwrite(char),
    /// Insert a string at the cursor. If there is a selection, replaces the selection with the string instead.
    InsertString(String),
    /// Start a new line. Ignored for single line text inputs.
    NewLine,
    /// Delete the character behind the cursor.
    /// If there is a selection, deletes the selection instead.
    Backspace,
    /// Delete the character at the cursor.
    /// If there is a selection, deletes the selection instead.
    Delete,
    /// Indent at the cursor.
    Indent,
    /// Unindent at the cursor.
    Unindent,
    /// Moves the cursor to the character at the given position.
    Click(IVec2),
    /// Selects the word at the given position.
    DoubleClick(IVec2),
    /// Selects the line at the given position.
    TripleClick(IVec2),
    /// Select the text up to the given position
    Drag(IVec2),
    /// Scroll vertically by the given number of lines.
    /// Negative values scroll upwards towards the start of the text, positive downwards to the end of the text.
    Scroll {
        /// Number of lines to scroll.
        /// Negative values scroll upwards towards the start of the text, positive downwards to the end of the text.
        lines: i32,
    },
    /// Undo the previous action.
    Undo,
    /// Redo an undone action. Must directly follow an Undo.
    Redo,
    /// Select the entire contents of the text input buffer.
    SelectAll,
    /// Select the line at the cursor.
    SelectLine,
    /// Clear any selection.
    Escape,
    /// Clear the text input buffer.
    Clear,
    /// Set the contents of the text input buffer. The existing contents are discarded.
    SetText(String),
    /// Submit the contents of the text input buffer
    Submit,
}

impl TextEdit {
    /// An action that moves the cursor.
    /// If `with_select` is true, it selects as it moves
    pub fn motion(motion: Motion, with_select: bool) -> Self {
        Self::Motion {
            motion,
            with_select,
        }
    }
}

/// apply a motion action to the editor buffer
pub fn apply_motion<'a>(
    editor: &mut BorrowedWithFontSystem<Editor<'a>>,
    shift_pressed: bool,
    motion: Motion,
) {
    if shift_pressed {
        if editor.selection() == Selection::None {
            let cursor = editor.cursor();
            editor.set_selection(Selection::Normal(cursor));
        }
    } else {
        editor.action(Action::Escape);
    }
    editor.action(Action::Motion(motion));
}

/// Returns true if the cursor is at the end of a line
pub fn is_cursor_at_end_of_line(editor: &mut Editor<'_>) -> bool {
    let cursor = editor.cursor();
    editor.with_buffer(|buffer| {
        buffer
            .lines
            .get(cursor.line)
            .map(|line| cursor.index == line.text().len())
            .unwrap_or(false)
    })
}

/// Apply an action from the undo history to the text input buffer.
fn apply_action<'a>(
    editor: &mut BorrowedWithFontSystem<Editor<'a>>,
    action: cosmic_undo_2::Action<&cosmic_text::Change>,
) {
    match action {
        cosmic_undo_2::Action::Do(change) => {
            editor.apply_change(change);
        }
        cosmic_undo_2::Action::Undo(change) => {
            let mut reversed = change.clone();
            reversed.reverse();
            editor.apply_change(&reversed);
        }
    }
    editor.set_redraw(true);
}

/// Applies the [`TextEdit`]s queued for each [`TextInputBuffer`] and emits [`TextInputEvent`]s in response.
///
/// After all edits are applied, if a text input entity has a [TextInputValue] component and its buffer was changed,
/// then the [TextInputValue]'s text is updated with the new contents of the [TextInputBuffer].
pub fn apply_text_edits(
    mut commands: Commands,
    mut font_system: ResMut<CosmicFontSystem>,
    mut text_input_query: Query<(
        Entity,
        &mut TextInputBuffer,
        &mut TextEdits,
        &TextInputAttributes,
        Option<&TextInputFilter>,
        Option<&mut UndoHistory>,
        Option<&mut TextInputValue>,
    )>,
    mut clipboard: ResMut<Clipboard>,
) {
    for (
        entity,
        mut buffer,
        mut text_input_actions,
        attribs,
        maybe_filter,
        mut maybe_history,
        maybe_value,
    ) in text_input_query.iter_mut()
    {
        for edit in text_input_actions.queue.drain(..) {
            match edit {
                TextEdit::Submit => {
                    commands.trigger_targets(
                        TextInputEvent::Submission {
                            text: buffer.get_text(),
                        },
                        entity,
                    );

                    if attribs.clear_on_submit {
                        let _ = apply_text_edit(
                            buffer.editor.borrow_with(&mut font_system),
                            maybe_history.as_mut().map(AsMut::as_mut),
                            maybe_filter,
                            attribs.max_chars,
                            &mut clipboard,
                            &TextEdit::Clear,
                        );

                        if let Some(history) = maybe_history.as_mut() {
                            history.clear();
                        }
                    }
                }
                edit => {
                    if let Err(error) = apply_text_edit(
                        buffer.editor.borrow_with(&mut font_system),
                        maybe_history.as_mut().map(AsMut::as_mut),
                        maybe_filter,
                        attribs.max_chars,
                        &mut clipboard,
                        &edit,
                    ) {
                        commands.trigger_targets(TextInputEvent::InvalidEdit(error, edit), entity);
                    }
                }
            }
        }

        if let Some(mut value) = maybe_value {
            let contents = buffer.get_text();
            if value.0 != contents {
                value.0 = contents;
                commands.trigger_targets(TextInputEvent::TextChanged, entity);
            }
        }
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
    )>,
    time: Res<Time>,
    cursor_blink_interval: Res<TextCursorBlinkInterval>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut text_pipeline: ResMut<TextPipeline>,
    fonts: Res<Assets<Font>>,
) {
    let font_system = &mut font_system.0;
    let font_id_map = &mut text_pipeline.map_handle_to_font_id;
    for (mut input_buffer, target, edits, attributes) in text_input_query.iter_mut() {
        let TextInputBuffer {
            editor,
            space_advance,
            cursor_blink_timer,
        } = input_buffer.as_mut();

        if let Some(timer) = cursor_blink_timer {
            *timer = if edits.queue.is_empty() {
                (*timer + time.delta_secs()).rem_euclid(cursor_blink_interval.0.as_secs_f32() * 2.)
            } else {
                0.
            };
        }

        let _ = editor.with_buffer_mut(|buffer| {
            if target.is_changed() || attributes.is_changed() {
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

/// Update each [`PasswordMask`] to mirror its underlying [`TextInputBuffer`].
///
/// The recommended practice is to use fixed-width fonts for password inputs.
/// With variable sized fonts the glyph geometry of the password mask editor buffer may not match the
/// underlying editor buffer, possibly resulting in incorrect scrolling and mouse interactions.
pub fn update_password_masks(
    mut text_input_query: Query<(&mut TextInputBuffer, &mut PasswordMask)>,
    mut cosmic_font_system: ResMut<CosmicFontSystem>,
) {
    let font_system = &mut cosmic_font_system.0;
    for (mut buffer, mut mask) in text_input_query.iter_mut() {
        if buffer.editor.redraw() || mask.is_changed() {
            buffer.editor.shape_as_needed(font_system, false);
            let mask_text: String = buffer.get_text().chars().map(|_| mask.mask_char).collect();
            let mask_editor = &mut mask.bypass_change_detection().editor;
            *mask_editor = buffer.editor.clone();
            let mut editor = mask_editor.borrow_with(font_system);
            let selection = editor.selection();
            let cursor = editor.cursor();
            editor.action(Action::Motion(Motion::BufferStart));
            let start = editor.cursor();
            editor.set_selection(Selection::Normal(start));
            editor.action(Action::Motion(Motion::BufferEnd));
            editor.action(Action::Delete);
            editor.insert_string(&mask_text, None);
            editor.set_selection(selection);
            editor.set_cursor(cursor);
            editor.set_redraw(true);
        }
    }
}

/// Based on `LayoutRunIter` from cosmic-text but fixes a bug where the
/// bottom line should be visible but gets cropped when scrolling upwards.
#[derive(Debug)]
pub struct ScrollingLayoutRunIter<'b> {
    /// Cosmic text buffer
    buffer: &'b Buffer,
    /// Index of the current `BufferLine` (The paragraphs of text before line-breaking)
    paragraph_index: usize,
    /// Index of the current `LayoutLine`, a horizontal line of glyphs from the current `BufferLine` (The individual lines of a paragraph after line-breaking)
    broken_line_index: usize,
    /// Total height of the lines iterated so far
    total_height: f32,
    /// The y-coordinate of the top of the current `LayoutLine`.
    line_top: f32,
}

impl<'b> ScrollingLayoutRunIter<'b> {
    /// Returns a new iterator that iterates the visible lines of the `buffer`.
    pub fn new(buffer: &'b Buffer) -> Self {
        Self {
            buffer,
            paragraph_index: buffer.scroll().line,
            broken_line_index: 0,
            total_height: 0.0,
            line_top: 0.0,
        }
    }
}

impl<'b> Iterator for ScrollingLayoutRunIter<'b> {
    type Item = cosmic_text::LayoutRun<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        // Iterate paragraphs
        while let Some(line) = self.buffer.lines.get(self.paragraph_index) {
            let shape = line.shape_opt()?;
            let layout = line.layout_opt()?;

            // Iterate the paragraph's lines after line-breaking
            while let Some(layout_line) = layout.get(self.broken_line_index) {
                self.broken_line_index += 1;

                let line_height = layout_line
                    .line_height_opt
                    .unwrap_or(self.buffer.metrics().line_height);
                self.total_height += line_height;

                let line_top = self.line_top - self.buffer.scroll().vertical;
                let glyph_height = layout_line.max_ascent + layout_line.max_descent;
                let centering_offset = (line_height - glyph_height) / 2.0;
                let line_bottom = line_top + centering_offset + layout_line.max_ascent;
                if let Some(height) = self.buffer.size().1 {
                    if height + line_height < line_bottom {
                        // The line is below the target bound's bottom edge.
                        // No more lines are visible, return `None` to end the iteration.
                        return None;
                    }
                }
                self.line_top += line_height;
                if line_bottom < 0.0 {
                    // The bottom of the line is above the target's bounds top edge and not visible. Skip it.
                    continue;
                }

                return Some(cosmic_text::LayoutRun {
                    line_i: self.paragraph_index,
                    text: line.text(),
                    rtl: shape.rtl,
                    glyphs: &layout_line.glyphs,
                    line_y: line_bottom,
                    line_top,
                    line_height,
                    line_w: layout_line.w,
                });
            }
            self.paragraph_index += 1;
            self.broken_line_index = 0;
        }

        None
    }
}

/// Updates the `TextLayoutInfo` for each text input for rendering.
pub fn update_text_input_layouts(
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut text_query: Query<(
        &mut TextLayoutInfo,
        &mut TextInputBuffer,
        &TextInputAttributes,
        Option<&mut PasswordMask>,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<crate::pipeline::SwashCache>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
) {
    let font_system = &mut font_system.0;
    for (mut layout_info, mut buffer, attributes, mut maybe_password_mask) in text_query.iter_mut()
    {
        // Force a redraw when a password is revealed or hidden
        let force_redraw = maybe_password_mask
            .as_mut()
            .map(|mask| mask.is_changed() && mask.show_password)
            .unwrap_or(false);

        let space_advance = buffer.space_advance;
        let editor = if let Some(password_mask) = maybe_password_mask
            .as_mut()
            .filter(|mask| !mask.show_password)
        {
            // The underlying buffer is hidden, so set redraw to false to avoid unnecessary reupdates.
            buffer.editor.set_redraw(false);
            &mut password_mask.bypass_change_detection().editor
        } else {
            &mut buffer.editor
        };
        editor.shape_as_needed(font_system, false);

        if editor.redraw() || force_redraw {
            layout_info.glyphs.clear();
            layout_info.section_rects.clear();
            layout_info.selection_rects.clear();
            layout_info.cursor_index = None;
            layout_info.cursor = None;

            let selection = editor.selection_bounds();
            let cursor_position = editor.cursor_position();
            let cursor = editor.cursor();

            let result = editor.with_buffer_mut(|buffer| {
                let box_size = buffer_dimensions(buffer);
                let line_height = buffer.metrics().line_height;
                if let Some((x, y)) = cursor_position {
                    let size = Vec2::new(space_advance, line_height);
                    layout_info.cursor = Some((
                        IVec2::new(x, y).as_vec2() + 0.5 * size,
                        size,
                        cursor.affinity.after(),
                    ));
                }
                let result = ScrollingLayoutRunIter::new(buffer).try_for_each(|run| {
                    if let Some(selection) = selection {
                        if let Some((x0, w)) = run.highlight(selection.0, selection.1) {
                            let y0 = run.line_top;
                            let y1 = y0 + run.line_height;
                            let x1 = x0 + w;
                            let r = Rect::new(x0, y0, x1, y1);
                            layout_info.selection_rects.push(r);
                        }
                    }

                    run.glyphs
                        .iter()
                        .map(move |layout_glyph| (layout_glyph, run.line_y, run.line_i))
                        .try_for_each(|(layout_glyph, line_y, line_i)| {
                            let mut temp_glyph;
                            let span_index = layout_glyph.metadata;
                            let font_id = attributes.font.id();
                            let font_smoothing = attributes.font_smoothing;

                            let layout_glyph = if font_smoothing == FontSmoothing::None {
                                // If font smoothing is disabled, round the glyph positions and sizes,
                                // effectively discarding all subpixel layout.
                                temp_glyph = layout_glyph.clone();
                                temp_glyph.x = temp_glyph.x.round();
                                temp_glyph.y = temp_glyph.y.round();
                                temp_glyph.w = temp_glyph.w.round();
                                temp_glyph.x_offset = temp_glyph.x_offset.round();
                                temp_glyph.y_offset = temp_glyph.y_offset.round();
                                temp_glyph.line_height_opt =
                                    temp_glyph.line_height_opt.map(f32::round);

                                &temp_glyph
                            } else {
                                layout_glyph
                            };

                            let font_atlas_set = font_atlas_sets.sets.entry(font_id).or_default();

                            let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                            let atlas_info = font_atlas_set
                                .get_glyph_atlas_info(physical_glyph.cache_key, font_smoothing)
                                .map(Ok)
                                .unwrap_or_else(|| {
                                    font_atlas_set.add_glyph_to_atlas(
                                        &mut texture_atlases,
                                        &mut textures,
                                        font_system,
                                        &mut swash_cache.0,
                                        layout_glyph,
                                        font_smoothing,
                                    )
                                })?;

                            let texture_atlas =
                                texture_atlases.get(atlas_info.texture_atlas).unwrap();
                            let location = atlas_info.location;
                            let glyph_rect = texture_atlas.textures[location.glyph_index];
                            let left = location.offset.x as f32;
                            let top = location.offset.y as f32;
                            let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());

                            // offset by half the size because the origin is center
                            let x = glyph_size.x as f32 / 2.0 + left + physical_glyph.x as f32;
                            let y = line_y.round() + physical_glyph.y as f32 - top
                                + glyph_size.y as f32 / 2.0;

                            let position = Vec2::new(x, y);

                            let pos_glyph = PositionedGlyph {
                                position,
                                size: glyph_size.as_vec2(),
                                atlas_info,
                                span_index,
                                byte_index: layout_glyph.start,
                                byte_length: layout_glyph.end - layout_glyph.start,
                                line_index: line_i,
                            };
                            layout_info.glyphs.push(pos_glyph);
                            if cursor.line == line_i && cursor.index == layout_glyph.start {
                                layout_info.cursor_index = Some(layout_info.glyphs.len() - 1);
                                if let Some((ref mut position, ref mut size, ..)) =
                                    layout_info.cursor
                                {
                                    size.x = layout_glyph.w;
                                    if let Some(cursor_position) = cursor_position {
                                        *position =
                                            IVec2::from(cursor_position).as_vec2() + 0.5 * *size;
                                    }
                                }
                            }

                            Ok(())
                        })
                });

                // Check result.
                result?;
                layout_info.size = box_size;
                Ok(())
            });

            match result {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, try again next frame
                }
                Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(()) => {
                    layout_info.scroll =
                        editor.with_buffer(|buffer| Vec2::new(buffer.scroll().horizontal, 0.));

                    editor.set_redraw(false);
                }
            }
        }
    }
}

/// Error emitted by [`apply_text_edit`] when it fails to apply a [`TextEdit`].
#[derive(Debug, Clone)]
pub enum InvalidTextEditError {
    /// Applying the edit would cause the text to fail its [`TextInputFilter`].
    Filtered,
    /// Applying the edit would cause the text to be longer than its maximum character limit.
    Overflowed,
}

/// Apply a text input action to a text input
fn apply_text_edit(
    mut editor: BorrowedWithFontSystem<'_, Editor<'static>>,
    mut maybe_history: Option<&mut UndoHistory>,
    maybe_filter: Option<&TextInputFilter>,
    max_chars: Option<usize>,
    clipboard_contents: &mut ResMut<Clipboard>,
    edit: &TextEdit,
) -> Result<(), InvalidTextEditError> {
    editor.start_change();

    match edit {
        TextEdit::Copy => {
            if let Some(text) = editor.copy_selection() {
                clipboard_contents.0 = text;
            }
        }
        TextEdit::Cut => {
            if let Some(text) = editor.copy_selection() {
                clipboard_contents.0 = text;
                editor.delete_selection();
            }
        }
        TextEdit::Paste => {
            editor.insert_string(&clipboard_contents.0, None);
        }
        &TextEdit::Motion {
            motion,
            with_select,
        } => {
            apply_motion(&mut editor, with_select, motion);
        }
        TextEdit::Insert(ch) => {
            editor.action(Action::Insert(*ch));
        }
        TextEdit::InsertString(text) => {
            editor.insert_string(text, None);
        }
        &TextEdit::Overwrite(ch) => match editor.selection() {
            Selection::None => {
                if is_cursor_at_end_of_line(&mut editor) {
                    editor.action(Action::Insert(ch));
                } else {
                    editor.action(Action::Delete);
                    editor.action(Action::Insert(ch));
                }
            }
            _ => editor.action(Action::Insert(ch)),
        },
        TextEdit::NewLine => {
            editor.action(Action::Enter);
        }
        TextEdit::Backspace => {
            if !editor.delete_selection() {
                editor.action(Action::Backspace);
            }
        }
        TextEdit::Delete => {
            if !editor.delete_selection() {
                editor.action(Action::Delete);
            }
        }
        TextEdit::Indent => {
            editor.action(Action::Indent);
        }
        TextEdit::Unindent => {
            editor.action(Action::Unindent);
        }
        TextEdit::Click(point) => {
            editor.action(Action::Click {
                x: point.x,
                y: point.y,
            });
        }
        TextEdit::DoubleClick(point) => {
            editor.action(Action::DoubleClick {
                x: point.x,
                y: point.y,
            });
        }
        TextEdit::TripleClick(point) => {
            editor.action(Action::TripleClick {
                x: point.x,
                y: point.y,
            });
        }
        TextEdit::Drag(point) => {
            editor.action(Action::Drag {
                x: point.x,
                y: point.y,
            });
        }
        &TextEdit::Scroll { lines } => {
            editor.action(Action::Scroll { lines });
        }
        TextEdit::Undo => {
            if let Some(history) = maybe_history.as_mut() {
                for action in history.changes.undo() {
                    apply_action(&mut editor, action);
                }
            }
        }
        TextEdit::Redo => {
            if let Some(history) = maybe_history.as_mut() {
                for action in history.changes.redo() {
                    apply_action(&mut editor, action);
                }
            }
        }
        TextEdit::SelectAll => {
            editor.action(Action::Motion(Motion::BufferStart));
            let cursor = editor.cursor();
            editor.set_selection(Selection::Normal(cursor));
            editor.action(Action::Motion(Motion::BufferEnd));
        }
        TextEdit::SelectLine => {
            editor.action(Action::Motion(Motion::Home));
            let cursor = editor.cursor();
            editor.set_selection(Selection::Normal(cursor));
            editor.action(Action::Motion(Motion::End));
        }
        TextEdit::Escape => {
            editor.set_selection(Selection::None);
        }
        TextEdit::Clear => {
            editor.action(Action::Motion(Motion::BufferStart));
            let cursor = editor.cursor();
            editor.set_selection(Selection::Normal(cursor));
            editor.action(Action::Motion(Motion::BufferEnd));
            editor.action(Action::Delete);
        }
        TextEdit::SetText(text) => {
            editor.action(Action::Motion(Motion::Home));
            let cursor = editor.cursor();
            editor.set_selection(Selection::Normal(cursor));
            editor.action(Action::Motion(Motion::End));
            editor.insert_string(&text, None);
        }
        TextEdit::Submit => {}
    }

    let Some(mut change) = editor.finish_change() else {
        return Ok(());
    };

    if change.items.is_empty() {
        return Ok(());
    }

    if maybe_filter.is_some() || max_chars.is_some() {
        let text = editor.with_buffer(get_cosmic_text_buffer_contents);
        if maybe_filter.is_some_and(|filter| !filter.is_match(&text)) {
            change.reverse();
            editor.apply_change(&change);
            return Err(InvalidTextEditError::Filtered);
        }
        if max_chars.is_some_and(|max_chars| max_chars < text.chars().count()) {
            change.reverse();
            editor.apply_change(&change);
            return Err(InvalidTextEditError::Overflowed);
        }
    }

    if let Some(history) = maybe_history.as_mut() {
        history.changes.push(change);
    }

    // Set redraw manually, sometimes the editor doesn't set it automatically.
    editor.set_redraw(true);

    Ok(())
}

/// Automatically propagated events that can be dispatched by a text input entity.
#[derive(EntityEvent, Clone, Debug, Component)]
#[entity_event(traversal = &'static ChildOf, auto_propagate)]
pub enum TextInputEvent {
    /// The text input received an invalid `TextEdit` that failed to be applied
    InvalidEdit(InvalidTextEditError, TextEdit),
    /// Text from the input was submitted
    Submission {
        /// The submitted text
        text: String,
    },
    /// The contents of the text input changed due to a [`TextEdit`].
    /// Only dispatched if a text input entity has a [`TextInputValue`] component.
    TextChanged,
}

/// Placeholder text displayed when the input is empty (including whitespace).
/// Optional component.
#[derive(Default, Component, Clone, Debug, Reflect, Deref, DerefMut)]
#[reflect(Component, Default, Debug)]
#[require(PlaceholderLayout)]
pub struct Placeholder(pub String);

impl Placeholder {
    /// A new [`Placeholder`] text.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self(prompt.into())
    }
}

/// Layout for the [`Placeholder`] text
#[derive(Component)]
pub struct PlaceholderLayout {
    /// A [`Placeholder`] text's cosmic-text buffer (not an Editor as it isn't editable).
    buffer: Buffer,
    /// A [`Placeholder`] text's glyph layout. Displayed when the text input is empty.
    /// Doesn't reuse the editor's [`TextLayoutInfo`] as otherwise the placeholder would need a relayout
    /// everytime it was displayed.
    layout: TextLayoutInfo,
}

impl PlaceholderLayout {
    /// Returns the renderable glyph layout for the associated [`Placeholder`] text
    pub fn layout(&self) -> &TextLayoutInfo {
        &self.layout
    }
}

impl Default for PlaceholderLayout {
    fn default() -> Self {
        Self {
            buffer: Buffer::new_empty(Metrics::new(20.0, 20.0)),
            layout: Default::default(),
        }
    }
}

/// Generates a new [`PlaceholderLayout`] when a [`Placeholder`]'s text or its target's geometry has changed.
pub fn update_placeholder_layouts(
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut swash_cache: ResMut<crate::pipeline::SwashCache>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_query: Query<
        (
            &Placeholder,
            &TextInputAttributes,
            &TextInputTarget,
            &TextFont,
            &mut PlaceholderLayout,
        ),
        Or<(
            Changed<Placeholder>,
            Changed<TextInputAttributes>,
            Changed<TextFont>,
            Changed<TextInputTarget>,
        )>,
    >,
) {
    for (placeholder, style, target, text_font, mut prompt_layout) in text_query.iter_mut() {
        let PlaceholderLayout { buffer, layout } = prompt_layout.as_mut();

        layout.clear();

        if placeholder.0.is_empty() || target.is_empty() {
            continue;
        }

        if !fonts.contains(text_font.font.id()) {
            continue;
        }

        let line_height = text_font.line_height.eval(text_font.font_size);

        let metrics = Metrics::new(text_font.font_size, line_height).scale(target.scale_factor);

        if metrics.font_size <= 0. || metrics.line_height <= 0. {
            continue;
        }

        let bounds: TextBounds = target.size.into();
        let face_info = load_font_to_fontdb(
            text_font.font.clone(),
            font_system.as_mut(),
            &mut text_pipeline.map_handle_to_font_id,
            &fonts,
        );

        buffer.set_size(font_system.as_mut(), bounds.width, bounds.height);

        buffer.set_wrap(&mut font_system, style.line_break.into());

        let attrs = cosmic_text::Attrs::new()
            .metadata(0)
            .family(cosmic_text::Family::Name(&face_info.family_name))
            .stretch(face_info.stretch)
            .style(face_info.style)
            .weight(face_info.weight)
            .metrics(metrics);

        buffer.set_text(
            &mut font_system,
            &placeholder.0,
            &attrs,
            cosmic_text::Shaping::Advanced,
        );

        let align = Some(style.justify.into());
        for buffer_line in buffer.lines.iter_mut() {
            buffer_line.set_align(align);
        }

        buffer.shape_until_scroll(&mut font_system, false);

        let box_size = buffer_dimensions(buffer);
        let result = buffer.layout_runs().try_for_each(|run| {
            run.glyphs
                .iter()
                .map(move |layout_glyph| (layout_glyph, run.line_y, run.line_i))
                .try_for_each(|(layout_glyph, line_y, line_i)| {
                    let mut temp_glyph;
                    let span_index = layout_glyph.metadata;
                    let font_id = text_font.font.id();
                    let font_smoothing = text_font.font_smoothing;

                    let layout_glyph = if font_smoothing == FontSmoothing::None {
                        // If font smoothing is disabled, round the glyph positions and sizes,
                        // effectively discarding all subpixel layout.
                        temp_glyph = layout_glyph.clone();
                        temp_glyph.x = temp_glyph.x.round();
                        temp_glyph.y = temp_glyph.y.round();
                        temp_glyph.w = temp_glyph.w.round();
                        temp_glyph.x_offset = temp_glyph.x_offset.round();
                        temp_glyph.y_offset = temp_glyph.y_offset.round();
                        temp_glyph.line_height_opt = temp_glyph.line_height_opt.map(f32::round);

                        &temp_glyph
                    } else {
                        layout_glyph
                    };

                    let font_atlas_set = font_atlas_sets.sets.entry(font_id).or_default();

                    let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                    let atlas_info = font_atlas_set
                        .get_glyph_atlas_info(physical_glyph.cache_key, font_smoothing)
                        .map(Ok)
                        .unwrap_or_else(|| {
                            font_atlas_set.add_glyph_to_atlas(
                                &mut texture_atlases,
                                &mut textures,
                                &mut font_system,
                                &mut swash_cache.0,
                                layout_glyph,
                                font_smoothing,
                            )
                        })?;

                    let texture_atlas = texture_atlases.get(atlas_info.texture_atlas).unwrap();
                    let location = atlas_info.location;
                    let glyph_rect = texture_atlas.textures[location.glyph_index];
                    let left = location.offset.x as f32;
                    let top = location.offset.y as f32;
                    let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());

                    // offset by half the size because the origin is center
                    let x = glyph_size.x as f32 / 2.0 + left + physical_glyph.x as f32;
                    let y =
                        line_y.round() + physical_glyph.y as f32 - top + glyph_size.y as f32 / 2.0;

                    let position = Vec2::new(x, y);

                    let pos_glyph = PositionedGlyph {
                        position,
                        size: glyph_size.as_vec2(),
                        atlas_info,
                        span_index,
                        byte_index: layout_glyph.start,
                        byte_length: layout_glyph.end - layout_glyph.start,
                        line_index: line_i,
                    };
                    layout.glyphs.push(pos_glyph);
                    Ok(())
                })
        });

        prompt_layout.layout.size = target.scale_factor.recip() * box_size;

        match result {
            Err(TextError::NoSuchFont) => {
                // There was an error processing the text layout, try again next frame
                prompt_layout.layout.clear();
            }
            Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
                panic!("Fatal error when processing text: {e}.");
            }
            Ok(()) => {}
        }
    }
}
