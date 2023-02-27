use bevy::prelude::*;

fn run_sub_app(mut sub_app: NonSendMut<DebugApp>) {
    sub_app.app.update();
}

struct DebugApp {
    app: App,
}

fn main() {
    let mut app = App::new();

    let sub_app = App::new();
    app.insert_non_send_resource(DebugApp { app: sub_app });
    app.add_system(run_sub_app);

    app.update();
}
