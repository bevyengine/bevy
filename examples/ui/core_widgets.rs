//! This example illustrates how to create widgets using the `bevy_core_widgets` widget set.

use bevy::{
    color::palettes::basic::*,
    core_widgets::{
        Activate, Callback, CoreButton, CoreCheckbox, CoreRadio, CoreRadioGroup, CoreSlider,
        CoreSliderDragState, CoreSliderThumb, CoreWidgetsPlugins, SliderRange, SliderValue,
        TrackClick, ValueChange,
    },
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
        .insert_resource(DemoWidgetStates {
            slider_value: 50.0,
            slider_click: TrackClick::Snap,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_widget_values,
                update_button_style,
                update_button_style2,
                update_slider_style.after(update_widget_values),
                update_slider_style2.after(update_widget_values),
                update_checkbox_or_radio_style.after(update_widget_values),
                update_checkbox_or_radio_style2.after(update_widget_values),
                toggle_disabled,
            ),
        )
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);
const ELEMENT_OUTLINE: Color = Color::srgb(0.45, 0.45, 0.45);
const ELEMENT_FILL: Color = Color::srgb(0.35, 0.75, 0.35);
const ELEMENT_FILL_DISABLED: Color = Color::srgb(0.5019608, 0.5019608, 0.5019608);

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

/// Marker which identifies a styled radio button. We'll use this to change the track click
/// behavior.
#[derive(Component, Default)]
struct DemoRadio(TrackClick);

/// A struct to hold the state of various widgets shown in the demo.
///
/// While it is possible to use the widget's own state components as the source of truth,
/// in many cases widgets will be used to display dynamic data coming from deeper within the app,
/// using some kind of data-binding. This example shows how to maintain an external source of
/// truth for widget states.
#[derive(Resource)]
struct DemoWidgetStates {
    slider_value: f32,
    slider_click: TrackClick,
}

/// Update the widget states based on the changing resource.
fn update_widget_values(
    res: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &mut CoreSlider), With<DemoSlider>>,
    radios: Query<(Entity, &DemoRadio, Has<Checked>)>,
    mut commands: Commands,
) {
    if res.is_changed() {
        for (slider_ent, mut slider) in sliders.iter_mut() {
            commands
                .entity(slider_ent)
                .insert(SliderValue(res.slider_value));
            slider.track_click = res.slider_click;
        }

        for (radio_id, radio_value, checked) in radios.iter() {
            let will_be_checked = radio_value.0 == res.slider_click;
            if will_be_checked != checked {
                if will_be_checked {
                    commands.entity(radio_id).insert(Checked);
                } else {
                    commands.entity(radio_id).remove::<Checked>();
                }
            }
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // System to print a value when the button is clicked.
    let on_click = commands.register_system(|_: In<Activate>| {
        info!("Button clicked!");
    });

    // System to update a resource when the slider value changes. Note that we could have
    // updated the slider value directly, but we want to demonstrate externalizing the state.
    let on_change_value = commands.register_system(
        |value: In<ValueChange<f32>>, mut widget_states: ResMut<DemoWidgetStates>| {
            widget_states.slider_value = value.0.value;
        },
    );

    // System to update a resource when the radio group changes.
    let on_change_radio = commands.register_system(
        |value: In<Activate>,
         mut widget_states: ResMut<DemoWidgetStates>,
         q_radios: Query<&DemoRadio>| {
            if let Ok(radio) = q_radios.get(value.0 .0) {
                widget_states.slider_click = radio.0;
            }
        },
    );

    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(
        &assets,
        Callback::System(on_click),
        Callback::System(on_change_value),
        Callback::System(on_change_radio),
    ));
}

fn demo_root(
    asset_server: &AssetServer,
    on_click: Callback<In<Activate>>,
    on_change_value: Callback<In<ValueChange<f32>>>,
    on_change_radio: Callback<In<Activate>>,
) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            ..default()
        },
        TabGroup::default(),
        children![
            button(asset_server, on_click),
            slider(0.0, 100.0, 50.0, on_change_value),
            checkbox(asset_server, "Checkbox", Callback::Ignore),
            radio_group(asset_server, on_change_radio),
            Text::new("Press 'D' to toggle widget disabled states"),
        ],
    )
}

