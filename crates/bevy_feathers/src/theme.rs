//! A framework for theming.
use bevy_app::{Propagate, PropagateOver};
use bevy_color::{palettes, Alpha, Color, Oklcha};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    lifecycle::Insert,
    observer::On,
    query::Changed,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::TextColor;
use bevy_ui::{BackgroundColor, BorderColor};
use smol_str::SmolStr;

use crate::tokens;

/// A design token for the theme. This serves as the lookup key for the theme properties.
#[derive(Clone, PartialEq, Eq, Hash, Reflect, Default)]
pub struct ThemeToken(SmolStr);

impl ThemeToken {
    /// Construct a new [`ThemeToken`] from a [`SmolStr`].
    pub const fn new(text: SmolStr) -> Self {
        Self(text)
    }

    /// Construct a new [`ThemeToken`] from a static string.
    pub const fn new_static(text: &'static str) -> Self {
        Self(SmolStr::new_static(text))
    }
}

impl core::fmt::Display for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ThemeToken({:?})", self.0)
    }
}

/// A collection of properties that make up a theme.
#[derive(Default, Clone, Reflect, Debug)]
#[reflect(Default, Debug)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<ThemeToken, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color(&self, token: &ThemeToken) -> Color {
        let color = self.0.color.get(token);
        match color {
            Some(c) => *c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Associate a design token with a given color.
    pub fn set_color(&mut self, token: &str, color: Color) {
        self.0
            .color
            .insert(ThemeToken::new(SmolStr::new(token)), color);
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub ThemeToken);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub ThemeToken);

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct InheritableThemeTextColor(pub ThemeToken);

/// Component which causes the color of a text span to be set based on a theme color. Unlike
/// [`InheritableThemeTextColor`], this can work when set directly on the text span entity, and is
/// not inherited.
// TODO: This is necessary because an entity with Propagate doesn't update itself, only its
// descendants.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct ThemeTextColor(pub ThemeToken);

/// A marker component that is used to indicate that the text entity wants to opt-in to using
/// inherited text styles.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
pub struct ThemedText;

pub(crate) fn update_theme(
    mut q_background: Query<(&mut BackgroundColor, &ThemeBackgroundColor)>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor)>,
    mut q_text_color: Query<(&mut TextColor, &ThemeTextColor)>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        // Update all background colors
        for (mut bg, theme_bg) in q_background.iter_mut() {
            bg.0 = theme.color(&theme_bg.0);
        }

        // Update all border colors
        for (mut border, theme_border) in q_border.iter_mut() {
            border.set_all(theme.color(&theme_border.0));
        }

        // Update all direct text span colors
        for (mut text_color, theme_text_color) in q_text_color.iter_mut() {
            text_color.0 = theme.color(&theme_text_color.0);
        }
    }
}

pub(crate) fn on_changed_background(
    insert: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.color(&theme_bg.0);
    }
}

pub(crate) fn on_changed_border(
    insert: On<Insert, ThemeBorderColor>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.color(&theme_border.0));
    }
}

pub(crate) fn on_changed_text_color(
    insert: On<Insert, ThemeTextColor>,
    mut q_span: Query<(&mut TextColor, &ThemeTextColor), Changed<ThemeTextColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut text_color, theme_text_color)) = q_span.get_mut(insert.entity) {
        text_color.0 = theme.color(&theme_text_color.0);
    }
}

/// An observer which looks for changes to the [`InheritableThemeTextColor`] component on an entity,
/// and propagates downward the text color to all participating text entities.
pub(crate) fn on_changed_font_color(
    insert: On<Insert, InheritableThemeTextColor>,
    font_color: Query<&InheritableThemeTextColor>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok(token) = font_color.get(insert.entity) {
        let color = theme.color(&token.0);
        commands
            .entity(insert.entity)
            .insert(Propagate(TextColor(color)));
    }
}

// [`EditablePalette`] is the *parametric* form an editor manipulates
// [`EditablePalette::resolve`] bakes it into a [`ResolvedPalette`] — one [`Color`]
// per [`Slot`]. [`build_theme`] then maps each theme token to a slot using
// a mapping which can be got from [`default_token_slots`] and looks up its color

