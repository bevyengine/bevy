//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{
        Callback, CoreButton, CoreCheckbox, CoreSlider, CoreSliderThumb, CoreWidgetsPlugins,
        SliderRange, SliderValue,
    },
    ecs::system::SystemId,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
        InputDispatchPlugin,
    },
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled, Pressed},
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CoreWidgetsPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(DemoWidgetStates { slider_value: 50.0 })
        .add_systems(Startup, setup)
        .add_observer(button_on_add_pressed)
        .add_observer(button_on_remove_pressed)
        .add_observer(button_on_add_disabled)
        .add_observer(button_on_remove_disabled)
        .add_observer(button_on_change_hover)
        .add_observer(slider_on_add_disabled)
        .add_observer(slider_on_remove_disabled)
        .add_observer(slider_on_change_hover)
        .add_observer(slider_on_change_value)
        .add_observer(slider_on_change_range)
        .add_observer(checkbox_on_add_disabled)
        .add_observer(checkbox_on_remove_disabled)
        .add_observer(checkbox_on_change_hover)
        .add_observer(checkbox_on_add_checked)
        .add_observer(checkbox_on_remove_checked)
        .add_systems(Update, (update_widget_values, toggle_disabled))
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);
const CHECKBOX_OUTLINE: Color = Color::srgb(0.45, 0.45, 0.45);
const CHECKBOX_CHECK: Color = Color::srgb(0.35, 0.75, 0.35);

/// Marker which identifies buttons with a particular style, in this case the "Demo style".
#[derive(Component)]
struct DemoButton;

/// Marker which identifies sliders with a particular style.
#[derive(Component, Default)]
struct DemoSlider;

/// Marker which identifies the slider's thumb element.
#[derive(Component, Default)]
struct DemoSliderThumb;

/// Marker which identifies checkboxes with a particular style.
#[derive(Component, Default)]
struct DemoCheckbox;

/// A struct to hold the state of various widgets shown in the demo.
///
/// While it is possible to use the widget's own state components as the source of truth,
/// in many cases widgets will be used to display dynamic data coming from deeper within the app,
/// using some kind of data-binding. This example shows how to maintain an external source of
/// truth for widget states.
#[derive(Resource)]
struct DemoWidgetStates {
    slider_value: f32,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // System to print a value when the button is clicked.
    let on_click = commands.register_system(|| {
        info!("Button clicked!");
    });

    // System to update a resource when the slider value changes. Note that we could have
    // updated the slider value directly, but we want to demonstrate externalizing the state.
    let on_change_value = commands.register_system(
        |value: In<f32>, mut widget_states: ResMut<DemoWidgetStates>| {
            widget_states.slider_value = *value;
        },
    );

    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(&assets, on_click, on_change_value));
}

fn demo_root(
    asset_server: &AssetServer,
    on_click: SystemId,
    on_change_value: SystemId<In<f32>>,
) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        TabGroup::default(),
        children![
            button(asset_server, Callback::System(on_click)),
            slider(0.0, 100.0, 50.0, Callback::System(on_change_value)),
            checkbox(asset_server, "Checkbox", Callback::Ignore),
            Text::new("Press 'D' to toggle widget disabled states"),
        ],
    )
}

fn button(asset_server: &AssetServer, on_click: Callback) -> impl Bundle {
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
            on_activate: on_click,
        },
        Hovered::default(),
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
        )],
    )
}

