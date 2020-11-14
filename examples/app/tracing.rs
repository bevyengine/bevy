use bevy::{input::system::exit_on_esc_system, prelude::*};
use std::{thread, time};
use tracing::info;
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

pub fn setup_global_subscriber() -> impl Drop {
    let fmt_layer = fmt::Layer::default();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,wgpu=warn"))
        .unwrap();

    let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(fmt_layer)
        .with(chrome_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    _guard
}

fn main() {
    let _guard = setup_global_subscriber();

    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(a_system.system())
        .add_system(foo_bar_baz.system())
        .add_system(exit_on_esc_system.system())
        .run();
}

fn a_system(commands: &mut Commands) {
    let ten_millis = time::Duration::from_millis(10);
    thread::sleep(ten_millis);

    commands.spawn((GlobalTransform::default(), Transform::default()));
}

fn foo_bar_baz(query: Query<&Transform>) {
    for transform in query.iter() {
        let five_millis = time::Duration::from_millis(5);
        thread::sleep(five_millis);

        info!(?transform);
    }
}
