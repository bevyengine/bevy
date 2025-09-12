//! This experimental example illustrates how to create widgets using the `bevy_ui_widgets` widget set.
//!
//! The patterns shown here are likely to change substantially as the `bevy_ui_widgets` crate
//! matures, so please exercise caution if you are using this as a reference for your own code.

use bevy::{
    color::palettes::basic::*,
    ecs::system::SystemId,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
        InputDispatchPlugin,
    },
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled, Pressed},
    ui_widgets::{
        Activate, Button, Callback, Checkbox, Slider, SliderRange, SliderThumb, SliderValue,
        UiWidgetsPlugins, ValueChange,
    },
};
use std::any::{Any, TypeId};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            UiWidgetsPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        .insert_resource(DemoWidgetStates { slider_value: 50.0 })
        .add_systems(Startup, setup)
        .add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Add, InteractionDisabled>)
        .add_observer(button_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(button_on_interaction::<Insert, Hovered>)
        .add_observer(slider_on_interaction::<Add, InteractionDisabled>)
        .add_observer(slider_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(slider_on_interaction::<Insert, Hovered>)
        .add_observer(slider_on_change_value::<SliderValue>)
        .add_observer(slider_on_change_value::<SliderRange>)
        .add_observer(checkbox_on_interaction::<Add, InteractionDisabled>)
        .add_observer(checkbox_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(checkbox_on_interaction::<Insert, Hovered>)
        .add_observer(checkbox_on_interaction::<Add, Checked>)
        .add_observer(checkbox_on_interaction::<Remove, Checked>)
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

    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(&assets, on_click, on_change_value));
}

fn demo_root(
    asset_server: &AssetServer,
    on_click: SystemId<In<Activate>>,
    on_change_value: SystemId<In<ValueChange<f32>>>,
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
            button(asset_server, Callback::System(on_click)),
            slider(0.0, 100.0, 50.0, Callback::System(on_change_value)),
            checkbox(asset_server, "Checkbox", Callback::Ignore),
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
        Button {
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

fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    mut buttons: Query<
        (
            &Hovered,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<DemoButton>,
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((hovered, disabled, pressed, mut color, mut border_color, children)) =
        buttons.get_mut(event.event_target())
    {
        if children.is_empty() {
            return;
        }
        let Ok(mut text) = text_query.get_mut(children[0]) else {
            return;
        };
        let hovered = hovered.get();
        let pressed = pressed && !(E::is::<Remove>() && C::is::<Pressed>());
        let disabled = disabled && !(E::is::<Remove>() && C::is::<InteractionDisabled>());
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
        Slider {
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
                    height: px(6),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK), // Border color for the checkbox
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
                    SliderThumb,
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

fn slider_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    sliders: Query<(Entity, &Hovered, Has<InteractionDisabled>), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, hovered, disabled)) = sliders.get(event.event_target()) {
        let disabled = disabled && !(E::is::<Remove>() && C::is::<InteractionDisabled>());
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child)
                && is_thumb
            {
                thumb_bg.0 = thumb_color(disabled, hovered.0);
            }
        }
    }
}

fn slider_on_change_value<C: Component>(
    insert: On<Insert, C>,
    sliders: Query<(Entity, &SliderValue, &SliderRange), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, value, range)) = sliders.get(insert.entity) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, is_thumb)) = thumbs.get_mut(child)
                && is_thumb
            {
                thumb_node.left = percent(range.thumb_position(value.0) * 100.0);
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
        Checkbox { on_change },
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
                BorderColor::all(CHECKBOX_OUTLINE), // Border color for the checkbox
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

fn checkbox_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    checkboxes: Query<
        (&Hovered, Has<InteractionDisabled>, Has<Checked>, &Children),
        With<DemoCheckbox>,
    >,
    mut borders: Query<(&mut BorderColor, &mut Children), Without<DemoCheckbox>>,
    mut marks: Query<&mut BackgroundColor, (Without<DemoCheckbox>, Without<Children>)>,
) {
    if let Ok((hovered, disabled, checked, children)) = checkboxes.get(event.event_target()) {
        let hovered = hovered.get();
        let checked = checked && !(E::is::<Remove>() && C::is::<Checked>());
        let disabled = disabled && !(E::is::<Remove>() && C::is::<InteractionDisabled>());

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
        } else if hovered {
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
        Or<(With<Button>, With<Slider>, With<Checkbox>)>,
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

trait Is {
    fn is<T: Any>() -> bool;
}

impl<A: Any> Is for A {
    #[inline]
    fn is<T: Any>() -> bool {
        TypeId::of::<A>() == TypeId::of::<T>()
    }
}