/// A single semantic colour role, these are all the colors that make up
/// a theme.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Slot {
    /// Window & top pane-header backgrounds: `WINDOW_BG`, `PANE_HEADER_BG`.
    #[default]
    Neutral0,
    /// Surface bodies & menus: `PANE_BODY_BG`, `SUBPANE_BODY_BG`, `DIALOG_BG`, `MENU_BG`, `COLOR_PLANE_BG`.
    Neutral1,
    /// Recessed fills & chrome: `SLIDER_BG*`, `SCROLLBAR_BG`, `TEXT_INPUT_BG`, `MENU_BORDER`, 
    /// `MENUITEM_BG_HOVER`, `LISTROW_BG_HOVER`, `BUTTON_PLAIN_BG_HOVER`/`_PRESSED`,
    /// `BUTTON_BG_DISABLED`, `BUTTON_PRIMARY_BG_DISABLED`, `PANE_HEADER_BORDER`/`_DIVIDER`.
    Neutral2,
    /// Raised container headers & borders: `SUBPANE_HEADER_BG`, `GROUP_HEADER_BG`/`_BORDER`,
    /// `GROUP_BODY_BG`/`_BORDER`, `DIALOG_BORDER`, `DIALOG_HEADER_BG`.
    Neutral3,
    /// Control rest bg & borders + selected row: `BUTTON_BG`, `CHECKBOX_BG`/`_BORDER`, `RADIO_BORDER`, 
    /// `SWITCH_BG`/`_BORDER`, `TEXT_INPUT_LABEL_BG`, `MENUITEM_BG_PRESSED`/`_FOCUSED`,
    /// `LISTROW_BG_SELECTED`, `SLIDER_BAR_DISABLED`, `SUBPANE_HEADER_BORDER`, `SUBPANE_BODY_BORDER`.
    Neutral4,
    /// Control hover: `BUTTON_BG_HOVER`, `CHECKBOX_BG`/`_BORDER_HOVER`, `RADIO_BORDER_HOVER`, 
    /// `SWITCH_BG`/`_BORDER_HOVER`.
    Neutral5,
    /// Control pressed: `BUTTON_BG_PRESSED`, `CHECKBOX_BG`/`_BORDER_PRESSED`, `RADIO_BORDER_PRESSED`, 
    /// `SWITCH_BG`/`_BORDER_PRESSED`.
    Neutral6,
    /// Bright on-surface labels: `BUTTON_TEXT`, `MENUITEM_TEXT`, `TEXT_INPUT_TEXT`, `LISTROW_TEXT`,
    /// `PANE_HEADER_TEXT`, `SUBPANE_HEADER_TEXT`, `GROUP_HEADER_TEXT`.
    Text0,
    /// Body text & switch knob: `TEXT_MAIN`, `DIALOG_TEXT`, `CHECKBOX_TEXT`, `RADIO_TEXT`,
    /// `SWITCH_SLIDE_BG`/`_BORDER` (+ hover/pressed).
    Text1,
    /// Disabled bright text + dimmed text: `BUTTON_TEXT_DISABLED`, `BUTTON_PRIMARY_TEXT_DISABLED`, 
    /// `SLIDER_TEXT_DISABLED`, `MENUITEM_TEXT_DISABLED`, `TEXT_INPUT_TEXT_DISABLED`, 
    /// `LISTROW_TEXT_DISABLED`, `TEXT_DIM`.
    TextDim0,
    /// Disabled checkbox/radio/switch borders, marks, knob & label text: the 
    /// `BORDER`/`BORDER_CHECKED`/`MARK`/`TEXT` `_DISABLED` variants of `CHECKBOX_*`/`RADIO_*`, plus 
    /// `SWITCH`'s `BORDER`/`SLIDE_BG`/`SLIDE_BORDER` `_DISABLED` (+ checked). Their `BG*_DISABLED` 
    /// fills are [`Slot::Transparent`].
    TextDim1,
    /// Base call-to-action: `BUTTON_PRIMARY_BG`, `SLIDER_BAR`, `SCROLLBAR_THUMB`, 
    /// `*_BG_CHECKED`/`*_BORDER_CHECKED` (checkbox/switch), `RADIO_BORDER_CHECKED`, `RADIO_MARK`, 
    /// `TEXT_INPUT_SELECTION`.
    Accent0,
    /// Call-to-action hover: `BUTTON_PRIMARY_BG_HOVER`, `SLIDER_BAR_HOVER`/`_PRESSED`, 
    /// `*_BG_CHECKED_HOVER`/`*_BORDER_CHECKED_HOVER` (checkbox/switch), `RADIO_BORDER_CHECKED_HOVER`, 
    /// `RADIO_MARK_HOVER`.
    Accent1,
    /// Call-to-action pressed: `BUTTON_PRIMARY_BG_PRESSED`, `SCROLLBAR_THUMB_HOVER`, 
    /// `*_BG_CHECKED_PRESSED`/`*_BORDER_CHECKED_PRESSED` (checkbox/switch), 
    /// `RADIO_BORDER_CHECKED_PRESSED`, `RADIO_MARK_PRESSED`.
    Accent2,
    /// Brightest accent: `TEXT_INPUT_CURSOR`.
    Accent3,
    /// Foreground over accent-filled components: `BUTTON_PRIMARY_TEXT`, `SLIDER_TEXT`, `CHECKBOX_MARK`, 
    /// `SWITCH_SLIDE_BG`/`_BORDER_CHECKED` (+ hover/pressed).
    Contrast,
    /// Keyboard focus ring: `FOCUS_RING`.
    FocusRing,
    /// Red axis: `TEXT_INPUT_X_AXIS`.
    XAxis,
    /// Green axis: `TEXT_INPUT_Y_AXIS`.
    YAxis,
    /// Blue axis: `TEXT_INPUT_Z_AXIS`.
    ZAxis,
    /// Always [`Color::NONE`]; used by tokens that paint nothing: `BUTTON_PLAIN_BG`/`_DISABLED`, 
    /// all `RADIO_BG*`, `*_BG_DISABLED`/`*_BG_CHECKED_DISABLED` (checkbox/switch), 
    /// `TEXT_INPUT_SELECTION_UNFOCUSED`, `LISTROW_BG`.
    Transparent,
}

