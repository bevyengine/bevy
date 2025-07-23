//! multiple text inputs example

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::YELLOW;
use bevy::color::palettes::tailwind::GRAY_600;
use bevy::core_widgets::Activate;
use bevy::core_widgets::Callback;
use bevy::core_widgets::CoreButton;
use bevy::core_widgets::CoreWidgetsPlugins;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::Clipboard;
use bevy::text::Prompt;
use bevy::text::PromptColor;
use bevy::text::TextInputFilter;
use bevy::text::TextInputPasswordMask;
use bevy::text::TextInputSubmit;
use bevy::text::TextInputValue;
use bevy::ui::widget::TextInput;
use bevy_ecs::relationship::RelatedSpawnerCommands;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
            CoreWidgetsPlugins,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_targets, update_clipboard_display))
        .run();
}

#[derive(Component)]
struct DemoInput;

const FONT_OPTIONS: [[&'static str; 2]; 3] = [
    ["fonts/FiraMono-Medium.ttf", "FiraMono"],
    ["fonts/FiraSans-Bold.ttf", "FiraSans"],
    ["fonts/Orbitron-Medium.ttf", "Orbitron"],
];

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);

    let last_submission = commands.spawn(Text::new("None")).id();

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.),
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn((
                    Node {
                        display: Display::Grid,
                        border: UiRect::all(Val::Px(5.)),
                        padding: UiRect::all(Val::Px(25.)),
                        row_gap: Val::Px(20.),
                        column_gap: Val::Px(20.),
                        grid_template_columns: vec![GridTrack::fr(1.); 3],
                        ..default()
                    },
                    BorderColor::all(YELLOW.into()),
                    BackgroundColor(NAVY.into()),
                    TabGroup::default(),
                ))
                .with_children(|commands| {
                    for (i, label) in ["Input", "Value", "Submission"].into_iter().enumerate() {
                        commands.spawn((
                            Text::new(label),
                            Node {
                                grid_column: GridPlacement::start(i as i16 + 1),
                                grid_row: GridPlacement::start(1),
                                justify_self: JustifySelf::Center,
                                ..Default::default()
                            },
                        ));
                    }

                    inputs_grid(commands);
                });

            commands
                .spawn((
                    Node {
                        width: Val::Px(600.),
                        border: UiRect::all(Val::Px(2.)),
                        padding: UiRect::all(Val::Px(4.)),
                        ..Default::default()
                    },
                    children![(Text::new("Last submission: "), TextColor(YELLOW.into()))],
                ))
                .add_child(last_submission);

            commands.spawn((
                Node {
                    width: Val::Px(600.),
                    border: UiRect::all(Val::Px(2.)),
                    padding: UiRect::all(Val::Px(4.)),
                    ..Default::default()
                },
                children![
                    (Text::new("Clipboard contents: "), TextColor(YELLOW.into())),
                    (Text::default(), ClipboardMarker)
                ],
            ));

            commands
                .spawn((Node {
                    column_gap: Val::Px(20.),
                    ..Default::default()
                },))
                .with_children(|commands| {
                    for [font, label] in FONT_OPTIONS.iter() {
                        let font = asset_server.load(*font);
                        spawn_font_button(commands, font, label);
                    }
                });
        })
        .observe(
            move |on_submit: On<TextInputSubmit>, mut text_query: Query<&mut Text>| {
                if let Ok(mut text) = text_query.get_mut(last_submission) {
                    text.0 = on_submit.event().text.clone();
                }
            },
        );
}

fn inputs_grid(commands: &mut RelatedSpawnerCommands<ChildOf>) {
    for (n, (label, input_filter, password)) in [
        ("text", None, false),
        ("alphanumeric", Some(TextInputFilter::Alphanumeric), false),
        ("decimal", Some(TextInputFilter::Decimal), false),
        ("hex", Some(TextInputFilter::Hex), false),
        ("integer", Some(TextInputFilter::Integer), false),
        ("password", Some(TextInputFilter::Alphanumeric), true),
    ]
    .into_iter()
    .enumerate()
    {
        spawn_row(
            commands,
            GridPlacement::start(n as i16 + 2),
            label,
            input_filter,
            password,
        );
    }
}

