use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(mouse_position.system())
        .run();
}

// Register ::Horizontal and ::Vertical resources, so they can be read
fn setup(mut mouse_axis: ResMut<Axis<Mouse>>, mut cursor_axis: ResMut<Axis<Cursor>>) {
    // Axis<Mouse> represents raw mouse movements, without acceleration
    // but it's inconsistent across platforms
    // same mouse move can have a different scale
    // ideal for 3D camera movement etc.
    mouse_axis.register(Mouse::Horizontal);
    mouse_axis.register(Mouse::Vertical);

    // Axis<Cursor> represents position of cursor related to window's origin
    // it's accurate for determining cursor location
    // but not for detecting mouse movements as it's affected by acceleration
    // also cursor will stop changing once window is not focused
    // and synchronise itself when the window is focused again
    cursor_axis.register(Cursor::Horizontal(WindowId::primary()));
    cursor_axis.register(Cursor::Vertical(WindowId::primary()));
}

fn mouse_position(mouse_axis: Res<Axis<Mouse>>, cursor_axis: Res<Axis<Cursor>>) {
    // Axis<Mouse/Cursor> will return Option<f32> that is either Some(f32)
    // if the axis was registered and None if axis was not registered
    println!(
        "mouse: [x: :{}, y: :{}] cursor: [x: :{}, y: :{}]",
        mouse_axis.get(Mouse::Horizontal).unwrap(),
        mouse_axis.get(Mouse::Vertical).unwrap(),
        cursor_axis
            .get(Cursor::Horizontal(WindowId::primary()))
            .unwrap(),
        cursor_axis
            .get(Cursor::Vertical(WindowId::primary()))
            .unwrap(),
    );
}