impl Slot {
    /// Every slot, in discriminant order (matches [`ResolvedPalette`] storage).
    pub const ALL: [Slot; 21] = [
        Slot::Neutral0,
        Slot::Neutral1,
        Slot::Neutral2,
        Slot::Neutral3,
        Slot::Neutral4,
        Slot::Neutral5,
        Slot::Neutral6,
        Slot::Text0,
        Slot::Text1,
        Slot::TextDim0,
        Slot::TextDim1,
        Slot::Accent0,
        Slot::Accent1,
        Slot::Accent2,
        Slot::Accent3,
        Slot::Contrast,
        Slot::FocusRing,
        Slot::XAxis,
        Slot::YAxis,
        Slot::ZAxis,
        Slot::Transparent,
    ];

    /// Number of slots — the backing size of [`ResolvedPalette`].
    pub const COUNT: usize = Self::ALL.len();

    /// Human-readable name, for editor UI / pickers.
    pub fn label(self) -> &'static str {
        match self {
            Slot::Neutral0 => "Neutral 0",
            Slot::Neutral1 => "Neutral 1",
            Slot::Neutral2 => "Neutral 2",
            Slot::Neutral3 => "Neutral 3",
            Slot::Neutral4 => "Neutral 4",
            Slot::Neutral5 => "Neutral 5",
            Slot::Neutral6 => "Neutral 6",
            Slot::Text0 => "Text 0",
            Slot::Text1 => "Text 1",
            Slot::TextDim0 => "Text Dim 0",
            Slot::TextDim1 => "Text Dim 1",
            Slot::Accent0 => "Accent 0",
            Slot::Accent1 => "Accent 1",
            Slot::Accent2 => "Accent 2",
            Slot::Accent3 => "Accent 3",
            Slot::Contrast => "Contrast",
            Slot::FocusRing => "Focus Ring",
            Slot::XAxis => "X Axis",
            Slot::YAxis => "Y Axis",
            Slot::ZAxis => "Z Axis",
            Slot::Transparent => "Transparent",
        }
    }
}

/// Fully-resolved colours: one [`Color`] per [`Slot`], built by
/// [`EditablePalette::resolve`] and read by [`build_theme`]. Indexed by slot.
#[derive(Clone, Debug)]
pub struct ResolvedPalette([Color; Slot::COUNT]);

