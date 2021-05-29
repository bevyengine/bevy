fn main() {
    App::build()
        .insert_resource(XrConfig {
            mode: XrMode::TrackingOnly,
            enable_generic_controllers: false,
        })
        .add_plugins(DefaultPlugins)
        .run();
}
