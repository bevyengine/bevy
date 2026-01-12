//! An example demonstrating a mobile scrollable list with clickable items
use bevy::{color::palettes::css, prelude::*};

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_list(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            overflow: Overflow::scroll_y(),
            ..Default::default()
        })
        .observe(scroll_list)
        .with_children(|s| {
            for n in 1..=100 {
                s.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::vertical(Val::Px(10.)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BackgroundColor(css::GRAY.into()),
                ))
                .observe(click_item)
                .with_child(Text::new(format!("Item {n}")));
            }
        });
}

fn scroll_list(
    event: On<Pointer<Drag>>,
    mut scroll_position_query: Query<(&mut ScrollPosition, &ComputedNode)>,
) {
    if let Ok((mut scroll_position, computed_node)) = scroll_position_query.get_mut(event.entity) {
        scroll_position.y =
            (scroll_position.y - event.delta.y).clamp(0., computed_node.content_size.y);
    }
}

fn click_item(event: On<Pointer<Click>>, text_query: Query<(&ChildOf, &Text)>) {
    if !event.dragged
        && let Some((_, text)) = text_query
            .iter()
            .find(|(child_of, _)| child_of.parent() == event.entity)
    {
        println!("Clicked on {}", &text.0);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (spawn_camera, spawn_list))
        .run();
}