impl core::ops::Index<Slot> for ResolvedPalette {
    type Output = Color;
    fn index(&self, slot: Slot) -> &Color {
        &self.0[slot as usize]
    }
}

/// Represents a set of [`Oklcha`] colors which have same hue and chroma but different lightnesses
#[derive(Clone, Debug)]
pub struct OklchaArray<const N: usize> {
    /// Hue of the colors
    pub hue: f32,
    /// Chroma of the colors
    pub chroma: f32,
    /// N lightness values
    pub l: [f32; N],
}

impl<const N: usize> OklchaArray<N> {
    fn to_color(&self, index: usize) -> Color {
        Color::oklcha(self.l[index], self.chroma, self.hue, 1.0)
    }
    fn to_array(&self) -> [Color; N] {
        core::array::from_fn(|index| self.to_color(index))
    }
}

/// The theme's parametric palette
/// Call [`Self::resolve`] to bake it into a [`ResolvedPalette`].
#[derive(Clone, Debug)]
pub struct EditablePalette {
    /// Lightness of each neutral ramp stop; forms [`Slot::Neutral0`]..=[`Slot::Neutral6`].
    pub neutrals: OklchaArray<7>,

    /// Lightness of each accent stop; forms [`Slot::Accent0`]..=[`Slot::Accent3`] 
    /// (and [`Slot::FocusRing`], derived from `accent[0]`).
    pub accent: OklchaArray<4>,

    /// Foreground on accent-filled components (white in most themes); forms [`Slot::Contrast`].
    pub contrast: Oklcha,

    /// Lightness of each text stop; forms [`Slot::Text0`]..=[`Slot::Text1`] 
    /// (and [`Slot::TextDim0`]..=[`Slot::TextDim1`], derived).
    pub text: OklchaArray<2>,

    /// Alpha applied to `text` to derive [`Slot::TextDim0`]..=[`Slot::TextDim1`].
    pub dim_text_alpha_modifier: f32,

    /// RGB axis colors; form [`Slot::XAxis`], [`Slot::YAxis`], [`Slot::ZAxis`].
    pub axes: [Oklcha; 3],
}

impl EditablePalette {
    /// Bake the parametric palette into one resolved colour per [`Slot`].
    ///
    /// The `copy_from_slice` blocks below rely on each ramp's variants being
    /// contiguous and in order within [`Slot`] (Neutral0..=Neutral6, etc.).
    pub fn resolve(&self) -> ResolvedPalette {
        let neutral = self.neutrals.to_array();
        let accent = self.accent.to_array();
        let text = self.text.to_array();
        let text_dim = text.map(|c| c.with_alpha(self.dim_text_alpha_modifier));
        let axes: [Color; 3] = self.axes.map(|c| c.into());

        let mut c = [Color::NONE; Slot::COUNT];
        c[Slot::Neutral0 as usize..=Slot::Neutral6 as usize].copy_from_slice(&neutral);
        c[Slot::Text0 as usize..=Slot::Text1 as usize].copy_from_slice(&text);
        c[Slot::TextDim0 as usize..=Slot::TextDim1 as usize].copy_from_slice(&text_dim);
        c[Slot::Accent0 as usize..=Slot::Accent3 as usize].copy_from_slice(&accent);
        c[Slot::Contrast as usize] = self.contrast.into();
        c[Slot::FocusRing as usize] = accent[0].with_alpha(0.5);
        c[Slot::XAxis as usize..=Slot::ZAxis as usize].copy_from_slice(&axes);
        // Slot::Transparent stays Color::NONE.
        ResolvedPalette(c)
    }
}

/// Build Feathers theme properties by resolving every token's [`Slot`] against `p`.
pub fn build_theme(palette: &ResolvedPalette, token_slots: &[(ThemeToken, Slot)]) -> ThemeProps {
    ThemeProps {
        color: token_slots
            .iter()
            .map(|(token, slot)| (token.clone(), palette[*slot]))
            .collect(),
    }
}

/// Get the 3 default axis colors (RGB)
pub fn default_axis_colors() -> [Oklcha; 3] {
    [
        Oklcha::new(0.5232, 0.1404, 13.84, 1.0),
        Oklcha::new(0.5866, 0.1543, 129.84, 1.0),
        Oklcha::new(0.4847, 0.1249, 253.08, 1.0),
    ]
}

