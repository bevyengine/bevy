//! This example illustrates how to create buttons with their textures sliced
//! and kept in proportion instead of being stretched by the button dimensions

use bevy::{
    color::palettes::css::{GOLD, ORANGE},
    picking::hover::Hovered,
    prelude::*,
    reflect::Is,
    ui::{widget::NodeImageMode, Pressed},
    ui_widgets::Button,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Insert, Hovered>)
        .run();
}

fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    mut buttons: Query<(&Hovered, Has<Pressed>, &Children, &mut ImageNode), With<Button>>,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((hovered, pressed, children, mut image)) = buttons.get_mut(event.event_target()) {
        let mut text = text_query.get_mut(children[0]).unwrap();
        let hovered = hovered.get();
        let pressed = pressed && !(E::is::<Remove>() && C::is::<Pressed>());
        match (hovered, pressed) {
            (true, true) => {
                **text = "Press".to_string();
                image.color = GOLD.into();
            }
            (true, false) => {
                **text = "Hover".to_string();
                image.color = ORANGE.into();
            }
            (false, _) => {
                **text = "Button".to_string();
                image.color = Color::WHITE;
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load("textures/fantasy_ui_borders/panel-border-010.png");

    let slicer = TextureSlicer {
        border: BorderRect::all(22.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };
    // ui camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            for [w, h] in [[150.0, 150.0], [300.0, 150.0], [150.0, 300.0]] {
                parent
                    .spawn((
                        Button,
                        // detect the hover
                        Hovered::default(),
                        ImageNode {
                            image: image.clone(),
                            image_mode: NodeImageMode::Sliced(slicer.clone()),
                            ..default()
                        },
                        Node {
                            width: px(w),
                            height: px(h),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            margin: UiRect::all(px(20)),
                            ..default()
                        },
                    ))
                    .with_child((
                        Text::new("Button"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Px(33.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
            }
        });
}
