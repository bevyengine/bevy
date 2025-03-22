use crate::CosmicFontSystem;
use bevy_app::{
    prelude::{App, Plugin},
    Startup, Update,
};
use bevy_ecs::{
    prelude::{resource_exists, Commands, Event, ResMut, Resource},
    schedule::IntoSystemConfigs,
};
use bevy_tasks::{futures::check_ready, AsyncComputeTaskPool, Task};
use cosmic_text::fontdb::Database;

/// Fired once when system fonts are loaded and available in [`CosmicFontSystem`].
#[derive(Event)]
pub struct SystemFontsAvailable;

/// Plugin which loads system fonts
pub(crate) struct SystemFontsPlugin;

impl Plugin for SystemFontsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SystemFontsAvailable>()
            .add_systems(Startup, load_system_fonts)
            .add_systems(
                Update,
                check_system_font_status.run_if(resource_exists::<SystemFontsDatabase>),
            );
    }
}

#[derive(Resource)]
struct SystemFontsDatabase {
    task: Task<Database>,
}

fn load_system_fonts(mut commands: Commands) {
    let thread_pool = AsyncComputeTaskPool::get();

    let task = thread_pool.spawn(async move {
        let mut font_system = cosmic_text::FontSystem::new();

        // This is slow and blocking
        font_system.db_mut().load_system_fonts();

        let (_locale, database) = font_system.into_locale_and_db();

        database
    });

    commands.insert_resource(SystemFontsDatabase { task });
}

fn check_system_font_status(
    mut commands: Commands,
    mut system_fonts: ResMut<SystemFontsDatabase>,
    mut cosmic_font_system: ResMut<CosmicFontSystem>,
) {
    if let Some(mut database) = check_ready(&mut system_fonts.task) {
        let primary_database = cosmic_font_system.db_mut();
        let secondary_database = &mut database;

        // If we loaded more system fonts than the the Bevy application did via assets,
        // swap the databases to minimize copy operations.
        if primary_database.len() < secondary_database.len() {
            core::mem::swap(primary_database, secondary_database);
        }

        for face_info in secondary_database.faces().cloned() {
            primary_database.push_face_info(face_info);
        }

        commands.remove_resource::<SystemFontsDatabase>();
        commands.send_event(SystemFontsAvailable);
    }
}
