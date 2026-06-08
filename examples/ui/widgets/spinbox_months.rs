//! Demonstrates using the headless [`SpinBox`](bevy::ui_widgets::SpinBox) with a non-numeric
//! value type by cycling through a `Month` enum.

use bevy::{
    ecs::relationship::Relationship,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
        InputFocus,
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        observe, Button, SpinBox, SpinBoxButtonPress, SpinBoxDecrementButton, SpinBoxDirection,
        SpinBoxIncrementButton,
    },
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TabNavigationPlugin))
        .init_resource::<InputFocus>()
        .add_systems(Startup, setup)
        .add_systems(Update, (update_button_style, update_month_display))
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);

#[derive(Component)]
struct MonthSpinBox;

#[derive(Component)]
struct MonthDisplay;

#[derive(Component)]
struct DemoButton;

#[derive(Component, Clone, Copy)]
struct MonthValue(Month);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl Month {
    const ALL: [Self; 12] = [
        Self::January,
        Self::February,
        Self::March,
        Self::April,
        Self::May,
        Self::June,
        Self::July,
        Self::August,
        Self::September,
        Self::October,
        Self::November,
        Self::December,
    ];

    fn advance(self, direction: SpinBoxDirection) -> Self {
        let index = Self::ALL.iter().position(|month| *month == self).unwrap();
        let next = match direction {
            SpinBoxDirection::Increment => (index + 1) % Self::ALL.len(),
            SpinBoxDirection::Decrement => (index + Self::ALL.len() - 1) % Self::ALL.len(),
        };
        Self::ALL[next]
    }

    fn label(self) -> &'static str {
        match self {
            Self::January => "January",
            Self::February => "February",
            Self::March => "March",
            Self::April => "April",
            Self::May => "May",
            Self::June => "June",
            Self::July => "July",
            Self::August => "August",
            Self::September => "September",
            Self::October => "October",
            Self::November => "November",
            Self::December => "December",
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        TabGroup::default(),
        children![(
            Node {
                width: px(220),
                height: px(54),
                padding: UiRect::all(px(4)),
                border: UiRect::all(px(2)),
                column_gap: px(6),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(Color::BLACK),
            MonthSpinBox,
            MonthValue(Month::January),
            SpinBox,
            TabIndex(0),
            observe(
                |press: On<SpinBoxButtonPress>,
                 mut spinboxes: Query<&mut MonthValue, With<MonthSpinBox>>| {
                    if let Ok(mut month) = spinboxes.get_mut(press.entity) {
                        month.0 = month.0.advance(press.direction);
                    }
                },
            ),
            children![
                (
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect::horizontal(px(10)),
                        border: UiRect::all(px(1)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                    MonthDisplay,
                    Text::new(Month::January.label()),
                    TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Node {
                        width: px(36),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    },
                    children![
                        demo_button("+", SpinBoxIncrementButton),
                        demo_button("-", SpinBoxDecrementButton),
                    ],
                ),
            ],
        )],
    ));
}

fn demo_button<M: Bundle>(label: &'static str, marker: M) -> impl Bundle {
    (
        DemoButton,
        marker,
        Button,
        Hovered::default(),
        Node {
            flex_grow: 1.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1)),
            ..default()
        },
        BorderColor::all(Color::WHITE),
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

fn update_button_style(
    mut buttons: Query<(&Hovered, &mut BackgroundColor), (With<DemoButton>, Changed<Hovered>)>,
) {
    for (hovered, mut background) in &mut buttons {
        background.0 = if hovered.0 {
            HOVERED_BUTTON
        } else {
            NORMAL_BUTTON
        };
    }
}

fn update_month_display(
    spinboxes: Query<&MonthValue, (With<MonthSpinBox>, Changed<MonthValue>)>,
    mut displays: Query<(&ChildOf, &mut Text), With<MonthDisplay>>,
) {
    for (parent, mut text) in &mut displays {
        if let Ok(month) = spinboxes.get(parent.get()) {
            **text = month.0.label().to_string();
        }
    }
}
