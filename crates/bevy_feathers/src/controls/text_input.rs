use bevy_app::{Plugin, PreUpdate, PropagateOver};
use bevy_asset::AssetServer;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::PickingSystems;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, FontSource, FontWeight, LineBreak, TextCursorStyle, TextFont, TextLayout,
};
use bevy_ui::{
    px, AlignItems, BorderRadius, Display, InteractionDisabled, JustifyContent, Node, UiRect,
};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    focus::FocusWithinIndicator,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemedText, UiTheme},
    tokens,
};

/// Decorative frame around a text input widget. This is a separate entity to allow icons
/// (such as "search" or "clear") to be inserted adjacent to the input.
///
/// This is spawnable by inheriting it as a "scene component".
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersTextInputContainer;

impl FeathersTextInputContainer {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect {
                    right: px(3.0),
                    left: px(3.0),
                },
                flex_grow: 1.0,
                border_radius: {BorderRadius::all(px(4.0))},
                column_gap: px(4),
            }
            FeathersTextInputContainer
            FocusWithinIndicator
            ThemeBackgroundColor(tokens::TEXT_INPUT_BG)
            InheritableThemeTextColor(tokens::TEXT_INPUT_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::COMPACT_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}

/// Scene function to spawn a decorated text input.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`FeathersTextInputProps`].
///
/// ```ignore
/// @FeathersTextInput
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(FeathersTextInputProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersTextInput;

/// Props used to construct the [`FeathersTextInput`] scene.
pub struct FeathersTextInputProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
    /// Components and observers to add to the editable text entity.
    pub input: Box<dyn Scene>,
    /// Additional controls to place before the editable text entity.
    pub leading_controls: Box<dyn SceneList>,
    /// Additional controls to place after the editable text entity.
    pub extra_controls: Box<dyn SceneList>,
}

impl Default for FeathersTextInputProps {
    fn default() -> Self {
        Self {
            visible_width: None,
            max_characters: None,
            input: Box::new(bsn!()),
            leading_controls: Box::new(bsn_list!()),
            extra_controls: Box::new(bsn_list!()),
        }
    }
}

impl FeathersTextInput {
    fn scene(props: FeathersTextInputProps) -> impl Scene {
        bsn! {
            @FeathersTextInputContainer
            FeathersTextInput
            Children [{props.leading_controls}, (
                @FeathersTextInputBare {
                    @visible_width: {props.visible_width},
                    @max_characters: {props.max_characters},
                }
                {props.input}
            ), {props.extra_controls}]
        }
    }
}

fn update_text_input_disabled(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>),
        (
            With<FeathersTextInput>,
            Or<(Added<FeathersTextInput>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    q_bare_input: Query<(), With<FeathersTextInputBare>>,
    mut commands: Commands,
) {
    for (entity, disabled) in &q_inputs {
        if let Some(input) = q_children
            .iter_descendants(entity)
            .find(|entity| q_bare_input.contains(*entity))
        {
            if disabled {
                commands.entity(input).insert(InteractionDisabled);
            } else {
                commands.entity(input).remove::<InteractionDisabled>();
            }
        }
    }
}

fn update_text_input_disabled_remove(
    q_inputs: Query<(), With<FeathersTextInput>>,
    q_children: Query<&Children>,
    q_bare_input: Query<(), With<FeathersTextInputBare>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    for entity in removed_disabled
        .read()
        .filter(|entity| q_inputs.contains(*entity))
    {
        if let Some(input) = q_children
            .iter_descendants(entity)
            .find(|entity| q_bare_input.contains(*entity))
        {
            commands.entity(input).remove::<InteractionDisabled>();
        }
    }
}

/// An undecorated text input for embedding in custom containers.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersTextInputBareProps)]
#[reflect(Component, Default, Clone)]
pub struct FeathersTextInputBare;

/// Props used to construct a [`FeathersTextInputBare`] scene.
#[derive(Default)]
pub struct FeathersTextInputBareProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
}

impl FeathersTextInputBare {
    fn scene(props: FeathersTextInputBareProps) -> impl Scene {
        bsn! {
            Node {
                flex_grow: {
                    if props.visible_width.is_some() {
                        0_f32
                    } else {
                        1_f32
                    }
                } ,
            }
            FeathersTextInputBare
            EditableText {
                cursor_width: 0.3,
                visible_width: {props.visible_width},
                max_characters: {props.max_characters},
            }
            ThemedText
            TextLayout {
                linebreak: LineBreak::NoWrap,
            }
            TabIndex(0)
            template(|ctx| {
                Ok(TextFont {
                    font: FontSource::Handle(ctx.resource::<AssetServer>().load(fonts::REGULAR)),
                    font_size: size::COMPACT_FONT,
                    weight: FontWeight::NORMAL,
                    ..Default::default()
                })
            })
            PropagateOver<TextFont>
            EntityCursor::System(bevy_window::SystemCursorIcon::Text)
            TextCursorStyle::default()
        }
    }
}

fn update_text_cursor_color(
    mut q_text_input: Query<&mut TextCursorStyle, With<FeathersTextInputBare>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for mut cursor_style in q_text_input.iter_mut() {
            cursor_style.color = theme.color(&tokens::TEXT_INPUT_CURSOR);
            cursor_style.selection_color = theme.color(&tokens::TEXT_INPUT_SELECTION);
            cursor_style.unfocused_selection_color =
                theme.color(&tokens::TEXT_INPUT_SELECTION_UNFOCUSED);
        }
    }
}

fn update_text_input_styles(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        (With<FeathersTextInputBare>, Added<InteractionDisabled>),
    >,
    mut commands: Commands,
) {
    for (input_ent, disabled, font_color) in q_inputs.iter() {
        set_text_input_styles(input_ent, disabled, font_color, &mut commands);
    }
}

fn update_text_input_styles_remove(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        With<FeathersTextInputBare>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|ent| {
        if let Ok((input_ent, disabled, font_color)) = q_inputs.get(ent) {
            set_text_input_styles(input_ent, disabled, font_color, &mut commands);
        }
    });
}

fn set_text_input_styles(
    input_ent: Entity,
    disabled: bool,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let font_color_token = match disabled {
        true => tokens::TEXT_INPUT_TEXT_DISABLED,
        false => tokens::TEXT_INPUT_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Text,
    };

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(input_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(input_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the text input styles.
pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_text_cursor_color,
                (
                    update_text_input_disabled_remove,
                    update_text_input_disabled,
                    update_text_input_styles,
                    update_text_input_styles_remove,
                )
                    .chain(),
            )
                .in_set(PickingSystems::Last),
        );
    }
}
