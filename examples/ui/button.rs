use bevy::prelude::*;

/// This example illustrates how to create a button that changes color and text based on its interaction state.
fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<EventListenerState>()
        .init_resource::<ButtonMaterials>()
        .add_startup_system(setup.system())
        .add_system(button_system.system())
        .run();
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}

impl FromResources for ButtonMaterials {
    fn from_resources(resources: &Resources) -> Self {
        let mut materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            hovered: materials.add(Color::rgb(0.05, 0.05, 0.05).into()),
            pressed: materials.add(Color::rgb(0.1, 0.5, 0.1).into()),
        }
    }
}

#[derive(Default)]
struct EventListenerState {
    region_reader: EventReader<PointerRegion>,
    press_reader: EventReader<PointerPress>,
    click_reader: EventReader<PointerClick>,
}

#[allow(clippy::too_many_arguments)]
fn button_system(
    button_materials: Res<ButtonMaterials>,
    mut state: ResMut<EventListenerState>,
    region_events: Res<Events<PointerRegion>>,
    press_events: Res<Events<PointerPress>>,
    click_events: Res<Events<PointerClick>>,
    mut interaction_query: Query<(
        &Button,
        Mutated<Interaction>,
        &mut Handle<ColorMaterial>,
        &Children,
    )>,
    event_query: Query<&Button>,
    text_query: Query<&mut Text>,
) {
    for (_button, interaction, mut material, children) in &mut interaction_query.iter() {
        let mut text = text_query.get_mut::<Text>(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                text.value = "Press".to_string();
                *material = button_materials.pressed;
            }
            Interaction::Hovered => {
                text.value = "Hover".to_string();
                *material = button_materials.hovered;
            }
            Interaction::None => {
                text.value = "Button".to_string();
                *material = button_materials.normal;
            }
        }
    }

    for event in state.region_reader.iter(&region_events) {
        if let Ok(_button) = event_query.get::<Button>(event.entity) {
            match event.action {
                RegionAction::Enter => println!("Enter"),
                RegionAction::Exit => println!("Exit"),
                RegionAction::Hover(pos) => println!("Hover: {}", pos),
                RegionAction::Move(pos) => println!("Move: {}", pos),
            }
        }
    }

    for event in state.press_reader.iter(&press_events) {
        if let Ok(_button) = event_query.get::<Button>(event.entity) {
            match event.action {
                PressAction::Up => println!("Up"),
                PressAction::Down => println!("Down"),
            }
        }
    }

    for event in state.click_reader.iter(&click_events) {
        if let Ok(_button) = event_query.get::<Button>(event.entity) {
            println!("Click")
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    button_materials: Res<ButtonMaterials>,
) {
    commands
        // ui camera
        .spawn(UiCameraComponents::default())
        .spawn(ButtonComponents {
            style: Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                // center button
                margin: Rect::all(Val::Auto),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: button_materials.normal,
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(TextComponents {
                text: Text {
                    value: "Button".to_string(),
                    font: asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap(),
                    style: TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.8, 0.8, 0.8),
                    },
                },
                ..Default::default()
            });
        });
}
