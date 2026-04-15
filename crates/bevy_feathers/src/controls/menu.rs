use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_log::warn;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_scene::{prelude::*, template_value};
use bevy_text::{FontSize, FontWeight};
use bevy_ui::{
    px, AlignItems, AlignSelf, BoxShadow, Display, FlexDirection, GlobalZIndex,
    InteractionDisabled, JustifyContent, Node, OverrideClip, PositionType, Pressed, UiRect, Val,
};
use bevy_ui_widgets::{
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    ActivateOnPress, MenuAction, MenuButton, MenuEvent, MenuFocusState, MenuItem, MenuPopup,
};

use crate::{
    constants::{fonts, icons, size},
    controls::{button, ButtonProps, ButtonVariant},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    icon,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};
use bevy_input_focus::{
    tab_navigation::{NavAction, TabIndex},
    InputFocus, InputFocusVisible,
};

/// Parameters for the menu button template, passed to [`menu_button`] function.
pub struct MenuButtonProps {
    /// Label for this menu button
    pub caption: Box<dyn SceneList>,
    /// Rounded corners options
    pub corners: RoundedCorners,
    /// Include the standard downward-pointing chevron (default true).
    pub arrow: bool,
}

impl Default for MenuButtonProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
            corners: Default::default(),
            arrow: true,
        }
    }
}

/// Marker for menu button
#[derive(Component, Default, Clone)]
struct FeathersMenuButton;

/// Marker for menu items
#[derive(Component, Default, Clone)]
struct FeathersMenuItem;

/// Marker for menu popup
#[derive(Component, Default, Clone)]
struct FeathersMenuPopup;

/// Component that contains the popup content generator.
#[derive(Component, Clone, Default)]
struct FeathersMenuContainer;

/// Menu scene function. This wraps the menu button and provides an anchor for the popover.
pub fn menu() -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Stretch,
            align_items: AlignItems::Stretch,
        }
        FeathersMenuContainer
        on(on_menu_event)
    }
}

fn on_menu_event(
    mut ev: On<MenuEvent>,
    q_menu_children: Query<&Children>,
    q_popovers: Query<&mut Visibility, With<FeathersMenuPopup>>,
    q_buttons: Query<(), With<FeathersMenuButton>>,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
) {
    match ev.event().action {
        MenuAction::Open(nav) => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            ev.propagate(false);
            for child in children.iter() {
                if q_popovers.contains(*child) {
                    commands
                        .entity(*child)
                        .insert((Visibility::Visible, MenuFocusState::Opening(nav)));
                    return;
                }
            }
            warn!("Menu popup not found");
        }
        MenuAction::Toggle => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            for child in children.iter() {
                if let Ok(visibility) = q_popovers.get(*child) {
                    ev.propagate(false);
                    if visibility == Visibility::Visible {
                        commands.entity(*child).insert(Visibility::Hidden);
                    } else {
                        commands.entity(*child).insert((
                            Visibility::Visible,
                            MenuFocusState::Opening(NavAction::First),
                        ));
                    }
                    return;
                }
            }
            warn!("Menu popup not found");
        }
        MenuAction::CloseAll => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            for child in children.iter() {
                if q_popovers.contains(*child) {
                    ev.propagate(false);
                    commands.entity(*child).insert(Visibility::Hidden);
                }
            }
        }
        MenuAction::FocusRoot => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            for child in children.iter() {
                if q_buttons.contains(*child) {
                    ev.propagate(false);
                    focus.set(*child);
                    break;
                }
            }
        }
    }
}

/// Menu button scene function. This produces a button that has a dropdown arrow.
///
/// # Arguments
/// * `props` - construction properties for the button.
pub fn menu_button(props: MenuButtonProps) -> impl Scene {
    bsn! {
        :button(ButtonProps {
            caption: props.caption,
            variant: ButtonVariant::Normal,
            corners: props.corners,
        })
        ActivateOnPress
        MenuButton
        FeathersMenuButton
        // Additional children for menu chevron
        Children [
            {
                if props.arrow {
                    Box::new(bsn_list!(
                        Node {
                            flex_grow: 1.0,
                        },
                        :icon(icons::CHEVRON_DOWN),
                    )) as Box<dyn SceneList>
                } else {
                    Box::new(bsn_list!()) as Box<dyn SceneList>
                }
            }
        ]
    }
}

