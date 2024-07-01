//! a test that confirms that 'bevy' does not panic while changing from Windowed to SizedFullscreen when viewport is set

use bevy::
{   prelude::*,
    render::camera::Viewport,
    window::WindowMode,
};

//Having a viewport set to the same size as a window used to cause panic on some occasions when switching to SizedFullscreen
const WINDOW_WIDTH : f32 = 1366.0;
const WINDOW_HEIGHT: f32 = 768.0;

fn main()
{   //Specify Window Size.
    let window = Window { resolution: ( WINDOW_WIDTH, WINDOW_HEIGHT ).into(), ..default() };
    let primary_window = Some ( window );

    App::new()
        .add_plugins( DefaultPlugins.set( WindowPlugin { primary_window, ..default() } ) )
        .add_systems( Startup, startup )
        .add_systems( Update, toggle_window_mode )
        .run();
}

fn startup( mut cmds: Commands )
{   //Match viewport to Window size.
    let physical_position = UVec2::new( 0, 0 );
    let physical_size = Vec2::new( WINDOW_WIDTH, WINDOW_HEIGHT ).as_uvec2();
    let viewport = Some ( Viewport { physical_position, physical_size, ..default() } );

    cmds.spawn( Camera2dBundle::default() ).insert( Camera { viewport, ..default() } );
}

fn toggle_window_mode
(   mut qry_window: Query<&mut Window>,
)
{   let Ok( mut window ) = qry_window.get_single_mut() else { return };

    window.mode = match window.mode {
        WindowMode::Windowed => {
            //it takes a while for the window to change from windowed to sizedfullscreen and back
            std::thread::sleep(std::time::Duration::from_secs(4));
            WindowMode::SizedFullscreen
        },
        _  => {
            std::thread::sleep(std::time::Duration::from_secs(4));
            WindowMode::Windowed
        },
    };
}