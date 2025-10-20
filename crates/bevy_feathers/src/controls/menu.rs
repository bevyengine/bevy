use alloc::sync::Arc;

use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    schedule::IntoScheduleConfigs,
    system::{Commands, EntityCommands, Query},
};
use bevy_log::info;
use bevy_picking::{
    events::{Click, Pointer},
    hover::Hovered,
    PickingSystems,
};
use bevy_scene2::{prelude::*, template_value};
use bevy_ui::{
    AlignItems, BoxShadow, Display, FlexDirection, GlobalZIndex, InteractionDisabled,
    JustifyContent, Node, OverrideClip, PositionType, Pressed, UiRect, Val,
};
use bevy_ui_widgets::{
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    MenuAction, MenuEvent, MenuItem, MenuPopup,
};

use crate::{
    constants::{fonts, icons, size},
    controls::{button, ButtonProps, ButtonVariant},
    font_styles::InheritableFont,
    icon,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};
use bevy_input_focus::tab_navigation::TabIndex;

/// Parameters for the menu button template, passed to [`menu_button`] function.
#[derive(Default)]
pub struct MenuButtonProps {
    /// Rounded corners options
    pub corners: RoundedCorners,
}

/// Marker for menu items
#[derive(Component, Default, Clone)]
struct MenuItemStyle;

/// Marker for menu popup
#[derive(Component, Default, Clone)]
struct MenuPopupStyle;

/// Marker for menu wrapper
#[derive(Component, Clone, Default)]
struct Menu(Option<Arc<dyn Fn(EntityCommands) + 'static + Send + Sync>>);

/// Menu scene function. This wraps the menu button and provides an anchor for the popopver.
pub fn menu<F: Fn(EntityCommands) + 'static + Send + Sync>(spawn_popover: F) -> impl Scene {
    let menu = Menu(Some(Arc::new(spawn_popover)));
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Stretch,
            align_items: AlignItems::Stretch,
        }
        template_value(menu)
        on(|
            ev: On<MenuEvent>,
            q_menu: Query<(&Menu, &Children)>,
            q_popovers: Query<Entity, With<MenuPopupStyle>>,
            // mut redraw_events: MessageWriter<RequestRedraw>,
            mut commands: Commands| {
            match ev.event().action {
                // MenuEvent::Open => todo!(),
                // MenuEvent::Close => todo!(),
                MenuAction::Toggle => {
                    let mut was_open = false;
                    let Ok((menu, children)) = q_menu.get(ev.source) else {
                        return;
                    };
                    for child in children.iter() {
                        if q_popovers.contains(*child) {
                            commands.entity(*child).despawn();
                            was_open = true;
                        }
                    }
                    // Spawn the menu if not already open.
                    if !was_open {
                        info!("Opening, !was_open");
                        if let Some(factory) = menu.0.as_ref() {
                            (*factory)(commands.entity(ev.source));
                            // redraw_events.write(RequestRedraw);
                        }
                    }
                },
                MenuAction::CloseAll => {
                    let Ok((_menu, children)) = q_menu.get(ev.source) else {
                        return;
                    };
                    for child in children.iter() {
                        if q_popovers.contains(*child) {
                            commands.entity(*child).despawn();
                        }
                    }
                },
                // MenuEvent::FocusRoot => todo!(),
                event => {
                    info!("Menu Event: {:?}", event);
                }
            }
        })
    }
}

/// Button scene function.
///
/// # Arguments
/// * `props` - construction properties for the button.
pub fn menu_button(props: MenuButtonProps) -> impl Scene {
    bsn! {
        :button(ButtonProps {
            variant: ButtonVariant::Normal,
            corners: props.corners,
        })
        Node {
            // TODO: HACK to deal with lack of intercepted children
            flex_direction: FlexDirection::RowReverse,
        }
        on(|ev: On<Pointer<Click>>, mut commands: Commands| {
            commands.trigger(MenuEvent { source: ev.entity, action: MenuAction::Toggle });
        })
        [
            :icon(icons::CHEVRON_DOWN),
            Node {
                flex_grow: 0.2,
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
            padding: UiRect::all(Val::Px(4.0)),
        }
        MenuPopupStyle
        MenuPopup
        template_value(Visibility::Hidden)
        template_value(RoundedCorners::All.to_border_radius(4.0))
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

/// Menu item scene function
pub fn menu_item() -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            min_width: size::ROW_HEIGHT,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
        }
        MenuItemStyle
        MenuItem
        Hovered
        // TODO: port CursonIcon to GetTemplate
        // CursorIcon::System(bevy_window::SystemCursorIcon::Pointer)
        TabIndex(0)
        ThemeBackgroundColor(tokens::MENU_BG) // Same as menu
        ThemeFontColor(tokens::MENUITEM_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
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
            With<MenuItemStyle>,
            Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (button_ent, disabled, pressed, hovered, bg_color, font_color) in q_menuitems.iter() {
        set_menuitem_colors(
            button_ent,
            disabled,
            pressed,
            hovered.0,
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
        With<MenuItemStyle>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .for_each(|ent| {
            if let Ok((button_ent, disabled, pressed, hovered, bg_color, font_color)) =
                q_menuitems.get(ent)
            {
                set_menuitem_colors(
                    button_ent,
                    disabled,
                    pressed,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_menuitem_colors(
    button_ent: Entity,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &ThemeFontColor,
    commands: &mut Commands,
) {
    let bg_token = match (pressed, hovered) {
        (true, _) => tokens::MENUITEM_BG_PRESSED,
        (false, true) => tokens::MENUITEM_BG_HOVER,
        (false, false) => tokens::MENU_BG,
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

/// Plugin which registers the systems for updating the menu and menu button styles.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_menuitem_styles, update_menuitem_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
