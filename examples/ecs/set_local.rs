//! Shows how to share a resource between every system in a [`SystemSet`].
//!
//! [`SetLocal<T>`] and [`SetLocalMut<T>`] work like [`Res<T>`] and [`ResMut<T>`], except that
//! the value `T` is local to the set.

use bevy::prelude::*;

/// Shapes queued for drawing by the systems of one layer.
#[derive(Resource, Default)]
struct DrawList(Vec<&'static str>);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum Layer {
    World,
    Ui,
}

fn main() {
    App::new()
        // Gives each layer its own draw list.
        .init_set_local::<DrawList>(Layer::World)
        .init_set_local::<DrawList>(Layer::Ui)
        .add_systems(
            Update,
            (
                (
                    clear,
                    // These write to the same draw list, so they never run at the same time.
                    (queue_squares, queue_circles),
                    // These only read it, so they can run at the same time.
                    (draw, count_shapes),
                )
                    .chain()
                    .in_set(Layer::World),
                // None of these share a draw list with the systems above, so they can run in
                // parallel with them.
                (clear, queue_circles, draw).chain().in_set(Layer::Ui),
            ),
        )
        .run();
}

// The below systems are written in a way that is agnostic to the specific layer they belong to.

fn clear(mut draw_list: SetLocalMut<DrawList>) {
    draw_list.0.clear();
}

fn queue_squares(mut draw_list: SetLocalMut<DrawList>) {
    draw_list.0.push("square");
}

fn queue_circles(mut draw_list: SetLocalMut<DrawList>) {
    draw_list.0.push("circle");
}

fn draw(draw_list: SetLocal<DrawList>) {
    println!("{:?}: drawing {:?}", draw_list.set(), draw_list.0);
}

fn count_shapes(draw_list: SetLocal<DrawList>) {
    println!("{:?}: {} shapes", draw_list.set(), draw_list.0.len());
}
