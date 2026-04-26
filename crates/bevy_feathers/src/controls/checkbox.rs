use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    spawn::{Spawn, SpawnRelated, SpawnableList},
    system::{Commands, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_math::Rot2;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BorderRadius, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, PositionType, Pressed, UiRect, UiTransform, Val,
};
use bevy_ui_widgets::{ActivateOnPress, Checkbox};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    focus::FocusIndicator,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// Parameters for the checkbox template, passed to [`checkbox`] function.
pub struct CheckboxProps {
    /// Label for this checkbox. This can contain multiple entities, which will be contained
    /// in a flexbox.
    pub caption: Box<dyn SceneList>,
}

impl Default for CheckboxProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
        }
    }
}

/// Marker for the checkbox frame (contains both checkbox and label)
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxFrame;

/// Marker for the checkbox outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxOutline;

/// Marker for the checkbox check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxMark;

/// Scene function to spawn a checkbox.
///
/// # Emitted events
/// * [`bevy_ui_widgets::ValueChange<bool>`] with the new value when the checkbox changes state.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
pub fn checkbox(props: CheckboxProps) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
        }
        Checkbox
        CheckboxFrame
        Hovered
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
        TabIndex(0)
        InheritableThemeTextColor(tokens::CHECKBOX_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [(
            Node {
                width: size::CHECKBOX_SIZE,
                height: size::CHECKBOX_SIZE,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
            }
            CheckboxOutline
            ThemeBackgroundColor(tokens::CHECKBOX_BG)
            ThemeBorderColor(tokens::CHECKBOX_BORDER)
            FocusIndicator
            Children [(
                // Cheesy checkmark: rotated node with L-shaped border.
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    top: Val::Px(0.0),
                    width: Val::Px(6.),
                    height: Val::Px(11.),
                    border: UiRect {
                        bottom: Val::Px(2.0),
                        right: Val::Px(2.0),
                    },
                }
                UiTransform::from_rotation(Rot2::FRAC_PI_4)
                CheckboxMark
                ThemeBorderColor(tokens::CHECKBOX_MARK)
            )]),
            {props.caption}
        ]
    }
}

/// Template function to spawn a checkbox.
///
/// This version does not take any props. A caption can be set by appending a child entity.
///
/// # Emitted events
/// * [`bevy_ui_widgets::ValueChange<bool>`] with the new value when the checkbox changes state.
///
/// These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
#[deprecated(since = "0.19.0", note = "Use the checkbox() BSN function")]
pub fn checkbox_bundle<C: SpawnableList<ChildOf> + Send + Sync + 'static, B: Bundle>(
    overrides: B,
    label: C,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..Default::default()
        },
        Checkbox,
        CheckboxFrame,
        Hovered::default(),
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer),
        TabIndex(0),
        InheritableThemeTextColor(tokens::CHECKBOX_TEXT),
        InheritableFont {
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
            ..Default::default()
        },
        overrides,
        Children::spawn((
            Spawn((
                Node {
                    width: size::CHECKBOX_SIZE,
                    height: size::CHECKBOX_SIZE,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..Default::default()
                },
                FocusIndicator,
                CheckboxOutline,
                ThemeBackgroundColor(tokens::CHECKBOX_BG),
                ThemeBorderColor(tokens::CHECKBOX_BORDER),
                children![(
                    // Cheesy checkmark: rotated node with L-shaped border.
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(0.0),
                        width: Val::Px(6.),
                        height: Val::Px(11.),
                        border: UiRect {
                            bottom: Val::Px(2.0),
                            right: Val::Px(2.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    UiTransform::from_rotation(Rot2::FRAC_PI_4),
                    CheckboxMark,
                    ThemeBorderColor(tokens::CHECKBOX_MARK),
                )],
            )),
            label,
        )),
    )
}

