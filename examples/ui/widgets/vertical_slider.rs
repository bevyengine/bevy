//! Simple example showing vertical and horizontal slider widgets with snap behavior and value labels

use bevy::{
    input_focus::{
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
        InputDispatchPlugin,
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        observe, slider_self_update, CoreSliderDragState, Slider, SliderRange, SliderThumb,
        SliderValue, TrackClick, UiWidgetsPlugins,
    },
};

const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            UiWidgetsPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_slider_visuals, update_value_labels))
        .run();
}

#[derive(Component)]
struct ValueLabel(Entity);

#[derive(Component)]
struct DemoSlider;

#[derive(Component)]
struct DemoSliderThumb;

#[derive(Component)]
struct VerticalSlider;

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: px(50),
                ..default()
            },
            TabGroup::default(),
        ))
        .with_children(|parent| {
            // Vertical slider
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(10),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Vertical"),
                        TextFont {
                            font: assets.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));

                    let label_id = parent
                        .spawn((
                            Text::new("50"),
                            TextFont {
                                font: assets.load("fonts/FiraSans-Bold.ttf").into(),
                                font_size: FontSize::Px(24.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ))
                        .id();

                    parent.spawn((
                        vertical_slider(),
                        ValueLabel(label_id),
                        observe(slider_self_update),
                    ));
                });

            // Horizontal slider
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(10),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Horizontal"),
                        TextFont {
                            font: assets.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));

                    let label_id = parent
                        .spawn((
                            Text::new("50"),
                            TextFont {
                                font: assets.load("fonts/FiraSans-Bold.ttf").into(),
                                font_size: FontSize::Px(24.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ))
                        .id();

                    parent.spawn((
                        horizontal_slider(),
                        ValueLabel(label_id),
                        observe(slider_self_update),
                    ));
                });
        });
}

fn vertical_slider() -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            column_gap: px(4),
            width: px(12),
            height: px(200),
            ..default()
        },
        DemoSlider,
        VerticalSlider,
        Hovered::default(),
        Slider {
            track_click: TrackClick::Snap,
        },
        SliderValue(50.0),
        SliderRange::new(0.0, 100.0),
        TabIndex(0),
        Children::spawn((
            Spawn((
                Node {
                    width: px(6),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK),
            )),
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    top: px(12),
                    bottom: px(0),
                    left: px(0),
                    right: px(0),
                    ..default()
                },
                children![(
                    DemoSliderThumb,
                    SliderThumb,
                    Node {
                        display: Display::Flex,
                        width: px(12),
                        height: px(12),
                        position_type: PositionType::Absolute,
                        bottom: percent(0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

fn horizontal_slider() -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            column_gap: px(4),
            height: px(12),
            width: px(200),
            ..default()
        },
        DemoSlider,
        Hovered::default(),
        Slider {
            track_click: TrackClick::Snap,
        },
        SliderValue(50.0),
        SliderRange::new(0.0, 100.0),
        TabIndex(0),
        Children::spawn((
            Spawn((
                Node {
                    height: px(6),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK),
            )),
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(12),
                    top: px(0),
                    bottom: px(0),
                    ..default()
                },
                children![(
                    DemoSliderThumb,
                    SliderThumb,
                    Node {
                        display: Display::Flex,
                        width: px(12),
                        height: px(12),
                        position_type: PositionType::Absolute,
                        left: percent(0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

fn update_slider_visuals(
    sliders: Query<
        (
            Entity,
            &SliderValue,
            &SliderRange,
            &Hovered,
            &CoreSliderDragState,
            Has<VerticalSlider>,
        ),
        (
            Or<(
                Changed<SliderValue>,
                Changed<Hovered>,
                Changed<CoreSliderDragState>,
            )>,
            With<DemoSlider>,
        ),
    >,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, &mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    for (slider_ent, value, range, hovered, drag_state, is_vertical) in sliders.iter() {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, mut thumb_bg, is_thumb)) = thumbs.get_mut(child)
                && is_thumb
            {
                let position = range.thumb_position(value.0) * 100.0;
                if is_vertical {
                    thumb_node.bottom = percent(position);
                } else {
                    thumb_node.left = percent(position);
                }

                let is_active = hovered.0 | drag_state.dragging;
                thumb_bg.0 = if is_active {
                    SLIDER_THUMB.lighter(0.3)
                } else {
                    SLIDER_THUMB
                };
            }
        }
    }
}

fn update_value_labels(
    sliders: Query<(&SliderValue, &ValueLabel), (Changed<SliderValue>, With<DemoSlider>)>,
    mut texts: Query<&mut Text>,
) {
    for (value, label) in sliders.iter() {
        if let Ok(mut text) = texts.get_mut(label.0) {
            **text = format!("{:.0}", value.0);
        }
    }
}
