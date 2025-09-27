//! Shows how exclusive app hooks work for modifying subapps at runtime. This example only allows access from the main app to specific sub apps.

use bevy::{
    ecs::{intern::Interned, system::SystemId},
    prelude::*,
};

#[derive(Resource, Default)]
struct CallWithSubAppList(Vec<CallWithSubApp>);
struct CallWithSubApp(SystemId<InMut<'static, SubApp>, ()>, Interned<dyn AppLabel>);
impl Command for CallWithSubApp {
    fn apply(self, w: &mut World) {
        w.resource_mut::<CallWithSubAppList>().0.push(self);
    }
}
struct SubAppAccessPlugin;
impl Plugin for SubAppAccessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CallWithSubAppList>()
            .add_exclusive_hook(hook);
    }
}

#[derive(AppLabel, Clone, Copy, Hash, Default, Debug, PartialEq, Eq)]
struct OurSubAppLabel;

fn print_hi() {
    println!("Hello from sub app");
}

fn subapp_access(mut sub_app: InMut<SubApp>) {
    sub_app.world_mut().spawn(Name::new("Hello from sub app"));
    let id = sub_app.register_system(print_hi);
    sub_app.world_mut().run_system(id).unwrap();
}

fn main() {
    let mut app = App::new();
    app.add_plugins(SubAppAccessPlugin)
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, init)
        .insert_sub_app(OurSubAppLabel, SubApp::default());
    app.run();
}

fn init(mut commands: Commands) {
    let id = commands.register_system(subapp_access);
    commands.queue(CallWithSubApp(id, OurSubAppLabel.intern()));
}

fn hook(app: &mut App) {
    let list = std::mem::take(&mut app.world_mut().resource_mut::<CallWithSubAppList>().0);
    for command in list {
        #[expect(unsafe_code, reason = "App is guaranteed to outlive the subapp")]
        // SAFETY: The app is guaranteed to outlive the SubApp
        let sub_app = unsafe {
            core::ptr::from_mut(
                app.sub_apps_mut()
                    .sub_apps
                    .get_mut(&command.1)
                    .expect("Failed to get sub app"),
            )
            .as_mut()
            .unwrap_unchecked()
        };
        app.world_mut()
            .run_system_with(command.0, sub_app)
            .expect("Failed to run system on sub app");
    }
}
