//! Demonstrates the use of with_rect on ImageNodes.

use bevy::prelude::*;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
    .add_systems(Startup, setup)
    .run();
}

fn setup(
  mut commands: Commands, 
  asset_server: Res<AssetServer>,
  mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
  mut ui_scale : ResMut<UiScale>,
) {
  let texture = asset_server.load("textures/array_texture.png");
  let layout = TextureAtlasLayout::from_grid(UVec2::splat(250), 1, 3, None, None);
  let texture_atlas_layout = texture_atlas_layouts.add(layout);

  ui_scale.0 = 0.5;
  
  commands.spawn(Camera2d);

  commands.spawn(Node {
    display: Display::Flex,
    align_items: AlignItems::Center,
    ..default()
  })
  .with_children(|parent| {

    // this example node displays an texture in its entirety
    parent.spawn(ImageNode::new(texture.clone()));

    // this example node shows a texture constrained by a rect
    parent.spawn(
      ImageNode::new(texture.clone())
        .with_rect(
          Rect::new(0., 200., 250., 450.)
        )
    );

    // this example node displays an index within a texture atlas
    parent.spawn(ImageNode::from_atlas_image(
      texture.clone(),
      TextureAtlas {
        layout: texture_atlas_layout.clone(),
        index: 1,
      },
    ));

    // this example node displays an index within a texture atlas
    // constrained by a rect
    parent.spawn(ImageNode::from_atlas_image(
      texture.clone(),
      TextureAtlas {
        layout: texture_atlas_layout.clone(),
        index: 1,
      },
    ).with_rect(
       Rect::new(0., 0., 150., 150.)
     )
   );

  });
}
