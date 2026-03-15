//! Simple example demonstrating overflow behavior.

use bevy::{
    color::palettes::css::*,
    picking::hover::Hovered,
    prelude::*,
    reflect::Is,
    ui::Pressed,
    ui_widgets::Button,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_observer(update_outlines_on_interaction::<Add, Pressed>)
        .add_observer(update_outlines_on_interaction::<Remove, Pressed>)
        .add_observer(update_outlines_on_interaction::<Insert, Hovered>)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let text_style = TextFont::default();

    let image = asset_server.load("branding/icon.png");

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            BackgroundColor(ANTIQUE_WHITE.into()),
        ))
        .with_children(|parent| {
            for overflow in [
                Overflow::visible(),
                Overflow::clip_x(),
                Overflow::clip_y(),
                Overflow::clip(),
            ] {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        margin: UiRect::horizontal(px(25)),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        let label = format!("{overflow:#?}");
                        parent
                            .spawn((
                                Node {
                                    padding: UiRect::all(px(10)),
                                    margin: UiRect::bottom(px(25)),
                                    ..Default::default()
                                },
                                BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                            ))
                            .with_children(|parent| {
                                parent.spawn((Text::new(label), text_style.clone()));
                            });
                        parent
                            .spawn((
                                Node {
                                    width: px(100),
                                    height: px(100),
                                    padding: UiRect {
                                        left: px(25),
                                        top: px(25),
                                        ..Default::default()
                                    },
                                    border: UiRect::all(px(5)),
                                    overflow,
                                    ..default()
                                },
                                BorderColor::all(Color::BLACK),
                                BackgroundColor(GRAY.into()),
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    ImageNode::new(image.clone()),
                                    Node {
                                        min_width: px(100),
                                        min_height: px(100),
                                        ..default()
                                    },
                                    Button,
                                    // Hover detection
                                    Hovered::default(),
                                    Outline {
                                        width: px(2),
                                        offset: px(2),
                                        color: Color::NONE,
                                    },
                                ));
                            });
                    });
            }
        });
}

fn update_outlines_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    mut outline_query: Query<(&Hovered, Has<Pressed>, &mut Outline), With<Button>>,
) {
    if let Ok((hovered, pressed, mut outline)) = outline_query.get_mut(event.event_target()) {
        let hovered = hovered.get();
        let pressed = pressed && !(E::is::<Remove>() && C::is::<Pressed>());
        outline.color = match (hovered, pressed) {
            (true, true) => RED.into(),
            (true, false) => WHITE.into(),
            (false, _) => Color::NONE,
        }
    }
}
