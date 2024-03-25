//! This example illustrates how to create buttons with their texture atlases sliced
//! and kept in proportion instead of being stretched by the button dimensions

use bevy::{
    color::palettes::css::{GOLD, ORANGE},
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .run();
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut TextureAtlas, &Children, &mut UiImage),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut atlas, children, mut image) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                text.sections[0].value = "Press".to_string();
                atlas.index = (atlas.index + 1) % 30;
                image.color = GOLD.into();
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
                image.color = ORANGE.into();
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
                image.color = Color::WHITE;
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture_handle = asset_server.load("textures/fantasy_ui_borders/border_sheet.png");
    let atlas_layout = TextureAtlasLayout::from_grid(UVec2::new(50, 50), 6, 6, None, None);
    let atlas_layout_handle = texture_atlases.add(atlas_layout);

    let slicer = TextureSlicer {
        border: BorderRect::square(22.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };
    // ui camera
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for (idx, [w, h]) in [
                (0, [150.0, 150.0]),
                (7, [300.0, 150.0]),
                (13, [150.0, 300.0]),
            ] {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(w),
                                height: Val::Px(h),
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                margin: UiRect::all(Val::Px(20.0)),
                                ..default()
                            },
                            image: texture_handle.clone().into(),
                            ..default()
                        },
                        ImageScaleMode::Sliced(slicer.clone()),
                        TextureAtlas {
                            index: idx,
                            layout: atlas_layout_handle.clone(),
                        },
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::srgb(0.9, 0.9, 0.9),
                            },
                        ));
                    });
            }
        });
}
