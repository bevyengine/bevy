/// An example that uses the `NodeOrder` component to reorder UI elements.
use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::width(Val::Percent(100.)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                gap: Size::all(Val::Px(10.)),
                ..Default::default()
            },
            background_color: Color::BLACK.into(),
            ..Default::default()
        })
        .with_children(|builder| {
            for (i, color) in [Color::RED, Color::GREEN, Color::YELLOW, Color::CYAN, Color::AQUAMARINE, Color::CRIMSON, Color::FUCHSIA, Color::PINK].into_iter().enumerate() {
                builder
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::all(Val::Px(100.)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..Default::default()
                        },
                        background_color: color.into(),
                        ..Default::default()
                    })
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section(
                            format!("{i}"),
                            TextStyle {
                                font_size: 60.,
                                color: Color::BLACK,
                                ..Default::default()
                            },
                        ));
                    });
            }
        });
}

fn update(
    mut order: Local<i32>,
    mut button_query: Query<(&Interaction, &mut NodeOrder), Changed<Interaction>>,
) {
    for (interaction, mut node_order) in button_query.iter_mut() {
        if *interaction == Interaction::Clicked {
            *order -= 1;
            node_order.0 = *order;
        }
    }
}
