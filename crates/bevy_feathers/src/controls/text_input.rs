use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    spawn::{SpawnRelated, SpawnableList},
    system::{Commands, Query, Res},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene2::{prelude::*, template_value};
use bevy_text::{
    EditableText, FontSize, FontWeight, LineBreak, TextCursorStyle, TextFont, TextLayout,
};
use bevy_ui::{
    px, AlignItems, AlignSelf, BorderColor, BorderRadius, Display, InteractionDisabled,
    JustifyContent, Node, UiRect, Val,
};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    theme::{ThemeBackgroundColor, ThemeFontColor, ThemedText, UiTheme},
    tokens,
};

/// Marker to indicate a text input widget with feathers styling.
#[derive(Component, Default, Clone)]
struct FeathersTextInput;

/// Marker to indicate a the inner part of the text input widget.
#[derive(Component, Default, Clone)]
struct FeathersTextInputInner;

/// Parameters for the text input template, passed to [`text_input`] function.
pub struct TextInputProps {
    /// Icons to be placed to the left of the input
    pub adorn_left: Option<Box<dyn SceneList>>,
    /// Icons to be placed to the right of the input
    pub adorn_right: Option<Box<dyn SceneList>>,
    /// Max characters
    pub max_characters: Option<usize>,
}

/// Scene function to spawn a text input.
///
/// # Arguments
/// * `props` - construction properties for the text input.
pub fn text_input(props: TextInputProps) -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(6.0), Val::Px(0.)),
            flex_grow: 1.0,
            border_radius: {BorderRadius::all(px(4.0))},
        }
        FeathersTextInput
        ThemeBackgroundColor(tokens::TEXT_INPUT_BG)
        ThemeFontColor(tokens::TEXT_INPUT_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: FontSize::Px(13.0),
            weight: FontWeight::NORMAL,
        }
        Children [
            // {props.adorn_left},
            (
                Node {
                    flex_grow: 1.0,
                }
                FeathersTextInputInner
                EditableText {
                    cursor_width: 0.3,
                    max_characters: {props.max_characters},
                }
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                }
                TabIndex(0)
                ThemedText
                EntityCursor::System(bevy_window::SystemCursorIcon::Text)
                TextCursorStyle::default()
            )
            // {props.adorn_right},
        ]
    }
}

fn update_text_cursor_color(
    mut q_text_input: Query<&mut TextCursorStyle, With<FeathersTextInputInner>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for mut cursor_style in q_text_input.iter_mut() {
            cursor_style.color = theme.color(&tokens::TEXT_INPUT_CURSOR);
            cursor_style.selection_color = theme.color(&tokens::TEXT_INPUT_SELECTION);
        }
    }
}

// fn update_text_input_styles(
//     q_buttons: Query<
//         (
//             Entity,
//             &FeathersTextInput,
//             Has<InteractionDisabled>,
//             &ThemeBackgroundColor,
//             &ThemeFontColor,
//         ),
//         Added<InteractionDisabled>,
//     >,
//     mut commands: Commands,
// ) {
//     for (button_ent, variant, disabled, pressed, hovered, bg_color, font_color) in q_buttons.iter()
//     {
//         set_text_input_styles(
//             button_ent,
//             variant,
//             disabled,
//             pressed,
//             hovered.0,
//             bg_color,
//             font_color,
//             &mut commands,
//         );
//     }
// }

// fn update_text_input_styles_remove(
//     q_buttons: Query<(
//         Entity,
//         &FeathersTextInput,
//         Has<InteractionDisabled>,
//         &ThemeBackgroundColor,
//         &ThemeFontColor,
//     )>,
//     mut removed_disabled: RemovedComponents<InteractionDisabled>,
//     mut removed_pressed: RemovedComponents<Pressed>,
//     mut commands: Commands,
// ) {
//     removed_disabled
//         .read()
//         .chain(removed_pressed.read())
//         .for_each(|ent| {
//             if let Ok((button_ent, variant, disabled, pressed, hovered, bg_color, font_color)) =
//                 q_buttons.get(ent)
//             {
//                 set_button_styles(
//                     button_ent,
//                     variant,
//                     disabled,
//                     pressed,
//                     hovered.0,
//                     bg_color,
//                     font_color,
//                     &mut commands,
//                 );
//             }
//         });
// }

// fn set_text_input_styles(
//     button_ent: Entity,
//     variant: &ButtonVariant,
//     disabled: bool,
//     pressed: bool,
//     hovered: bool,
//     bg_color: &ThemeBackgroundColor,
//     font_color: &ThemeFontColor,
//     commands: &mut Commands,
// ) {
//     let bg_token = match (variant, disabled, pressed, hovered) {
//         (ButtonVariant::Normal, true, _, _) => tokens::BUTTON_BG_DISABLED,
//         (ButtonVariant::Normal, false, true, _) => tokens::BUTTON_BG_PRESSED,
//         (ButtonVariant::Normal, false, false, true) => tokens::BUTTON_BG_HOVER,
//         (ButtonVariant::Normal, false, false, false) => tokens::BUTTON_BG,
//         (ButtonVariant::Primary, true, _, _) => tokens::BUTTON_PRIMARY_BG_DISABLED,
//         (ButtonVariant::Primary, false, true, _) => tokens::BUTTON_PRIMARY_BG_PRESSED,
//         (ButtonVariant::Primary, false, false, true) => tokens::BUTTON_PRIMARY_BG_HOVER,
//         (ButtonVariant::Primary, false, false, false) => tokens::BUTTON_PRIMARY_BG,
//     };

//     let font_color_token = match (variant, disabled) {
//         (ButtonVariant::Normal, true) => tokens::BUTTON_TEXT_DISABLED,
//         (ButtonVariant::Normal, false) => tokens::BUTTON_TEXT,
//         (ButtonVariant::Primary, true) => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
//         (ButtonVariant::Primary, false) => tokens::BUTTON_PRIMARY_TEXT,
//     };

//     let cursor_shape = match disabled {
//         true => bevy_window::SystemCursorIcon::NotAllowed,
//         false => bevy_window::SystemCursorIcon::Pointer,
//     };

//     // Change background color
//     if bg_color.0 != bg_token {
//         commands
//             .entity(button_ent)
//             .insert(ThemeBackgroundColor(bg_token));
//     }

//     // Change font color
//     if font_color.0 != font_color_token {
//         commands
//             .entity(button_ent)
//             .insert(ThemeFontColor(font_color_token));
//     }

//     // Change cursor shape
//     commands
//         .entity(button_ent)
//         .insert(EntityCursor::System(cursor_shape));
// }

/// Plugin which registers the systems for updating the text input styles.
pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_text_cursor_color).in_set(PickingSystems::Last),
        );
    }
}