/// Menu Popup scene function
pub fn menu_popup() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Stretch,
            align_items: AlignItems::Stretch,
            border: UiRect::all(Val::Px(1.0)),
            padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
            border_radius: {RoundedCorners::All.to_border_radius(4.0)},
        }
        FeathersMenuPopup
        MenuPopup
        template_value(Visibility::Hidden)
        ThemeBackgroundColor(tokens::MENU_BG)
        ThemeBorderColor(tokens::MENU_BORDER)
        BoxShadow::new(
            Srgba::BLACK.with_alpha(0.9).into(),
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(1.0),
            Val::Px(4.0),
        )
        GlobalZIndex(100)
        template_value(
            Popover {
                positions: vec![
                    PopoverPlacement {
                        side: PopoverSide::Bottom,
                        align: PopoverAlign::Start,
                        gap: 2.0,
                    },
                    PopoverPlacement {
                        side: PopoverSide::Top,
                        align: PopoverAlign::Start,
                        gap: 2.0,
                    },
                ],
                window_margin: 10.0,
            }
        )
        OverrideClip
    }
}

/// Parameters for the menu button template, passed to [`menu_button`] function.
pub struct MenuItemProps {
    /// Label for this menu item
    pub caption: Box<dyn SceneList>,
}

impl Default for MenuItemProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
        }
    }
}

/// Menu item scene function
pub fn menu_item(props: MenuItemProps) -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            min_width: size::ROW_HEIGHT,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
        }
        FeathersMenuItem
        MenuItem
        Hovered
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
        TabIndex(0)
        ThemeBackgroundColor(tokens::MENU_BG) // Same as menu
        ThemeFontColor(tokens::MENUITEM_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: FontSize::Px(14.0),
            weight: FontWeight::NORMAL,
        }
        Children [
            {props.caption}
        ]
    }
}

fn update_menuitem_styles(
    q_menuitems: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeFontColor,
        ),
        (
            With<FeathersMenuItem>,
            Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
        ),
    >,
    mut commands: Commands,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
) {
    for (item_ent, disabled, pressed, hovered, bg_color, font_color) in q_menuitems.iter() {
        set_menuitem_colors(
            item_ent,
            disabled,
            pressed,
            hovered.0,
            Some(item_ent) == focus.get() && focus_visible.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_menuitem_styles_remove(
    q_menuitems: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeFontColor,
        ),
        With<FeathersMenuItem>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .for_each(|ent| {
            if let Ok((item_ent, disabled, pressed, hovered, bg_color, font_color)) =
                q_menuitems.get(ent)
            {
                set_menuitem_colors(
                    item_ent,
                    disabled,
                    pressed,
                    hovered.0,
                    Some(item_ent) == focus.get() && focus_visible.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn update_menuitem_styles_focus_changed(
    q_menuitems: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeFontColor,
        ),
        With<FeathersMenuItem>,
    >,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut commands: Commands,
) {
    if focus.is_changed() || focus_visible.is_changed() {
        for (item_ent, disabled, pressed, hovered, bg_color, font_color) in q_menuitems.iter() {
            set_menuitem_colors(
                item_ent,
                disabled,
                pressed,
                hovered.0,
                Some(item_ent) == focus.get() && focus_visible.0,
                bg_color,
                font_color,
                &mut commands,
            );
        }
    }
}

fn set_menuitem_colors(
    button_ent: Entity,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    focused: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &ThemeFontColor,
    commands: &mut Commands,
) {
    let bg_token = match (focused, pressed, hovered) {
        (true, _, _) => tokens::MENUITEM_BG_FOCUSED,
        (false, true, _) => tokens::MENUITEM_BG_PRESSED,
        (false, false, true) => tokens::MENUITEM_BG_HOVER,
        (false, false, false) => tokens::MENU_BG,
    };

    let font_color_token = match disabled {
        true => tokens::MENUITEM_TEXT_DISABLED,
        false => tokens::MENUITEM_TEXT,
    };

    // Change background color
    if bg_color.0 != bg_token {
        commands
            .entity(button_ent)
            .insert(ThemeBackgroundColor(bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(button_ent)
            .insert(ThemeFontColor(font_color_token));
    }
}

/// A decorative divider between menu items
pub fn menu_divider() -> impl Scene {
    bsn! {
        Node {
            height: px(1),
            justify_content: JustifyContent::Start,
            align_self: AlignSelf::Stretch,
            margin: UiRect::axes(Val::Px(0.0), Val::Px(2.)),
        }
        ThemeBackgroundColor(tokens::MENU_BORDER) // Same as menu
    }
}

/// Plugin which registers the systems for updating the menu and menu button styles.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_menuitem_styles,
                update_menuitem_styles_remove,
                update_menuitem_styles_focus_changed,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