fn update_checkbox_styles(
    q_checkboxes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Pressed>,
            Has<ActivateOnPress>,
            &Hovered,
            &InheritableThemeTextColor,
        ),
        (
            With<CheckboxFrame>,
            Or<(
                Changed<Hovered>,
                Added<Checked>,
                Added<Pressed>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<(&ThemeBackgroundColor, &ThemeBorderColor), With<CheckboxOutline>>,
    mut q_mark: Query<&ThemeBorderColor, With<CheckboxMark>>,
    mut commands: Commands,
) {
    for (checkbox_ent, disabled, checked, pressed, activate_on_press, hovered, font_color) in
        q_checkboxes.iter()
    {
        let Some(outline_ent) = q_children
            .iter_descendants(checkbox_ent)
            .find(|en| q_outline.contains(*en))
        else {
            continue;
        };
        let Some(mark_ent) = q_children
            .iter_descendants(checkbox_ent)
            .find(|en| q_mark.contains(*en))
        else {
            continue;
        };
        let (outline_bg, outline_border) = q_outline.get_mut(outline_ent).unwrap();
        let mark_color = q_mark.get_mut(mark_ent).unwrap();
        set_checkbox_styles(
            checkbox_ent,
            outline_ent,
            mark_ent,
            disabled,
            checked,
            pressed,
            hovered.0,
            activate_on_press,
            outline_bg,
            outline_border,
            mark_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_checkbox_styles_remove(
    q_checkboxes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Pressed>,
            Has<ActivateOnPress>,
            &Hovered,
            &InheritableThemeTextColor,
        ),
        With<CheckboxFrame>,
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<(&ThemeBackgroundColor, &ThemeBorderColor), With<CheckboxOutline>>,
    mut q_mark: Query<&ThemeBorderColor, With<CheckboxMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut remove_pressed: RemovedComponents<Pressed>,
    mut remove_activate_on_press: RemovedComponents<ActivateOnPress>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .chain(remove_pressed.read())
        .chain(remove_activate_on_press.read())
        .for_each(|ent| {
            if let Ok((
                checkbox_ent,
                disabled,
                checked,
                pressed,
                activate_on_press,
                hovered,
                font_color,
            )) = q_checkboxes.get(ent)
            {
                let Some(outline_ent) = q_children
                    .iter_descendants(checkbox_ent)
                    .find(|en| q_outline.contains(*en))
                else {
                    return;
                };
                let Some(mark_ent) = q_children
                    .iter_descendants(checkbox_ent)
                    .find(|en| q_mark.contains(*en))
                else {
                    return;
                };
                let (outline_bg, outline_border) = q_outline.get_mut(outline_ent).unwrap();
                let mark_color = q_mark.get_mut(mark_ent).unwrap();
                set_checkbox_styles(
                    checkbox_ent,
                    outline_ent,
                    mark_ent,
                    disabled,
                    checked,
                    pressed,
                    hovered.0,
                    activate_on_press,
                    outline_bg,
                    outline_border,
                    mark_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_checkbox_styles(
    checkbox_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    pressed: bool,
    hovered: bool,
    activate_on_press: bool,
    outline_bg: &ThemeBackgroundColor,
    outline_border: &ThemeBorderColor,
    mark_color: &ThemeBorderColor,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let outline_border_token = if checked {
        if disabled {
            tokens::CHECKBOX_BORDER_CHECKED_DISABLED
        } else if pressed && !activate_on_press {
            tokens::CHECKBOX_BORDER_CHECKED_PRESSED
        } else if hovered {
            tokens::CHECKBOX_BORDER_CHECKED_HOVER
        } else {
            tokens::CHECKBOX_BORDER_CHECKED
        }
    } else {
        if disabled {
            tokens::CHECKBOX_BORDER_DISABLED
        } else if pressed && !activate_on_press {
            tokens::CHECKBOX_BORDER_PRESSED
        } else if hovered {
            tokens::CHECKBOX_BORDER_HOVER
        } else {
            tokens::CHECKBOX_BORDER
        }
    };

    let outline_bg_token = if checked {
        if disabled {
            tokens::CHECKBOX_BG_CHECKED_DISABLED
        } else if pressed && !activate_on_press {
            tokens::CHECKBOX_BG_CHECKED_PRESSED
        } else if hovered {
            tokens::CHECKBOX_BG_CHECKED_HOVER
        } else {
            tokens::CHECKBOX_BG_CHECKED
        }
    } else {
        if disabled {
            tokens::CHECKBOX_BG_DISABLED
        } else if pressed && !activate_on_press {
            tokens::CHECKBOX_BG_PRESSED
        } else if hovered {
            tokens::CHECKBOX_BG_HOVER
        } else {
            tokens::CHECKBOX_BG
        }
    };

    let mark_token = match disabled {
        true => tokens::CHECKBOX_MARK_DISABLED,
        false => tokens::CHECKBOX_MARK,
    };

    let font_color_token = match disabled {
        true => tokens::CHECKBOX_TEXT_DISABLED,
        false => tokens::CHECKBOX_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline background
    if outline_bg.0 != outline_bg_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change outline border
    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }

    // Change mark color
    if mark_color.0 != mark_token {
        commands
            .entity(mark_ent)
            .insert(ThemeBorderColor(mark_token));
    }

    // Change mark visibility
    commands.entity(mark_ent).insert(match checked {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    });

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(checkbox_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(checkbox_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the checkbox styles.
pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_checkbox_styles, update_checkbox_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
