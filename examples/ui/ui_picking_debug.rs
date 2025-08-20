//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::{
    color::palettes::css::RED, input_focus::InputFocus, prelude::*, prelude::*,
    winit::WinitSettings,
};

pub struct DragState {
    
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        // `InputFocus` must be set for accessibility to recognize the button.
        .init_resource::<InputFocus>()
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(9, 1.0),
            ..Default::default()
        },))
        .with_children(|parent| {
            let light_a = Color::srgb(0.8, 0.8, 0.99).darker(0.2); // soft blue
            let light_b = Color::srgb(0.8, 0.99, 0.8).darker(0.2); // soft green

            for i in 0..81 {
                    let color = if i % 2 == 0 { light_a } else { light_b };
                    

                    parent
                        .spawn((
                            Node {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                border: UiRect::all(Val::Px(5.)),
                                ..Default::default()
                            },
                            BorderColor::all(color.lighter(0.1)),
                            BackgroundColor(color),
                            Outline {
                                width: Val::Px(2.),
                                offset: Val::Px(-1.),
                                color: Color::NONE,
                            },
                            GlobalZIndex::default()
                        ))
                        .observe(|on: On<Pointer<Over>>, mut query: Query<(&mut Outline, &mut GlobalZIndex)>| {
                            if let Ok((mut outline, mut gzi)) = query.get_mut(on.target()) {
                                outline.color = RED.into();
                                gzi.0 = 1;
                            }
                        })
                        .observe(|on: On<Pointer<Out>>, mut query: Query<(&mut Outline, &mut GlobalZIndex)>| {
                            if let Ok((mut outline, mut gzi)) = query.get_mut(on.target()) {
                                outline.color = Color::NONE;
                                gzi.0 = 0;
                            }
                        })
                        .observe(|on: On<Pointer<DragStart>>| {
                            if on.button != PointerButton::Primary {
                                return;
                            }

                            

                        })
                        .observe(|on_move: On<Pointer<Move>>| {
                            let m = on_move.event();

                            println!("pointer_location: {}", m.pointer_location.position);
                            println!("delta: {}", m.delta);
                            println!("event.delta: {}", m.event.delta);
                            println!("event.hit.position: {:?}", m.hit.position);
                            println!("event.hit.normal: {:?}", m.hit.normal);
                        })
                        .with_child(Text::new(format!("{i}")));
                }
            });
        }


fn update() {}