fn button_on_add_pressed(
    trigger: On<Add, Pressed>,
    mut buttons: Query<
        (
            &Hovered,
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
        buttons.get_mut(trigger.target())
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

fn button_on_remove_pressed(
    trigger: On<Remove, Pressed>,
    mut buttons: Query<
        (
            &Hovered,
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
        buttons.get_mut(trigger.target())
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

fn button_on_add_disabled(
    trigger: On<Add, InteractionDisabled>,
    mut buttons: Query<
        (
            Has<Pressed>,
            &Hovered,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((pressed, hovered, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target())
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

fn button_on_remove_disabled(
    trigger: On<Remove, InteractionDisabled>,
    mut buttons: Query<
        (
            Has<Pressed>,
            &Hovered,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((pressed, hovered, mut color, mut border_color, children)) =
        buttons.get_mut(trigger.target())
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

fn button_on_change_hover(
    trigger: On<Insert, Hovered>,
    mut buttons: Query<
        (
            Has<Pressed>,
            &Hovered,
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
        buttons.get_mut(trigger.target())
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

/// Create a demo slider
fn slider(min: f32, max: f32, value: f32, on_change: Callback<In<f32>>) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            justify_items: JustifyItems::Center,
            column_gap: Val::Px(4.0),
            height: Val::Px(12.0),
            width: Val::Percent(30.0),
            ..default()
        },
        Name::new("Slider"),
        Hovered::default(),
        DemoSlider,
        CoreSlider {
            on_change,
            ..default()
        },
        SliderValue(value),
        SliderRange::new(min, max),
        TabIndex(0),
        Children::spawn((
            // Slider background rail
            Spawn((
                Node {
                    height: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK), // Border color for the checkbox
                BorderRadius::all(Val::Px(3.0)),
            )),
            // Invisible track to allow absolute placement of thumb entity. This is narrower than
            // the actual slider, which allows us to position the thumb entity using simple
            // percentages, without having to measure the actual width of the slider thumb.
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    // Track is short by 12px to accommodate the thumb.
                    right: Val::Px(12.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                children![(
                    // Thumb
                    DemoSliderThumb,
                    CoreSliderThumb,
                    Node {
                        display: Display::Flex,
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(0.0), // This will be updated by the slider's value
                        ..default()
                    },
                    BorderRadius::MAX,
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

fn slider_on_add_disabled(
    trigger: On<Add, InteractionDisabled>,
    sliders: Query<(Entity, &Hovered), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, hovered)) = sliders.get(trigger.target()) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_bg.0 = thumb_color(true, hovered.0);
                }
            }
        }
    }
}

fn slider_on_remove_disabled(
    trigger: On<Remove, InteractionDisabled>,
    sliders: Query<(Entity, &Hovered), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, hovered)) = sliders.get(trigger.target()) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_bg.0 = thumb_color(false, hovered.0);
                }
            }
        }
    }
}

fn slider_on_change_hover(
    trigger: On<Insert, Hovered>,
    sliders: Query<(Entity, &Hovered, Has<InteractionDisabled>), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, hovered, disabled)) = sliders.get(trigger.target()) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_bg.0 = thumb_color(disabled, hovered.0);
                }
            }
        }
    }
}

fn slider_on_change_value(
    trigger: On<Insert, SliderValue>,
    sliders: Query<(Entity, &SliderValue, &SliderRange), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, value, range)) = sliders.get(trigger.target()) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_node.left = Val::Percent(range.thumb_position(value.0) * 100.0);
                }
            }
        }
    }
}

fn slider_on_change_range(
    trigger: On<Insert, SliderRange>,
    sliders: Query<(Entity, &SliderValue, &SliderRange), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, value, range)) = sliders.get(trigger.target()) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_node.left = Val::Percent(range.thumb_position(value.0) * 100.0);
                }
            }
        }
    }
}

fn thumb_color(disabled: bool, hovered: bool) -> Color {
    match (disabled, hovered) {
        (true, _) => GRAY.into(),

        (false, true) => SLIDER_THUMB.lighter(0.3),

        _ => SLIDER_THUMB,
    }
}

/// Create a demo checkbox
fn checkbox(
    asset_server: &AssetServer,
    caption: &str,
    on_change: Callback<In<bool>>,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            align_content: AlignContent::Center,
            column_gap: Val::Px(4.0),
            ..default()
        },
        Name::new("Checkbox"),
        Hovered::default(),
        DemoCheckbox,
        CoreCheckbox { on_change },
        TabIndex(0),
        Children::spawn((
            Spawn((
                // Checkbox outer
                Node {
                    display: Display::Flex,
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(CHECKBOX_OUTLINE), // Border color for the checkbox
                BorderRadius::all(Val::Px(3.0)),
                children![
                    // Checkbox inner
                    (
                        Node {
                            display: Display::Flex,
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            top: Val::Px(2.0),
                            ..default()
                        },
                        BackgroundColor(Srgba::NONE.into()),
                    ),
                ],
            )),
            Spawn((
                Text::new(caption),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
                    ..default()
                },
            )),
        )),
    )
}

fn checkbox_on_add_disabled(
    trigger: On<Add, InteractionDisabled>,
    checkboxes: Query<(&Hovered, Has<Checked>, &Children), With<DemoCheckbox>>,
    mut borders: Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    mut marks: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    if let Ok((hovered, checked, children)) = checkboxes.get(trigger.target()) {
        set_checkbox_style(children, &mut borders, &mut marks, true, hovered.0, checked);
    }
}

