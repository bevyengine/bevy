//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{CoreButton, CoreWidgetsPlugin},
    ecs::system::SystemId,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex},
        InputDispatchPlugin,
    },
    picking::hover::IsHovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CoreWidgetsPlugin, InputDispatchPlugin))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_observer(on_add_pressed)
        .add_observer(on_remove_pressed)
        .add_observer(on_add_disabled)
        .add_observer(on_remove_disabled)
        .add_observer(on_change_hover)
        .add_systems(Update, toggle_disabled)
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

/// Marker which identifies buttons with a particular style, in this case the "Demo style".
#[derive(Component)]
struct DemoButton;

fn on_add_pressed(
    trigger: Trigger<OnAdd, Pressed>,
    mut buttons: Query<
        (
            &IsHovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((hovered, disabled, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target().unwrap())
    {
        let mut text = text_query.get_mut(children[0]).unwrap();
        set_button_style(
            disabled,
            hovered.get(),
            true,
            &mut color,
            &mut border_color,
            &mut text,
        );
    }
}

fn on_remove_pressed(
    trigger: Trigger<OnRemove, Pressed>,
    mut buttons: Query<
        (
            &IsHovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((hovered, disabled, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target().unwrap())
    {
        let mut text = text_query.get_mut(children[0]).unwrap();
        set_button_style(
            disabled,
            hovered.get(),
            false,
            &mut color,
            &mut border_color,
            &mut text,
        );
    }
}

fn on_add_disabled(
    trigger: Trigger<OnAdd, InteractionDisabled>,
    mut buttons: Query<
        (
            Has<Pressed>,
            &IsHovered,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((pressed, hovered, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target().unwrap())
    {
        let mut text = text_query.get_mut(children[0]).unwrap();
        set_button_style(
            true,
            hovered.get(),
            pressed,
            &mut color,
            &mut border_color,
            &mut text,
        );
    }
}

fn on_remove_disabled(
    trigger: Trigger<OnRemove, InteractionDisabled>,
    mut buttons: Query<
        (
            Has<Pressed>,
            &IsHovered,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((pressed, hovered, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target().unwrap())
    {
        let mut text = text_query.get_mut(children[0]).unwrap();
        set_button_style(
            false,
            hovered.get(),
            pressed,
            &mut color,
            &mut border_color,
            &mut text,
        );
    }
}

fn on_change_hover(
    trigger: Trigger<OnInsert, IsHovered>,
    mut buttons: Query<
        (
            Has<Pressed>,
            &IsHovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((pressed, hovered, disabled, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target().unwrap())
    {
        if children.is_empty() {
            return;
        }
        let Ok(mut text) = text_query.get_mut(children[0]) else {
            return;
        };
        set_button_style(
            disabled,
            hovered.get(),
            pressed,
            &mut color,
            &mut border_color,
            &mut text,
        );
    }
}

fn set_button_style(
    disabled: bool,
    hovered: bool,
    pressed: bool,
    color: &mut BackgroundColor,
    border_color: &mut BorderColor,
    text: &mut Text,
) {
    match (disabled, hovered, pressed) {
        // Disabled button
        (true, _, _) => {
            **text = "Disabled".to_string();
            *color = NORMAL_BUTTON.into();
            border_color.set_all(GRAY);
        }

        // Pressed and hovered button
        (false, true, true) => {
            **text = "Press".to_string();
            *color = PRESSED_BUTTON.into();
            border_color.set_all(RED);
        }

        // Hovered, unpressed button
        (false, true, false) => {
            **text = "Hover".to_string();
            *color = HOVERED_BUTTON.into();
            border_color.set_all(WHITE);
        }

        // Unhovered button (either pressed or not).
        (false, false, _) => {
            **text = "Button".to_string();
            *color = NORMAL_BUTTON.into();
            border_color.set_all(BLACK);
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let on_click = commands.register_system(|| {
        info!("Button clicked!");
    });
    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(button(&assets, on_click));
}

fn button(asset_server: &AssetServer, on_click: SystemId) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        TabGroup::default(),
        children![
            (
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                DemoButton,
                CoreButton {
                    on_click: Some(on_click),
                },
                IsHovered::default(),
                TabIndex(0),
                BorderColor::all(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Button"),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            Text::new("Press 'D' to toggle button disabled state"),
        ],
    )
}

fn toggle_disabled(
    input: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<(Entity, Has<InteractionDisabled>), With<CoreButton>>,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::KeyD) {
        for (entity, disabled) in &mut interaction_query {
            // disabled.0 = !disabled.0;
            if disabled {
                info!("Button enabled");
                commands.entity(entity).remove::<InteractionDisabled>();
            } else {
                info!("Button disabled");
                commands.entity(entity).insert(InteractionDisabled);
            }
        }
    }
}
