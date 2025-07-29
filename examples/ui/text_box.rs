//! minimal text input example
use bevy::color::palettes::css::NAVY;
use bevy::input_focus::tab_navigation::NavAction;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::AutoFocus;
use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::Prompt;
use bevy::text::TextInputVisibleLines;
use bevy::ui::widget::NextFocus;
use bevy::ui::widget::TextBox;
use bevy::ui::widget::TextInputSubmitBehaviour;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    let tid = commands.spawn(Text::new("TextBox example")).id();
    let id = commands
        .spawn((
            TextBox::default(),
            Prompt::new("please type here.."),
            TabIndex(0),
            AutoFocus,
            TextFont {
                font_size: 35.,
                ..Default::default()
            },
            Node {
                width: Val::Percent(50.),
                ..default()
            },
            BackgroundColor(NAVY.into()),
            TextInputVisibleLines(8.),
            TextInputSubmitBehaviour {
                clear_on_submit: true,
                navigate_on_submit: NextFocus::Navigate(NavAction::Next),
            },
        ))
        .id();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.),
                ..Default::default()
            },
            TabGroup::default(),
        ))
        .add_child(tid)
        .add_child(id);

    commands.insert_resource(InputFocus(Some(id)));
}