fn checkbox_on_remove_disabled(
    trigger: On<Remove, InteractionDisabled>,
    checkboxes: Query<(&Hovered, Has<Checked>, &Children), With<DemoCheckbox>>,
    mut borders: Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    mut marks: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    if let Ok((hovered, checked, children)) = checkboxes.get(trigger.target()) {
        set_checkbox_style(
            children,
            &mut borders,
            &mut marks,
            false,
            hovered.0,
            checked,
        );
    }
}

fn checkbox_on_change_hover(
    trigger: On<Insert, Hovered>,
    checkboxes: Query<
        (&Hovered, Has<InteractionDisabled>, Has<Checked>, &Children),
        With<DemoCheckbox>,
    >,
    mut borders: Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    mut marks: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    if let Ok((hovered, disabled, checked, children)) = checkboxes.get(trigger.target()) {
        set_checkbox_style(
            children,
            &mut borders,
            &mut marks,
            disabled,
            hovered.0,
            checked,
        );
    }
}

fn checkbox_on_add_checked(
    trigger: On<Add, Checked>,
    checkboxes: Query<
        (&Hovered, Has<InteractionDisabled>, Has<Checked>, &Children),
        With<DemoCheckbox>,
    >,
    mut borders: Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    mut marks: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    if let Ok((hovered, disabled, checked, children)) = checkboxes.get(trigger.target()) {
        set_checkbox_style(
            children,
            &mut borders,
            &mut marks,
            disabled,
            hovered.0,
            checked,
        );
    }
}

fn checkbox_on_remove_checked(
    trigger: On<Remove, Checked>,
    checkboxes: Query<(&Hovered, Has<InteractionDisabled>, &Children), With<DemoCheckbox>>,
    mut borders: Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    mut marks: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    if let Ok((hovered, disabled, children)) = checkboxes.get(trigger.target()) {
        set_checkbox_style(
            children,
            &mut borders,
            &mut marks,
            disabled,
            hovered.0,
            false,
        );
    }
}

fn set_checkbox_style(
    children: &Children,
    borders: &mut Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    marks: &mut Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
    disabled: bool,
    hovering: bool,
    checked: bool,
) {
    let Some(border_id) = children.first() else {
        return;
    };

    let Ok((mut border_color, border_children)) = borders.get_mut(*border_id) else {
        return;
    };

    let Some(mark_id) = border_children.first() else {
        warn!("Checkbox does not have a mark entity.");
        return;
    };

    let Ok(mut mark_bg) = marks.get_mut(*mark_id) else {
        warn!("Checkbox mark entity lacking a background color.");
        return;
    };

    let color: Color = if disabled {
        // If the checkbox is disabled, use a lighter color
        CHECKBOX_OUTLINE.with_alpha(0.2)
    } else if hovering {
        // If hovering, use a lighter color
        CHECKBOX_OUTLINE.lighter(0.2)
    } else {
        // Default color for the checkbox
        CHECKBOX_OUTLINE
    };

    // Update the background color of the check mark
    border_color.set_all(color);

    let mark_color: Color = match (disabled, checked) {
        (true, true) => CHECKBOX_CHECK.with_alpha(0.5),
        (false, true) => CHECKBOX_CHECK,
        (_, false) => Srgba::NONE.into(),
    };

    if mark_bg.0 != mark_color {
        // Update the color of the check mark
        mark_bg.0 = mark_color;
    }
}

/// Update the widget states based on the changing resource.
fn update_widget_values(
    res: Res<DemoWidgetStates>,
    mut sliders: Query<Entity, With<DemoSlider>>,
    mut commands: Commands,
) {
    if res.is_changed() {
        for slider_ent in sliders.iter_mut() {
            commands
                .entity(slider_ent)
                .insert(SliderValue(res.slider_value));
        }
    }
}

fn toggle_disabled(
    input: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<
        (Entity, Has<InteractionDisabled>),
        Or<(With<CoreButton>, With<CoreSlider>, With<CoreCheckbox>)>,
    >,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::KeyD) {
        for (entity, disabled) in &mut interaction_query {
            if disabled {
                info!("Widget enabled");
                commands.entity(entity).remove::<InteractionDisabled>();
            } else {
                info!("Widget disabled");
                commands.entity(entity).insert(InteractionDisabled);
            }
        }
    }
}
