use bevy::prelude::*;

/// This example illustrates how to create a button that changes color and text based on its
/// interaction state.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(button_system)
        .run();
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &Children, &mut UiTextureAtlas),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, children, mut atlas) in interaction_query.iter_mut() {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Clicked => {
                text.sections[0].value = "Press".to_string();
                atlas.index = 2;
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
                atlas.index = 1;
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
                atlas.index = 0;
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // load sprite sheet
    let texture_handle = asset_server.load("textures/array_texture.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(250.0, 250.0), 1, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(ButtonSheetBundle {
            style: Style {
                size: Size::new(Val::Px(250.0), Val::Px(250.0)),
                // center button
                margin: Rect::all(Val::Auto),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            texture_atlas: texture_atlas_handle.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section(
                    "Button",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                    Default::default(),
                ),
                ..Default::default()
            });
        });
}