fn button(asset_server: &AssetServer, on_click: Callback<In<Activate>>) -> impl Bundle {
    (
        Node {
            width: px(150),
            height: px(65),
            border: UiRect::all(px(5)),
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

fn update_button_style(
    mut buttons: Query<
        (
            Has<Pressed>,
            &Hovered,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (
            Or<(
                Changed<Pressed>,
                Changed<Hovered>,
                Added<InteractionDisabled>,
            )>,
            With<DemoButton>,
        ),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (pressed, hovered, disabled, mut color, mut border_color, children) in &mut buttons {
        let mut text = text_query.get_mut(children[0]).unwrap();
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

/// Supplementary system to detect removed marker components
fn update_button_style2(
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
    mut removed_depressed: RemovedComponents<Pressed>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut text_query: Query<&mut Text>,
) {
    removed_depressed
        .read()
        .chain(removed_disabled.read())
        .for_each(|entity| {
            if let Ok((pressed, hovered, disabled, mut color, mut border_color, children)) =
                buttons.get_mut(entity)
            {
                let mut text = text_query.get_mut(children[0]).unwrap();
                set_button_style(
                    disabled,
                    hovered.get(),
                    pressed,
                    &mut color,
                    &mut border_color,
                    &mut text,
                );
            }
        });
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
fn slider(
    min: f32,
    max: f32,
    value: f32,
    on_change: Callback<In<ValueChange<f32>>>,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            justify_items: JustifyItems::Center,
            column_gap: px(4),
            height: px(12),
            width: percent(30),
            ..default()
        },
        Name::new("Slider"),
        Hovered::default(),
        DemoSlider,
        CoreSlider {
            on_change,
            track_click: TrackClick::Snap,
        },
        SliderValue(value),
        SliderRange::new(min, max),
        TabIndex(0),
        Children::spawn((
            // Slider background rail
            Spawn((
                Node {
                    height: px(6),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK), // Border color for the slider
                BorderRadius::all(px(3)),
            )),
            // Invisible track to allow absolute placement of thumb entity. This is narrower than
            // the actual slider, which allows us to position the thumb entity using simple
            // percentages, without having to measure the actual width of the slider thumb.
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: px(0),
                    // Track is short by 12px to accommodate the thumb.
                    right: px(12),
                    top: px(0),
                    bottom: px(0),
                    ..default()
                },
                children![(
                    // Thumb
                    DemoSliderThumb,
                    CoreSliderThumb,
                    Node {
                        display: Display::Flex,
                        width: px(12),
                        height: px(12),
                        position_type: PositionType::Absolute,
                        left: percent(0), // This will be updated by the slider's value
                        ..default()
                    },
                    BorderRadius::MAX,
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

/// Update the visuals of the slider based on the slider state.
fn update_slider_style(
    sliders: Query<
        (
            Entity,
            &SliderValue,
            &SliderRange,
            &Hovered,
            &CoreSliderDragState,
            Has<InteractionDisabled>,
        ),
        (
            Or<(
                Changed<SliderValue>,
                Changed<SliderRange>,
                Changed<Hovered>,
                Changed<CoreSliderDragState>,
                Added<InteractionDisabled>,
            )>,
            With<DemoSlider>,
        ),
    >,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, &mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    for (slider_ent, value, range, hovered, drag_state, disabled) in sliders.iter() {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, mut thumb_bg, is_thumb)) = thumbs.get_mut(child)
                && is_thumb
            {
                thumb_node.left = percent(range.thumb_position(value.0) * 100.0);
                thumb_bg.0 = thumb_color(disabled, hovered.0 | drag_state.dragging);
            }
        }
    }
}

fn update_slider_style2(
    sliders: Query<
        (
            Entity,
            &Hovered,
            &CoreSliderDragState,
            Has<InteractionDisabled>,
        ),
        With<DemoSlider>,
    >,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
) {
    removed_disabled.read().for_each(|entity| {
        if let Ok((slider_ent, hovered, drag_state, disabled)) = sliders.get(entity) {
            for child in children.iter_descendants(slider_ent) {
                if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child)
                    && is_thumb
                {
                    thumb_bg.0 = thumb_color(disabled, hovered.0 | drag_state.dragging);
                }
            }
        }
    });
}

fn thumb_color(disabled: bool, hovered: bool) -> Color {
    match (disabled, hovered) {
        (true, _) => ELEMENT_FILL_DISABLED,

        (false, true) => SLIDER_THUMB.lighter(0.3),

        _ => SLIDER_THUMB,
    }
}

/// Create a demo checkbox
fn checkbox(
    asset_server: &AssetServer,
    caption: &str,
    on_change: Callback<In<ValueChange<bool>>>,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            align_content: AlignContent::Center,
            column_gap: px(4),
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
                    width: px(16),
                    height: px(16),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BorderColor::all(ELEMENT_OUTLINE), // Border color for the checkbox
                BorderRadius::all(px(3)),
                children![
                    // Checkbox inner
                    (
                        Node {
                            display: Display::Flex,
                            width: px(8),
                            height: px(8),
                            position_type: PositionType::Absolute,
                            left: px(2),
                            top: px(2),
                            ..default()
                        },
                        BackgroundColor(ELEMENT_FILL),
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

// Update the element's styles.
fn update_checkbox_or_radio_style(
    mut q_checkbox: Query<
        (Has<Checked>, &Hovered, Has<InteractionDisabled>, &Children),
        (
            Or<(With<DemoCheckbox>, With<DemoRadio>)>,
            Or<(
                Added<DemoCheckbox>,
                Changed<Hovered>,
                Added<Checked>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    mut q_border_color: Query<
        (&mut BorderColor, &mut Children),
        (Without<DemoCheckbox>, Without<DemoRadio>),
    >,
    mut q_bg_color: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    for (checked, Hovered(is_hovering), is_disabled, children) in q_checkbox.iter_mut() {
        let Some(border_id) = children.first() else {
            continue;
        };

        let Ok((mut border_color, border_children)) = q_border_color.get_mut(*border_id) else {
            continue;
        };

        let Some(mark_id) = border_children.first() else {
            warn!("Checkbox does not have a mark entity.");
            continue;
        };

        let Ok(mut mark_bg) = q_bg_color.get_mut(*mark_id) else {
            warn!("Checkbox mark entity lacking a background color.");
            continue;
        };

        set_checkbox_or_radio_style(
            is_disabled,
            *is_hovering,
            checked,
            &mut border_color,
            &mut mark_bg,
        );
    }
}

fn update_checkbox_or_radio_style2(
    mut q_checkbox: Query<
        (Has<Checked>, &Hovered, Has<InteractionDisabled>, &Children),
        Or<(With<DemoCheckbox>, With<DemoRadio>)>,
    >,
    mut q_border_color: Query<
        (&mut BorderColor, &mut Children),
        (Without<DemoCheckbox>, Without<DemoRadio>),
    >,
    mut q_bg_color: Query<
        &mut BackgroundColor,
        (Without<DemoCheckbox>, Without<DemoRadio>, Without<Children>),
    >,
    mut removed_checked: RemovedComponents<Checked>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
) {
    removed_checked
        .read()
        .chain(removed_disabled.read())
        .for_each(|entity| {
            if let Ok((checked, Hovered(is_hovering), is_disabled, children)) =
                q_checkbox.get_mut(entity)
            {
                let Some(border_id) = children.first() else {
                    return;
                };

                let Ok((mut border_color, border_children)) = q_border_color.get_mut(*border_id)
                else {
                    return;
                };

                let Some(mark_id) = border_children.first() else {
                    warn!("Checkbox does not have a mark entity.");
                    return;
                };

                let Ok(mut mark_bg) = q_bg_color.get_mut(*mark_id) else {
                    warn!("Checkbox mark entity lacking a background color.");
                    return;
                };

                set_checkbox_or_radio_style(
                    is_disabled,
                    *is_hovering,
                    checked,
                    &mut border_color,
                    &mut mark_bg,
                );
            }
        });
}

fn set_checkbox_or_radio_style(
    disabled: bool,
    hovering: bool,
    checked: bool,
    border_color: &mut BorderColor,
    mark_bg: &mut BackgroundColor,
) {
    let color: Color = if disabled {
        // If the element is disabled, use a lighter color
        ELEMENT_OUTLINE.with_alpha(0.2)
    } else if hovering {
        // If hovering, use a lighter color
        ELEMENT_OUTLINE.lighter(0.2)
    } else {
        // Default color for the element
        ELEMENT_OUTLINE
    };

    // Update the background color of the element
    border_color.set_all(color);

    let mark_color: Color = match (disabled, checked) {
        (true, true) => ELEMENT_FILL_DISABLED,
        (false, true) => ELEMENT_FILL,
        (_, false) => Srgba::NONE.into(),
    };

    if mark_bg.0 != mark_color {
        // Update the color of the element
        mark_bg.0 = mark_color;
    }
}

/// Create a demo radio group
fn radio_group(asset_server: &AssetServer, on_change: Callback<In<Activate>>) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            column_gap: px(4),
            ..default()
        },
        Name::new("RadioGroup"),
        CoreRadioGroup { on_change },
        TabIndex::default(),
        children![
            (radio(asset_server, TrackClick::Drag, "Slider Drag"),),
            (radio(asset_server, TrackClick::Step, "Slider Step"),),
            (radio(asset_server, TrackClick::Snap, "Slider Snap"),)
        ],
    )
}

/// Create a demo radio button
fn radio(asset_server: &AssetServer, value: TrackClick, caption: &str) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            align_content: AlignContent::Center,
            column_gap: px(4),
            ..default()
        },
        Name::new("RadioButton"),
        Hovered::default(),
        DemoRadio(value),
        CoreRadio,
        Children::spawn((
            Spawn((
                // Radio outer
                Node {
                    display: Display::Flex,
                    width: px(16),
                    height: px(16),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BorderColor::all(ELEMENT_OUTLINE), // Border color for the radio button
                BorderRadius::MAX,
                children![
                    // Radio inner
                    (
                        Node {
                            display: Display::Flex,
                            width: px(8),
                            height: px(8),
                            position_type: PositionType::Absolute,
                            left: px(2),
                            top: px(2),
                            ..default()
                        },
                        BorderRadius::MAX,
                        BackgroundColor(ELEMENT_FILL),
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

fn toggle_disabled(
    input: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<
        (Entity, Has<InteractionDisabled>),
        Or<(
            With<CoreButton>,
            With<CoreSlider>,
            With<CoreCheckbox>,
            With<CoreRadio>,
        )>,
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
