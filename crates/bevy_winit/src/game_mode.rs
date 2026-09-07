use bevy_app::{App, Plugin};
use bevy_ecs::resource::Resource;
use tracing::{debug, info};
use zbus::blocking::Connection;
use zbus::proxy;

#[proxy(
    interface = "com.feralinteractive.GameMode",
    default_service = "com.feralinteractive.GameMode",
    default_path = "/com/feralinteractive/GameMode"
)]
trait GameMode {
    fn register_game(&self, pid: i32) -> zbus::Result<i32>;
    fn unregister_game(&self, pid: i32) -> zbus::Result<i32>;
    fn query_status(&self, pid: i32) -> zbus::Result<i32>;
}

#[derive(Resource)]
struct GameModeResource {
    proxy: GameModeProxyBlocking<'static>,
    pid: i32,
}

impl Drop for GameModeResource {
    fn drop(&mut self) {
        info!("GameMode: unregistering pid {}", self.pid);
        if let Err(err) = self.proxy.unregister_game(self.pid) {
            debug!("GameMode unregister failed: {err}");
        }
    }
}

/// A [`Plugin`] that integrates with Feral Interactive's `GameMode` daemon on Linux.
///
/// When added, it registers the running process with `GameMode` on startup, which
/// asks the daemon to apply temporary system optimizations (CPU governor, I/O
/// priority, and similar) for the duration of the app. The registration is
/// released on exit.
///
/// If the `GameMode` daemon is not available, the plugin does nothing.
///
/// This plugin is only active on Linux with the `game_mode` feature enabled.
pub struct GameModePlugin;

impl Plugin for GameModePlugin {
    fn build(&self, app: &mut App) {
        gamemode(app);
    }
}

// Connects to the GameMode daemon over D-Bus, registers this process, and stores
// the proxy in a resource so the Drop impl can unregister on exit.
fn gamemode(app: &mut App) {
    let Ok(connection) = Connection::session() else {
        debug!("GameMode: no session bus, skipping");
        return;
    };

    let Ok(proxy) = GameModeProxyBlocking::new(&connection) else {
        debug!("GameMode: proxy init failed, skipping");
        return;
    };

    let pid = std::process::id() as i32;

    if proxy.query_status(pid).is_ok() {
        match proxy.register_game(pid) {
            Ok(_) => {
                info!("GameMode registered, pid {pid}");
                app.insert_resource(GameModeResource { proxy, pid });
            }
            Err(err) => debug!("GameMode: register failed: {err}"),
        }
    } else {
        debug!("GameMode not available, skipping registration");
    }
}
