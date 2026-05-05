//! This example shows how to setup a basic counter app using feathers
//!
//! To use feathers in your bevy app, you need to use the `bevy_feathers` feature

use bevy::{
    feathers::{
        controls::FeathersButton,
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    prelude::*,
    ui_widgets::Activate,
};

#[derive(Resource)]
struct Counter(i32);

#[derive(Component, Default, Clone)]
struct CounterText;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Don't forget to add the plugin.
            // Make sure you are using FeathersPlugins with an `s`
            FeathersPlugins,
        ))
        // Configure feathers to use the dark theme
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(Counter(0))
        .add_systems(Startup, scene.spawn())
        .add_systems(
            Update,
            update_counter_text.run_if(resource_changed::<Counter>),
        )
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, demo_root()]
}

fn demo_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[(
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            }
            Children [
                (
                    :FeathersButton
                    on(|_activate: On<Activate>, mut counter: ResMut<Counter>| {
                        counter.0 -= 1;
                    })
                    Children [ (Text("-1") ThemedText) ]
                ),
                (
                    Node {
                        margin: UiRect::horizontal(px(10.0)),
                    }
                    Text("0") ThemedText CounterText
                ),
                (
                    :FeathersButton
                    on(|_activate: On<Activate>, mut counter: ResMut<Counter>| {
                        counter.0 += 1;
                    })
                    Children [ (Text("+1") ThemedText) ]
                )
            ]
        )]
    }
}

fn update_counter_text(
    counter: Res<Counter>,
    mut counter_text: Single<&mut Text, With<CounterText>>,
) {
    info!("Counter updated");
    counter_text.0 = format!("{}", counter.0);
}
