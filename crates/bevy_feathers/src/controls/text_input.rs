use bevy_app::{Plugin, PreUpdate, PropagateOver};
use bevy_asset::AssetServer;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    lifecycle::RemovedComponents,
    query::{Added, Has, With},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::PickingSystems;
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
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, UiTheme},
    tokens,
};

/// Marker to indicate a text input widget with feathers styling.
#[derive(Component, Default, Clone)]
struct FeathersTextInputContainer;

/// Marker to indicate the inner part of the text input widget.
#[derive(Component, Default, Clone)]
struct FeathersTextInput;

/// Parameters for the text input template, passed to [`text_input`] function.
#[derive(Default, Clone)]
pub struct TextInputProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
}

/// Decorative frame around a text input widget. This is a separate entity to allow icons
/// (such as "search" or "clear") to be inserted adjacent to the input.
pub fn text_input_container() -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect {
                right: px(3.0),
            },
            border: UiRect {
                left: px(3.0)
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

/// Scene function to spawn a text input. For proper styling, this should be enclosed by a
/// `text_input_container`.
///
/// ```ignore
/// :text_input_container
/// Children [
///     text_input(props)
/// ]
/// ```
///
/// # Arguments
/// * `props` - construction properties for the text input.
pub fn text_input(props: TextInputProps) -> impl Scene {
    bsn! {
        Node {
            flex_grow: {
                if props.visible_width.is_some() {
                    0.
                } else {
                    1.
                }
            } ,
        }
        FeathersTextInput
        EditableText {
            cursor_width: 0.3,
            visible_width: {props.visible_width},
            max_characters: {props.max_characters},
        }
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

fn update_text_cursor_color(
    mut q_text_input: Query<&mut TextCursorStyle, With<FeathersTextInput>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for mut cursor_style in q_text_input.iter_mut() {
            cursor_style.color = theme.color(&tokens::TEXT_INPUT_CURSOR);
            cursor_style.selection_color = theme.color(&tokens::TEXT_INPUT_SELECTION);
        }
    }
}

fn update_text_input_styles(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        (With<FeathersTextInput>, Added<InteractionDisabled>),
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
        With<FeathersTextInput>,
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
                update_text_input_styles,
                update_text_input_styles_remove,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