fn spawn_row(
    commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    grid_row: GridPlacement,
    label: &str,
    input_filter: Option<TextInputFilter>,
    is_password: bool,
) {
    let update_target = commands
        .spawn((
            Text::default(),
            TextColor(Color::WHITE),
            TextLayout::new_with_no_wrap(),
        ))
        .id();

    let submit_target = commands
        .spawn((
            Text::default(),
            TextColor(Color::WHITE),
            TextLayout::new_with_no_wrap(),
        ))
        .id();

    commands
        .spawn((
            Node {
                display: Display::Grid,
                width: Val::Px(200.),
                overflow: Overflow::clip(),
                grid_row,
                grid_column: GridPlacement::start(2),
                justify_content: JustifyContent::Start,
                padding: UiRect::all(Val::Px(4.)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .add_child(update_target);

    commands
        .spawn((
            Node {
                display: Display::Grid,
                width: Val::Px(200.),
                overflow: Overflow::clip(),
                grid_row,
                grid_column: GridPlacement::start(3),
                justify_content: JustifyContent::Start,
                padding: UiRect::all(Val::Px(4.)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .add_child(submit_target);

    let mut input = commands.spawn((
        TextInput {
            justify: Justify::Left,
        },
        Prompt::new(label),
        PromptColor::new(GRAY_600),
        TextColor(Color::WHITE),
        TabIndex(0),
        DemoInput,
        TextInputValue::default(),
        UpdateTarget(update_target),
    ));

    if let Some(input_filter) = input_filter {
        input.insert(input_filter);
    }

    input.observe(
        move |on_submit: On<TextInputSubmit>, mut text_query: Query<&mut Text>| {
            if let Ok(mut text) = text_query.get_mut(submit_target) {
                text.0 = on_submit.event().text.clone();
            }
        },
    );

    if is_password {
        input.insert(TextInputPasswordMask::default());
    }

    let input_id = input.id();

    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Px(200.),
                grid_row,
                grid_column: GridPlacement::start(1),
                padding: UiRect::all(Val::Px(4.)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
            Outline {
                width: Val::Px(1.),
                color: Color::WHITE,
                ..Default::default()
            },
        ))
        .add_child(input_id);
}

fn spawn_font_button(
    commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    font: Handle<Font>,
    label: &str,
) {
    let on_activate = commands.commands().register_system(
        move |_: In<Activate>, mut query: Query<&mut TextFont, With<DemoInput>>| {
            for mut text_input_font in query.iter_mut() {
                text_input_font.font = font.clone();
            }
        },
    );

    commands.spawn((
        Node {
            padding: UiRect::all(Val::Px(5.)),
            border: UiRect::all(Val::Px(2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        CoreButton {
            on_activate: Callback::System(on_activate),
        },
        Hovered::default(),
        BorderColor::all(Color::BLACK),
        BorderRadius::MAX,
        BackgroundColor(NAVY.into()),
        children![(Text::new(label),)],
    ));
}

#[derive(Component)]
struct UpdateTarget(Entity);

#[derive(Component)]
struct ClipboardMarker;

fn update_targets(
    values_query: Query<(&TextInputValue, &UpdateTarget), Changed<TextInputValue>>,
    mut text_query: Query<&mut Text>,
) {
    for (value, target) in values_query.iter() {
        if let Ok(mut text) = text_query.get_mut(target.0) {
            text.0 = value.get().to_string();
        }
    }
}

fn update_clipboard_display(
    clipboard: Res<Clipboard>,
    mut text_query: Query<&mut Text, With<ClipboardMarker>>,
) {
    if clipboard.is_changed() {
        for mut text in text_query.iter_mut() {
            text.0 = clipboard.0.clone();
        }
    }
}
