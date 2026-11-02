//! This example demonstrates how to use BSN to compose scenes.
use bevy::{
    ecs::template::EntityTemplate, prelude::*, scene::Ready, text::FontSourceTemplate,
    ui_widgets::Button,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        @ui()
    }
}

fn ui() -> impl Scene {
    bsn! {
        #RootUI
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(5),
        }
        Children [
            @button("Ok", #RootUI)
            on(|_event: On<PointerPress>| println!("Ok pressed!"))
            --
            @button("Cancel", #RootUI)
            on(|_event: On<PointerPress>| println!("Cancel pressed!"))
            BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
        ]
        on(|event: On<Ready>| {
            println!("I'm the root entity => {:#?}", event.entity);
        })
    }
}

#[derive(FromTemplate, Component)]
struct RootUi(Entity);

fn button(label: &str, root_entity: EntityTemplate) -> impl Scene {
    bsn! {
        Button
        RootUi(root_entity)
        Node {
            width: px(150),
            height: px(65),
            border: px(5),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        on(|event: On<PointerPress>, q_root_ui: Query<&RootUi> |{
            let root_ui = q_root_ui.get(event.entity).expect("We are sure it exists");
            println!("This is my Root => {:#?}", root_ui.0);

        })
        Children [
            Text(label)
            TextFont {
                font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
                font_size: px(33.0),
            }
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        ]
    }
}
