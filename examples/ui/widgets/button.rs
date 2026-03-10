//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::{
    color::palettes::basic::*,
    input_focus::InputFocus,
    picking::hover::Hovered,
    prelude::*,
    reflect::Is,
    ui::Pressed,
    ui_widgets::Button,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // `InputFocus` must be set for accessibility to recognize the button.
        .init_resource::<InputFocus>()
        .add_systems(Startup, setup)
        .add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Insert, Hovered>)
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    mut input_focus: ResMut<InputFocus>,
    mut button_query: Query<
        (Entity, &Hovered, Has<Pressed>, &mut BackgroundColor, &mut BorderColor, &mut Button, &Children),
        With<Button>
    >,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((entity, hovered, pressed, mut color, mut border_color, mut button, children)) = button_query.get_mut(event.event_target()) {
        let Some(child) = children.first() else { return; };
        let mut text = text_query.get_mut(*child).unwrap();
        let hovered = hovered.get();
        let pressed = pressed && !(E::is::<Remove>() && C::is::<Pressed>());
        match (hovered, pressed) {
            (true, true) => {
                input_focus.set(entity);
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                *border_color = BorderColor::all(RED);

                // The accessibility system's only update the button's state when the `Button` component is marked as changed.
                button.set_changed();
            }
            (true, false) => {
                input_focus.set(entity);
                **text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                *border_color = BorderColor::all(Color::WHITE);
                button.set_changed();
            }
            (false, _) => {
                input_focus.clear();
                **text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                *border_color = BorderColor::all(Color::BLACK);
            }
        }
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(button(&assets));
}

fn button(asset_server: &AssetServer) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Button,
            // detect the hover
            Hovered::default(),
            Node {
                width: px(150),
                height: px(65),
                border: UiRect::all(px(5)),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(Color::BLACK),
            children![(
                Text::new("Button"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                    font_size: FontSize::Px(33.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            )]
        )],
    )
}
