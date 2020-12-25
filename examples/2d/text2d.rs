use bevy::prelude::*;

enum Location { Above, Below }

fn main() {
    App::build()
        // Uncomment this to override the default log settings:
        .add_resource(bevy::log::LogSettings {
            level: bevy::log::Level::INFO,
            filter: "wgpu=warn,bevy_ecs=info".to_string(),
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        //.add_system(animate.system())
        .run();
}


fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        // 2d camera
        .spawn(Camera2dBundle::default())
        // texture
        //.spawn(Text2dBundle {
        //    text: Text {
        //        value: "This text goes above".to_string(),
        //        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        //        style: TextStyle {
        //            font_size: 60.0,
        //            color: Color::WHITE,
        //            alignment: TextAlignment {
        //                vertical: VerticalAlign::Bottom,
        //                horizontal: HorizontalAlign::Left,
        //            },
        //        },
        //    },
        //    ..Default::default()
        //})
        .with(Location::Above)
        .spawn(Text2dBundle {
            text: Text {
                value: "This text goes below".to_string(),
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::RED,
                    alignment: TextAlignment {
                        vertical: VerticalAlign::Top,
                        horizontal: HorizontalAlign::Center,
                    },
                },
            },
            ..Default::default()
        })
        .with(Location::Below);
}

//fn animate(time: Res<Time>, mut query: Query<(&mut Transform, &Location), With<Text>>) {
//    for (mut transform, location) in query.iter_mut() {
//        match location {
//            Location::Above => {
//                transform.translation.x = 100.0 * time.seconds_since_startup().sin() as f32 - 100f32;
//                transform.translation.y = 100.0 * time.seconds_since_startup().cos() as f32 - 100f32;
//                transform.translation.z = 1.0;
//            }
//            Location::Below => {
//                transform.translation.x = 100.0 * time.seconds_since_startup().cos() as f32 - 100f32;
//                transform.translation.y = 100.0 * time.seconds_since_startup().sin() as f32 - 100f32;
//                transform.translation.z = 0.0;
//            }
//        }
//    }
//}