/// Default mapping from each token to a [`Slot`]
pub fn default_token_slots() -> &'static [(ThemeToken, Slot)] {
    const DEFAULT_TOKEN_SLOTS: &[(ThemeToken, Slot)] = &[
        (tokens::WINDOW_BG, Slot::Neutral0),
        (tokens::FOCUS_RING, Slot::FocusRing),
        (tokens::TEXT_MAIN, Slot::Text1),
        (tokens::TEXT_DIM, Slot::TextDim0),
        (tokens::BUTTON_BG, Slot::Neutral4),
        (tokens::BUTTON_BG_HOVER, Slot::Neutral5),
        (tokens::BUTTON_BG_PRESSED, Slot::Neutral6),
        (tokens::BUTTON_BG_DISABLED, Slot::Neutral2),
        (tokens::BUTTON_PRIMARY_BG, Slot::Accent0),
        (tokens::BUTTON_PRIMARY_BG_HOVER, Slot::Accent1),
        (tokens::BUTTON_PRIMARY_BG_PRESSED, Slot::Accent2),
        (tokens::BUTTON_PRIMARY_BG_DISABLED, Slot::Neutral2),
        (tokens::BUTTON_PLAIN_BG, Slot::Transparent),
        (tokens::BUTTON_PLAIN_BG_HOVER, Slot::Neutral2),
        (tokens::BUTTON_PLAIN_BG_PRESSED, Slot::Neutral2),
        (tokens::BUTTON_PLAIN_BG_DISABLED, Slot::Transparent),
        (tokens::BUTTON_TEXT, Slot::Text0),
        (tokens::BUTTON_TEXT_DISABLED, Slot::TextDim0),
        (tokens::BUTTON_PRIMARY_TEXT, Slot::Contrast),
        (tokens::BUTTON_PRIMARY_TEXT_DISABLED, Slot::TextDim0),
        (tokens::SLIDER_BG, Slot::Neutral2),
        (tokens::SLIDER_BG_HOVER, Slot::Neutral2),
        (tokens::SLIDER_BG_PRESSED, Slot::Neutral2),
        (tokens::SLIDER_BG_DISABLED, Slot::Neutral2),
        (tokens::SLIDER_BAR, Slot::Accent0),
        (tokens::SLIDER_BAR_HOVER, Slot::Accent1),
        (tokens::SLIDER_BAR_PRESSED, Slot::Accent1),
        (tokens::SLIDER_BAR_DISABLED, Slot::Neutral4),
        (tokens::SLIDER_TEXT, Slot::Contrast),
        (tokens::SLIDER_TEXT_DISABLED, Slot::TextDim0),
        (tokens::SCROLLBAR_BG, Slot::Neutral2),
        (tokens::SCROLLBAR_THUMB, Slot::Accent0),
        (tokens::SCROLLBAR_THUMB_HOVER, Slot::Accent2),
        (tokens::CHECKBOX_BG, Slot::Neutral4),
        (tokens::CHECKBOX_BG_HOVER, Slot::Neutral5),
        (tokens::CHECKBOX_BG_PRESSED, Slot::Neutral6),
        (tokens::CHECKBOX_BG_DISABLED, Slot::Transparent),
        (tokens::CHECKBOX_BG_CHECKED, Slot::Accent0),
        (tokens::CHECKBOX_BG_CHECKED_HOVER, Slot::Accent1),
        (tokens::CHECKBOX_BG_CHECKED_PRESSED, Slot::Accent2),
        (tokens::CHECKBOX_BG_CHECKED_DISABLED, Slot::Transparent),
        (tokens::CHECKBOX_BORDER, Slot::Neutral4),
        (tokens::CHECKBOX_BORDER_HOVER, Slot::Neutral5),
        (tokens::CHECKBOX_BORDER_PRESSED, Slot::Neutral6),
        (tokens::CHECKBOX_BORDER_DISABLED, Slot::TextDim1),
        (tokens::CHECKBOX_BORDER_CHECKED, Slot::Accent0),
        (tokens::CHECKBOX_BORDER_CHECKED_HOVER, Slot::Accent1),
        (tokens::CHECKBOX_BORDER_CHECKED_PRESSED, Slot::Accent2),
        (tokens::CHECKBOX_BORDER_CHECKED_DISABLED, Slot::TextDim1),
        (tokens::CHECKBOX_MARK, Slot::Contrast),
        (tokens::CHECKBOX_MARK_DISABLED, Slot::TextDim1),
        (tokens::CHECKBOX_TEXT, Slot::Text1),
        (tokens::CHECKBOX_TEXT_DISABLED, Slot::TextDim1),
        (tokens::RADIO_BG, Slot::Transparent),
        (tokens::RADIO_BG_HOVER, Slot::Transparent),
        (tokens::RADIO_BG_PRESSED, Slot::Transparent),
        (tokens::RADIO_BG_DISABLED, Slot::Transparent),
        (tokens::RADIO_BG_CHECKED, Slot::Transparent),
        (tokens::RADIO_BG_CHECKED_HOVER, Slot::Transparent),
        (tokens::RADIO_BG_CHECKED_PRESSED, Slot::Transparent),
        (tokens::RADIO_BG_CHECKED_DISABLED, Slot::Transparent),
        (tokens::RADIO_BORDER, Slot::Neutral4),
        (tokens::RADIO_BORDER_HOVER, Slot::Neutral5),
        (tokens::RADIO_BORDER_PRESSED, Slot::Neutral6),
        (tokens::RADIO_BORDER_DISABLED, Slot::TextDim1),
        (tokens::RADIO_BORDER_CHECKED, Slot::Accent0),
        (tokens::RADIO_BORDER_CHECKED_HOVER, Slot::Accent1),
        (tokens::RADIO_BORDER_CHECKED_PRESSED, Slot::Accent2),
        (tokens::RADIO_BORDER_CHECKED_DISABLED, Slot::TextDim1),
        (tokens::RADIO_MARK, Slot::Accent0),
        (tokens::RADIO_MARK_HOVER, Slot::Accent1),
        (tokens::RADIO_MARK_PRESSED, Slot::Accent2),
        (tokens::RADIO_MARK_DISABLED, Slot::TextDim1),
        (tokens::RADIO_TEXT, Slot::Text1),
        (tokens::RADIO_TEXT_DISABLED, Slot::TextDim1),
        (tokens::SWITCH_BG, Slot::Neutral4),
        (tokens::SWITCH_BG_HOVER, Slot::Neutral5),
        (tokens::SWITCH_BG_PRESSED, Slot::Neutral6),
        (tokens::SWITCH_BG_DISABLED, Slot::Transparent),
        (tokens::SWITCH_BG_CHECKED, Slot::Accent0),
        (tokens::SWITCH_BG_CHECKED_HOVER, Slot::Accent1),
        (tokens::SWITCH_BG_CHECKED_PRESSED, Slot::Accent2),
        (tokens::SWITCH_BG_CHECKED_DISABLED, Slot::Transparent),
        (tokens::SWITCH_BORDER, Slot::Neutral4),
        (tokens::SWITCH_BORDER_HOVER, Slot::Neutral5),
        (tokens::SWITCH_BORDER_PRESSED, Slot::Neutral6),
        (tokens::SWITCH_BORDER_DISABLED, Slot::TextDim1),
        (tokens::SWITCH_BORDER_CHECKED, Slot::Accent0),
        (tokens::SWITCH_BORDER_CHECKED_HOVER, Slot::Accent1),
        (tokens::SWITCH_BORDER_CHECKED_PRESSED, Slot::Accent2),
        (tokens::SWITCH_BORDER_CHECKED_DISABLED, Slot::TextDim1),
        (tokens::SWITCH_SLIDE_BG, Slot::Text1),
        (tokens::SWITCH_SLIDE_BG_HOVER, Slot::Text1),
        (tokens::SWITCH_SLIDE_BG_PRESSED, Slot::Text1),
        (tokens::SWITCH_SLIDE_BG_DISABLED, Slot::TextDim1),
        (tokens::SWITCH_SLIDE_BG_CHECKED, Slot::Contrast),
        (tokens::SWITCH_SLIDE_BG_CHECKED_HOVER, Slot::Contrast),
        (tokens::SWITCH_SLIDE_BG_CHECKED_PRESSED, Slot::Contrast),
        (tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED, Slot::TextDim1),
        (tokens::SWITCH_SLIDE_BORDER, Slot::Text1),
        (tokens::SWITCH_SLIDE_BORDER_HOVER, Slot::Text1),
        (tokens::SWITCH_SLIDE_BORDER_PRESSED, Slot::Text1),
        (tokens::SWITCH_SLIDE_BORDER_DISABLED, Slot::TextDim1),
        (tokens::SWITCH_SLIDE_BORDER_CHECKED, Slot::Contrast),
        (tokens::SWITCH_SLIDE_BORDER_CHECKED_HOVER, Slot::Contrast),
        (tokens::SWITCH_SLIDE_BORDER_CHECKED_PRESSED, Slot::Contrast),
        (tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED, Slot::TextDim1),
        (tokens::COLOR_PLANE_BG, Slot::Neutral1),
        (tokens::MENU_BG, Slot::Neutral1),
        (tokens::MENU_BORDER, Slot::Neutral2),
        (tokens::MENUITEM_BG_HOVER, Slot::Neutral2),
        (tokens::MENUITEM_BG_PRESSED, Slot::Neutral4),
        (tokens::MENUITEM_BG_FOCUSED, Slot::Neutral4),
        (tokens::MENUITEM_TEXT, Slot::Text0),
        (tokens::MENUITEM_TEXT_DISABLED, Slot::TextDim0),
        (tokens::TEXT_INPUT_BG, Slot::Neutral2),
        (tokens::TEXT_INPUT_LABEL_BG, Slot::Neutral4),
        (tokens::TEXT_INPUT_TEXT, Slot::Text0),
        (tokens::TEXT_INPUT_TEXT_DISABLED, Slot::TextDim0),
        (tokens::TEXT_INPUT_CURSOR, Slot::Accent3),
        (tokens::TEXT_INPUT_SELECTION, Slot::Accent0),
        (tokens::TEXT_INPUT_SELECTION_UNFOCUSED, Slot::Transparent),
        (tokens::TEXT_INPUT_X_AXIS, Slot::XAxis),
        (tokens::TEXT_INPUT_Y_AXIS, Slot::YAxis),
        (tokens::TEXT_INPUT_Z_AXIS, Slot::ZAxis),
        (tokens::PANE_HEADER_BG, Slot::Neutral0),
        (tokens::PANE_HEADER_BORDER, Slot::Neutral2),
        (tokens::PANE_HEADER_TEXT, Slot::Text0),
        (tokens::PANE_HEADER_DIVIDER, Slot::Neutral2),
        (tokens::PANE_BODY_BG, Slot::Neutral1),
        (tokens::SUBPANE_HEADER_BG, Slot::Neutral3),
        (tokens::SUBPANE_HEADER_BORDER, Slot::Neutral4),
        (tokens::SUBPANE_HEADER_TEXT, Slot::Text0),
        (tokens::SUBPANE_BODY_BG, Slot::Neutral1),
        (tokens::SUBPANE_BODY_BORDER, Slot::Neutral4),
        (tokens::GROUP_HEADER_BG, Slot::Neutral3),
        (tokens::GROUP_HEADER_BORDER, Slot::Neutral3),
        (tokens::GROUP_HEADER_TEXT, Slot::Text0),
        (tokens::GROUP_BODY_BG, Slot::Neutral3),
        (tokens::GROUP_BODY_BORDER, Slot::Neutral3),
        (tokens::LISTROW_BG, Slot::Transparent),
        (tokens::LISTROW_BG_HOVER, Slot::Neutral2),
        (tokens::LISTROW_BG_SELECTED, Slot::Neutral4),
        (tokens::LISTROW_TEXT, Slot::Text0),
        (tokens::LISTROW_TEXT_DISABLED, Slot::TextDim0),
        (tokens::DIALOG_BG, Slot::Neutral1),
        (tokens::DIALOG_BORDER, Slot::Neutral3),
        (tokens::DIALOG_HEADER_BG, Slot::Neutral3),
        (tokens::DIALOG_TEXT, Slot::Text1),
        //(tokens::DIALOG_HEADER_TEXT, Slot::Text0),
    ];
    &DEFAULT_TOKEN_SLOTS
}
