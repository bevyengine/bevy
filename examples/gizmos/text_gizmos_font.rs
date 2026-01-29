//! Example displaying all the available glyphs from the Simplex Hershey font
//! used by `bevy_gizmos`

use bevy::prelude::*;
use bevy_text_gizmos::prelude::*;

const ALL_GLYPHS: &'static str = " !\"#$%&'()*\n\
+,-./012345\n\
6789:;<=>?@\n\
ABCDEFGHIJK\n\
LMNOPQRSTUV\n\
WXYZ[\\]^_`a\n\
bcdefghijkl\n\
mnopqrstuvw\n\
xyz{|}~";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, draw_all_glyphs)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn draw_all_glyphs(mut text_gizmos: Gizmos) {
    text_gizmos.text_2d(
        Isometry2d::from_xy(-200., 200.),
        ALL_GLYPHS,
        30.0,
        Color::WHITE,
    );
}
