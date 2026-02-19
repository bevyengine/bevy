use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_core_widgets::CoreRadio;
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
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    AlignItems, BorderRadius, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, UiRect, Val,
};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    handle_or_path::HandleOrPath,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};

/// Marker for the radio outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioOutline;

/// Marker for the radio check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioMark;

/// Template function to spawn a radio.
///
/// # Arguments
/// * `props` - construction properties for the radio.
/// * `overrides` - a bundle of components that are merged in with the normal radio components.
/// * `label` - the label of the radio.
pub fn radio<C: SpawnableList<ChildOf> + Send + Sync + 'static, B: Bundle>(
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
        CoreRadio,
        Hovered::default(),
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer),
        TabIndex(0),
        ThemeFontColor(tokens::RADIO_TEXT),
        InheritableFont {
            font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
            font_size: 14.0,
        },
        overrides,
        Children::spawn((
            Spawn((
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: size::RADIO_SIZE,
                    height: size::RADIO_SIZE,
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                RadioOutline,
                BorderRadius::MAX,
                ThemeBorderColor(tokens::RADIO_BORDER),
                children![(
                    // Cheesy checkmark: rotated node with L-shaped border.
                    Node {
                        width: Val::Px(8.),
                        height: Val::Px(8.),
                        ..Default::default()
                    },
                    BorderRadius::MAX,
                    RadioMark,
                    ThemeBackgroundColor(tokens::RADIO_MARK),
                )],
            )),
            label,
        )),
    )
}

fn update_radio_styles(
    q_radioes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &ThemeFontColor,
        ),
        (
            With<CoreRadio>,
            Or<(Changed<Hovered>, Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<&ThemeBorderColor, With<RadioOutline>>,
    mut q_mark: Query<&ThemeBackgroundColor, With<RadioMark>>,
    mut commands: Commands,
) {
    for (radio_ent, disabled, checked, hovered, font_color) in q_radioes.iter() {
        let Some(outline_ent) = q_children
            .iter_descendants(radio_ent)
            .find(|en| q_outline.contains(*en))
        else {
            continue;
        };
        let Some(mark_ent) = q_children
            .iter_descendants(radio_ent)
            .find(|en| q_mark.contains(*en))
        else {
            continue;
        };
        let outline_border = q_outline.get_mut(outline_ent).unwrap();
        let mark_color = q_mark.get_mut(mark_ent).unwrap();
        set_radio_styles(
            radio_ent,
            outline_ent,
            mark_ent,
            disabled,
            checked,
            hovered.0,
            outline_border,
            mark_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_radio_styles_remove(
    q_radioes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &ThemeFontColor,
        ),
        With<CoreRadio>,
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<&ThemeBorderColor, With<RadioOutline>>,
    mut q_mark: Query<&ThemeBackgroundColor, With<RadioMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((radio_ent, disabled, checked, hovered, font_color)) = q_radioes.get(ent) {
                let Some(outline_ent) = q_children
                    .iter_descendants(radio_ent)
                    .find(|en| q_outline.contains(*en))
                else {
                    return;
                };
                let Some(mark_ent) = q_children
                    .iter_descendants(radio_ent)
                    .find(|en| q_mark.contains(*en))
                else {
                    return;
                };
                let outline_border = q_outline.get_mut(outline_ent).unwrap();
                let mark_color = q_mark.get_mut(mark_ent).unwrap();
                set_radio_styles(
                    radio_ent,
                    outline_ent,
                    mark_ent,
                    disabled,
                    checked,
                    hovered.0,
                    outline_border,
                    mark_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_radio_styles(
    radio_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    hovered: bool,
    outline_border: &ThemeBorderColor,
    mark_color: &ThemeBackgroundColor,
    font_color: &ThemeFontColor,
    commands: &mut Commands,
) {
    let outline_border_token = match (disabled, hovered) {
        (true, _) => tokens::RADIO_BORDER_DISABLED,
        (false, true) => tokens::RADIO_BORDER_HOVER,
        _ => tokens::RADIO_BORDER,
    };

    let mark_token = match disabled {
        true => tokens::RADIO_MARK_DISABLED,
        false => tokens::RADIO_MARK,
    };

    let font_color_token = match disabled {
        true => tokens::RADIO_TEXT_DISABLED,
        false => tokens::RADIO_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

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
        true => Visibility::Visible,
        false => Visibility::Hidden,
    });

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(radio_ent)
            .insert(ThemeFontColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(radio_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the radio styles.
pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_radio_styles, update_radio_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
