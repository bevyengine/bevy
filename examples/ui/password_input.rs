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

const FONT_PATH: &'static str = "fonts/FiraSans-Bold.ttf";

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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);

    let last_submission = commands.spawn(Text::new("None")).id();

    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.),
            ..Default::default()
        },
        children![(
            Node {
                width: Val::Px(400.),
                border: UiRect::all(Val::Px(5.)),
                ..default()
            },
            BorderColor::all(YELLOW.into()),
            BackgroundColor(NAVY.into()),
        )],
    ));
}
